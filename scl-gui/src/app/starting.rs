//! 启动器启动时的初始化逻辑，以及启动游戏。

use druid::{EventCtx, ExtEventSink, Target};
use scl_core::{
    client::{Client, ClientConfig},
    java::JavaRuntime,
    version::structs::{SCLLaunchConfig as CoreSCLLaunchConfig, VersionInfo as CoreVersionInfo},
};

use crate::{
    app::{AppEvent, APP_EVENT},
    app_state::AppState,
    config::{Config, JavaRuntimeInfo},
    core::CoreTransformer,
    utils::{prelude, progress::Progress, thread::thread_main},
};

/// 扫描版本文件夹并把结果发送回界面
pub fn load_versions(sink: ExtEventSink, versions_path: std::path::PathBuf) {
    thread_main(move || {
        let versions = prelude::load_game_versions(&versions_path)?;
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::VersionsLoaded(versions),
            Target::Global,
        );
        Ok(())
    });
}

/// 扫描系统中的 Java 运行时
pub fn load_java(sink: ExtEventSink) {
    thread_main(move || {
        let mut candidates = scl_core::java::search_for_java();
        if candidates.is_empty() {
            candidates.push("java".into());
        }
        // PATH 中的重复目录会让同一个可执行文件被扫描多次。
        candidates.sort();
        candidates.dedup();
        let mut result = Vec::new();
        let mut failed = 0usize;
        for path in candidates {
            match JavaRuntime::from_java_path(&path) {
                Ok(runtime) => result.push(JavaRuntimeInfo {
                    path: runtime.path().to_owned(),
                    name: format!("Java {} ({})", runtime.version(), runtime.main_version()),
                    version: runtime.version().to_owned(),
                    is_64bit: runtime.is_64bit(),
                    main_version: runtime.main_version(),
                }),
                Err(err) => {
                    failed += 1;
                    tracing::error!("无法解析 Java 运行时 {path}: {err}");
                }
            }
        }
        result.sort_by_key(|a| a.main_version);
        // 同一个 Java 8+/16+ PATH 项 path 相同，不能只按 path 去重，
        // 否则扫描后默认的 8+/16+ 两条会合并成一条。
        result.dedup_by(|a, b| a.is_same_rt(b));
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::JavaLoaded {
                javas: result,
                failed,
            },
            Target::Global,
        );
        Ok(())
    });
}

/// 初始化启动器
pub fn init(sink: ExtEventSink, data: &AppState) {
    load_versions(sink.clone(), data.versions_folder());
    load_java(sink);
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn official_window_class_matches(class: &str) -> bool {
    matches!(class, "GLFW30" | "LWJGL" | "SunAwtFrame")
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn official_window_title_allows(title: &str) -> bool {
    !title.starts_with("GLFW") && title != "PopupMessageWindow"
}

/// Windows：扫描窗口类，判断游戏（或注入器弹窗）窗口是否已经出现。
#[cfg(target_os = "windows")]
mod win_window_scan {
    use std::collections::HashSet;

    use super::{official_window_class_matches, official_window_title_allows};

    type Hwnd = isize;

    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(
            lp_enum_fn: unsafe extern "system" fn(Hwnd, isize) -> i32,
            l_param: isize,
        ) -> i32;
        fn GetClassNameW(hwnd: Hwnd, lp_class_name: *mut u16, n_max_count: i32) -> i32;
        fn GetWindowTextW(hwnd: Hwnd, lp_string: *mut u16, n_max_count: i32) -> i32;
        fn IsWindowVisible(hwnd: Hwnd) -> i32;
        fn ShowWindow(hwnd: Hwnd, n_cmd_show: i32) -> i32;
    }

    const LAUNCHER_WINDOW_CLASS: &str = "Sharp Craft Launcher";
    const SW_HIDE: i32 = 0;

    unsafe fn window_class_name(hwnd: Hwnd) -> String {
        let mut buffer = [0u16; 512];
        let len = GetClassNameW(hwnd, buffer.as_mut_ptr(), 512);
        if len <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buffer[..len as usize])
    }

    unsafe fn window_text(hwnd: Hwnd) -> String {
        let mut buffer = [0u16; 512];
        let len = GetWindowTextW(hwnd, buffer.as_mut_ptr(), 512);
        if len <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buffer[..len as usize])
    }

    /// 官方回调：先判类名，未命中直接跳过（不查询标题）；
    unsafe extern "system" fn scan_callback(hwnd: Hwnd, l_param: isize) -> i32 {
        let found = &mut *(l_param as *mut HashSet<Hwnd>);
        let class_name = window_class_name(hwnd);
        if !official_window_class_matches(&class_name) {
            return 1;
        }
        let title = window_text(hwnd);
        if official_window_title_allows(&title) {
            found.insert(hwnd);
        }
        1
    }

    unsafe extern "system" fn hide_callback(hwnd: Hwnd, l_param: isize) -> i32 {
        let target = &mut *(l_param as *mut Hwnd);
        if *target != 0 {
            return 0;
        }
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        if window_class_name(hwnd) == LAUNCHER_WINDOW_CLASS
            || window_text(hwnd) == LAUNCHER_WINDOW_CLASS
        {
            *target = hwnd;
            return 0;
        }
        1
    }

    /// 单轮扫描：收集当前所有符合官方判定条件的顶层窗口句柄。
    pub fn scan_game_windows() -> HashSet<Hwnd> {
        let mut found = HashSet::new();
        unsafe {
            EnumWindows(scan_callback, &mut found as *mut _ as isize);
        }
        found
    }

    pub fn is_window_visible(hwnd: Hwnd) -> bool {
        unsafe { IsWindowVisible(hwnd) != 0 }
    }

    pub fn hide_launcher_window() {
        unsafe {
            let mut hwnd: Hwnd = 0;
            EnumWindows(hide_callback, &mut hwnd as *mut Hwnd as isize);
            if hwnd != 0 {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
    }
}

/// Windows：按窗口类/标题找到启动器自己的窗口并隐藏
#[cfg(target_os = "windows")]
pub fn hide_launcher_window() {
    win_window_scan::hide_launcher_window();
}

/// Linux：通过 X11 窗口树扫描游戏窗口。
#[cfg(target_os = "linux")]
mod linux_window_scan {
    use std::{
        ffi::{c_char, c_int, c_long, c_uchar, c_uint, c_ulong, c_void, CStr},
        ptr,
    };

    type Display = *mut c_void;
    type Window = c_ulong;
    const XA_CARDINAL: c_ulong = 6;
    const SUCCESS: c_int = 0;

    #[repr(C)]
    struct XClassHint {
        res_name: *mut c_char,
        res_class: *mut c_char,
    }

    #[link(name = "X11")]
    extern "C" {
        fn XOpenDisplay(name: *const c_char) -> Display;
        fn XDefaultRootWindow(display: Display) -> Window;
        fn XQueryTree(
            display: Display,
            window: Window,
            root_return: *mut Window,
            parent_return: *mut Window,
            children_return: *mut *mut Window,
            nchildren_return: *mut c_uint,
        ) -> c_int;
        fn XFetchName(display: Display, window: Window, name_return: *mut *mut c_char) -> c_int;
        fn XGetClassHint(display: Display, window: Window, hint_return: *mut XClassHint) -> c_int;
        fn XInternAtom(display: Display, name: *const c_char, only_if_exists: c_int) -> c_ulong;
        fn XGetWindowProperty(
            display: Display,
            window: Window,
            property: c_ulong,
            long_offset: c_long,
            long_length: c_long,
            delete: c_int,
            req_type: c_ulong,
            actual_type_return: *mut c_ulong,
            actual_format_return: *mut c_int,
            nitems_return: *mut c_ulong,
            bytes_after_return: *mut c_ulong,
            prop_return: *mut *mut c_uchar,
        ) -> c_int;
        fn XFree(data: *mut c_void) -> c_int;
        fn XCloseDisplay(display: Display) -> c_int;
    }

    fn window_pid(display: Display, window: Window) -> Option<u32> {
        unsafe {
            let atom = XInternAtom(display, b"_NET_WM_PID\0".as_ptr() as *const c_char, 0);
            if atom == 0 {
                return None;
            }
            let mut actual_type: c_ulong = 0;
            let mut actual_format: c_int = 0;
            let mut nitems: c_ulong = 0;
            let mut bytes_after: c_ulong = 0;
            let mut prop: *mut c_uchar = ptr::null_mut();
            if XGetWindowProperty(
                display,
                window,
                atom,
                0,
                1,
                0,
                XA_CARDINAL,
                &mut actual_type,
                &mut actual_format,
                &mut nitems,
                &mut bytes_after,
                &mut prop,
            ) == SUCCESS
                && !prop.is_null()
                && actual_type == XA_CARDINAL
                && actual_format == 32
                && nitems > 0
            {
                let pid = *(prop as *const c_ulong) as u32;
                XFree(prop as *mut c_void);
                return Some(pid);
            }
            if !prop.is_null() {
                XFree(prop as *mut c_void);
            }
            None
        }
    }

    fn window_strings(display: Display, window: Window) -> (String, String) {
        unsafe {
            let mut name: *mut c_char = ptr::null_mut();
            let mut title = String::new();
            if XFetchName(display, window, &mut name) != 0 && !name.is_null() {
                title = CStr::from_ptr(name).to_string_lossy().into_owned();
                XFree(name as *mut c_void);
            }

            let mut hint = XClassHint {
                res_name: ptr::null_mut(),
                res_class: ptr::null_mut(),
            };
            let mut class = String::new();
            if XGetClassHint(display, window, &mut hint) != 0 {
                if !hint.res_name.is_null() {
                    class.push_str(&CStr::from_ptr(hint.res_name).to_string_lossy());
                    XFree(hint.res_name as *mut c_void);
                }
                if !hint.res_class.is_null() {
                    class.push(' ');
                    class.push_str(&CStr::from_ptr(hint.res_class).to_string_lossy());
                    XFree(hint.res_class as *mut c_void);
                }
            }
            (title, class)
        }
    }

    /// 与官方 Windows 判定对齐：窗口属于启动的 Java 子进程（_NET_WM_PID），
    /// 或标题/类包含 Minecraft，或 GLFW/LWJGL/AWT/FML 等常见窗口类；
    /// 排除启动器自身窗口。
    fn is_game_window(title: &str, class: &str, pid: u32, window_pid: Option<u32>) -> bool {
        let title = title.to_lowercase();
        let class = class.to_lowercase();
        if title.contains("sharp craft launcher") || class.contains("sharpcraftlauncher") {
            return false;
        }
        if window_pid == Some(pid) {
            return true;
        }
        title.contains("minecraft")
            || class.contains("minecraft")
            || class.contains("lwjgl")
            || class.contains("glfw")
            || class.contains("sunawtframe")
            || class.contains("fml")
    }

    unsafe fn scan_children(display: Display, parent: Window, depth: u8, pid: u32) -> bool {
        let mut root: Window = 0;
        let mut parent_return: Window = 0;
        let mut children: *mut Window = ptr::null_mut();
        let mut count: c_uint = 0;
        if XQueryTree(
            display,
            parent,
            &mut root,
            &mut parent_return,
            &mut children,
            &mut count,
        ) == 0
        {
            return false;
        }

        let mut found = false;
        for i in 0..count as usize {
            let window = *children.add(i);
            let (title, class) = window_strings(display, window);
            let window_pid = window_pid(display, window);
            if is_game_window(&title, &class, pid, window_pid) {
                found = true;
                break;
            }
            if depth > 0 && scan_children(display, window, depth - 1, pid) {
                found = true;
                break;
            }
        }
        if !children.is_null() {
            XFree(children as *mut c_void);
        }
        found
    }

    pub fn game_window_exists(pid: u32) -> bool {
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return false;
            }
            let root = XDefaultRootWindow(display);
            let found = scan_children(display, root, 3, pid);
            XCloseDisplay(display);
            found
        }
    }
}

#[cfg(target_os = "windows")]
pub fn wait_for_game_window(_pid: u32) -> bool {
    let baseline = win_window_scan::scan_game_windows();
    loop {
        std::thread::sleep(std::time::Duration::from_millis(64));
        let current = win_window_scan::scan_game_windows();
        let fresh: Vec<_> = current.difference(&baseline).copied().collect();
        if fresh.is_empty() {
            continue;
        }
        for _ in 0..150 {
            std::thread::sleep(std::time::Duration::from_millis(200));
            if fresh
                .iter()
                .any(|&hwnd| win_window_scan::is_window_visible(hwnd))
            {
                break;
            }
        }
        return true;
    }
}

/// 等待游戏窗口出现，最多等待 `timeout`
///
/// 对应官方「不等待游戏启动」的 1 秒观察窗口：超时后无论是否发现窗口都继续流程。
pub fn wait_for_game_window_for(pid: u32, timeout: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if game_window_exists(pid) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(64));
    }
    false
}

#[cfg(target_os = "linux")]
fn game_window_exists(pid: u32) -> bool {
    linux_window_scan::game_window_exists(pid)
}

#[cfg(target_os = "windows")]
fn game_window_exists(_pid: u32) -> bool {
    // Windows 分支已有独立的基线差分等待逻辑，这里仅用于带超时的观察窗口。
    !win_window_scan::scan_game_windows().is_empty()
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn game_window_exists(_pid: u32) -> bool {
    true
}

#[cfg(target_os = "linux")]
pub fn wait_for_game_window(pid: u32) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    while std::time::Instant::now() < deadline {
        if linux_window_scan::game_window_exists(pid) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    false
}

/// 其它平台：无法枚举窗口，按「进程已启动」处理。
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn wait_for_game_window(_pid: u32) -> bool {
    true
}

/// 重新扫描版本文件夹
pub fn reload_versions(ctx: &mut EventCtx, data: &mut AppState) {
    load_versions(ctx.get_external_handle(), data.versions_folder());
}

/// 启动当前选中的游戏版本
pub fn start_game(ctx: &mut EventCtx, data: &mut AppState) {
    if data.launching {
        return;
    }
    let version_name = data.config.selected_version.to_owned();
    if version_name.is_empty() {
        data.launch_error = Some("请先选择一个游戏版本".into());
        return;
    }
    let version_base = data.versions_folder();
    let account = data
        .current_account()
        .unwrap_or_else(|| crate::core::AuthMethod::Offline {
            player_name: "Steve".into(),
            uuid: String::new(),
        });
    let cfg = data.config.to_owned();
    let recheck = data.auto_recheck_retry;
    let sink = ctx.get_external_handle();
    data.launching = true;
    data.launch_window_ready = false;
    data.starting_tip = Some(crate::ui::starting::random_tip());
    data.launch_error = None;
    thread_main(move || {
        let result = launch_game(
            sink.clone(),
            version_name,
            version_base,
            account,
            cfg,
            recheck,
        );
        if let Err(err) = result {
            tracing::error!("启动环节发生错误: {err}");
            let _ = sink.submit_command(
                APP_EVENT,
                AppEvent::LaunchFailed(format!("{err:?}")),
                Target::Global,
            );
        }
        Ok(())
    });
}

fn launch_game(
    sink: ExtEventSink,
    version_name: String,
    version_base: std::path::PathBuf,
    account: crate::core::AuthMethod,
    cfg: Config,
    recheck: bool,
) -> anyhow::Result<()> {
    tracing::info!("正在解析游戏版本信息");
    tracing::info!("游戏版本目录：{}", version_base.display());
    tracing::info!("正在加载版本元数据：{}", version_name);
    let mut version_info = CoreVersionInfo {
        version_base: version_base.to_string_lossy().to_string(),
        version: version_name,
        ..Default::default()
    };
    version_info.load()?;
    // 与官方启动流程一致：版本没有独立配置时，用全局“版本隔离方式”
    // 计算本次启动是否隔离；有独立配置时以版本设置为准（覆盖全局）。
    if version_info.scl_launch_config.is_none() {
        let mut scl = CoreSCLLaunchConfig {
            jvm_args: cfg.jvm_args.clone(),
            game_args: cfg.game_args.clone(),
            wrapper_path: cfg.wrapper_path.clone(),
            wrapper_args: cfg.wrapper_args.clone(),
            ..Default::default()
        };
        scl.game_independent = cfg.version_independent_method.should_apply(
            &version_info.minecraft_version,
            crate::core::VersionType::from(version_info.version_type),
        );
        version_info.scl_launch_config = Some(scl);
    } else if let Some(scl) = version_info.scl_launch_config.as_ref() {
        tracing::info!("版本独立已启用！");
        tracing::info!("SCL 版本覆盖设置：{:?}", scl);
    }
    // 官方在版本元数据加载完成后、选择 Java 运行环境前切割全局 JVM/游戏参数；
    // 切割失败（如引号未闭合）会中止本次启动并带回原始串。
    let java_args = split_launch_args(&cfg.jvm_args, "解析切割 JVM 参数参数失败")?;
    let game_args = split_launch_args(&cfg.game_args, "解析切割游戏参数失败")?;
    let required_java = version_info.required_java;
    let java_info = cfg
        .java_for_version(required_java)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("没有可用的 Java 运行时，请先在设置中添加 Java"))?;
    // 官方在启动环节按「独立配置 → Java < 16 → Java 16+」的顺序选择运行时并分别记录日志。
    let independent_java = version_info
        .scl_launch_config
        .as_ref()
        .map(|x| x.java_path.as_str())
        .filter(|path| !path.is_empty());
    let (java_path, java_branch) = if let Some(path) = independent_java {
        tracing::info!("选择独立配置的 Java 运行环境: {}", path);
        (path.to_owned(), "独立配置")
    } else if required_java >= 16 {
        tracing::info!("选择 Java 16+ 运行环境: {}", java_info.path);
        (java_info.path.clone(), "16+")
    } else {
        tracing::info!("选择 Java < 16 运行环境: {}", java_info.path);
        (java_info.path.clone(), "< 16")
    };
    let runtime = match JavaRuntime::from_java_path(&java_path) {
        Ok(runtime) => runtime,
        Err(err) => {
            match java_branch {
                "独立配置" => tracing::error!("解析独立配置的 Java 运行环境失败: {err}"),
                "16+" => tracing::error!("解析 Java 16+ 运行环境失败: {err}"),
                _ => tracing::error!("解析 Java < 16 运行环境失败: {err}"),
            }
            return Err(err);
        }
    };
    let max_mem = if let Some(max_mem) = version_info
        .scl_launch_config
        .as_ref()
        .and_then(|x| x.max_mem)
    {
        max_mem as u32
    } else if cfg.auto_mem_size || cfg.mem_size == 0 {
        let mem_status = scl_core::utils::get_mem_status();
        let mod_count = version_info
            .get_mods()
            .map(|mods| mods.len())
            .unwrap_or_default();
        let automated = crate::config::automated_max_memory(mem_status.free, mod_count);
        tracing::info!(
            "自动内存：MemTotal={}MB MemFree={}MB 模组数量={} → -Xmx{}m",
            mem_status.max,
            mem_status.free,
            mod_count,
            automated
        );
        automated as u32
    } else {
        cfg.mem_size as u32
    };
    tracing::info!("已设置最大内存：{}m", max_mem);
    let game_independent = version_info
        .scl_launch_config
        .as_ref()
        .map(|x| x.game_independent)
        .unwrap_or(false);
    tracing::info!("正在构造启动参数");
    let client = Client::new(ClientConfig {
        auth: account.into_core(),
        version_info,
        version_type: format!("SCL {}", env!("CARGO_PKG_VERSION")),
        custom_java_args: java_args,
        custom_args: game_args,
        java_runtime: runtime,
        max_mem,
        recheck,
    })?;
    let mut client = client;
    tracing::info!(
        "启动工作目录：{}（game_independent={}）",
        client.game_dir,
        game_independent
    );
    let log_path = std::path::Path::new(&client.game_dir)
        .join("logs")
        .join("latest.log");
    tracing::info!("游戏日志路径：{}", log_path.display());
    tracing::info!(
        "以下是启动参数，将会包含你的帐户访问信息令牌！除非你清楚你在做什么，否则绝对不可以分享参数给他人！"
    );
    tracing::info!(
        "启动参数：\n\t{}\n\t以上是启动参数，将会包含你的帐户访问信息令牌！",
        client.args.join("\n\t")
    );
    // 启动前关闭 Minecraft 的 narrator（options.txt: narrator:0）。
    if let Err(err) = disable_narrator(&client.game_dir) {
        tracing::warn!("关闭 Minecraft narrator 失败：{err:?}");
    }
    client.set_stdout(std::process::Stdio::piped());
    client.set_stderr(std::process::Stdio::piped());
    let pid = match client.launch() {
        Ok(pid) => pid,
        Err(err) => {
            tracing::error!("启动 Java 进程时发生错误");
            return Err(err);
        }
    };
    tracing::info!("进程已启动，进程 PID: {}", pid);
    let _ = sink.submit_command(APP_EVENT, AppEvent::GameStarted(pid), Target::Global);

    // 捕获游戏输出日志，供启动失败页面复制崩溃日志使用
    let mut child = client.process.take();

    // stdout 与 stderr 需要同时读取，否则管道写满后游戏会被阻塞。
    // 两个读取线程与进程等待在同一个 scope 内并行执行。
    std::thread::scope(|scope| {
        if let Some(process) = child.as_mut() {
            if let Some(reader) = process.stdout.take() {
                let sink = sink.clone();
                scope.spawn(move || handle_stdout_logging(std::io::BufReader::new(reader), sink));
            }
            if let Some(reader) = process.stderr.take() {
                let sink = sink.clone();
                scope.spawn(move || handle_stderr_logging(std::io::BufReader::new(reader), sink));
            }
        }
        // 等待游戏进程退出并记录返回值
        if let Some(c) = child.as_mut() {
            if let Ok(status) = c.wait() {
                add_log(
                    &sink,
                    format!("进程已退出，返回值：{}", status.code().unwrap_or(-1)),
                );
                // 非零返回值视为启动后崩溃：交给界面展示「游戏启动了，但是出错了……」页面，
                // 页面里的日志已经包含 stdout/stderr 捕获到的 Java 输出。
                if !status.success() {
                    tracing::warn!("游戏尚未显示窗口时已经报错退出");
                    let _ = sink.submit_command(
                        APP_EVENT,
                        AppEvent::GameCrashed(status.code()),
                        Target::Global,
                    );
                } else {
                    let _ =
                        sink.submit_command(APP_EVENT, AppEvent::GameWindowReady, Target::Global);
                }
            }
        }
    });
    Ok(())
}

/// 启动前把游戏目录里的 options.txt 的 narrator 设为 0（关闭旁白）
fn disable_narrator(game_dir: &str) -> anyhow::Result<()> {
    let path = std::path::Path::new(game_dir).join("options.txt");
    let data = match fs_err::read_to_string(&path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err.into()),
    };
    let mut lines: Vec<String> = data.lines().map(str::to_owned).collect();
    let mut found = false;
    let mut changed = false;
    for line in &mut lines {
        if let Some(value) = line.strip_prefix("narrator:") {
            found = true;
            if value.trim() != "0" {
                *line = "narrator:0".to_owned();
                changed = true;
            }
        }
    }
    if !found {
        lines.push("narrator:0".to_owned());
        changed = true;
    }
    if changed {
        fs_err::write(&path, format!("{}\n", lines.join("\n")))?;
        tracing::info!("已关闭 Minecraft narrator");
    }
    Ok(())
}

/// 把一行游戏日志发送到界面
fn add_log(sink: &ExtEventSink, line: String) {
    let _ = sink.submit_command(APP_EVENT, AppEvent::LogLine(line), Target::Global);
}

/// 持续读取游戏标准输出日志
fn handle_stdout_logging(
    mut reader: std::io::BufReader<std::process::ChildStdout>,
    sink: ExtEventSink,
) {
    use std::io::BufRead;
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\n', '\r']);
                if !trimmed.is_empty() {
                    add_log(&sink, trimmed.to_owned());
                }
            }
        }
    }
}

/// 持续读取游戏标准错误日志
fn handle_stderr_logging(
    mut reader: std::io::BufReader<std::process::ChildStderr>,
    sink: ExtEventSink,
) {
    use std::io::BufRead;
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\n', '\r']);
                if !trimmed.is_empty() {
                    add_log(&sink, trimmed.to_owned());
                }
            }
        }
    }
}

/// 首次启动初始化：挑选一个合适的下载源并检查正版登录可用性
///
/// 原版通过 `Progress` 总线在窗口底部显示这些步骤，而不是修改启动页提示：
/// “欢迎使用 SCL！正在初始化一些设置……”→“正在挑选一个合适的下载源……”→
/// “正在检查正版登录可用性”→ “访问令牌检查失败！/访问令牌准备完毕！”。
pub fn init_settings(sink: ExtEventSink, config: Config) {
    thread_main(move || {
        use scl_core::{
            download::{vanilla::VanillaDownloadExt, Downloader},
            progress::{ReportState, Reporter},
        };

        use crate::core::{CoreTransformer, DownloadSource};

        let progress = Progress::new(sink.clone());
        let set_tip = |msg: &str| {
            progress.send(ReportState::SetMessage(msg.to_owned()));
        };

        tracing::info!("首次启动：正在初始化设置");
        set_tip("欢迎使用 SCL！正在初始化一些设置……");
        set_tip("正在挑选一个合适的下载源……");

        let candidates = [DownloadSource::BmclApi, DownloadSource::Default];
        let mut best: Option<(std::time::Duration, DownloadSource)> = None;
        for source in candidates {
            let start = std::time::Instant::now();
            let downloader = Downloader::<()>::default().with_source(source.to_core());
            if downloader.get_avaliable_vanilla_versions().is_ok() {
                let elapsed = start.elapsed();
                tracing::info!("下载源 {} 测速完成：{:?}", source.display_name(), elapsed);
                if best.as_ref().map_or(true, |(t, _)| elapsed < *t) {
                    best = Some((elapsed, source));
                }
            } else {
                tracing::warn!("下载源 {} 测速失败", source.display_name());
            }
        }
        let mut config = config;
        if let Some((_, source)) = best {
            tracing::info!("已选择下载源：{}", source.display_name());
            config.download_source = source;
        }

        set_tip("正在检查正版登录可用性");
        let mut token_ok = true;
        for account in config.accounts_data.accounts.iter_mut() {
            if matches!(account, crate::core::AuthMethod::Mojang { .. }) {
                let mut core = account.to_owned().into_core();
                match scl_core::auth::refresh_auth(&mut core, &config.client_token) {
                    Ok(true) => {
                        let updated = crate::core::AuthMethod::from_core(core);
                        *account = updated;
                    }
                    _ => {
                        token_ok = false;
                        tracing::error!("访问令牌检查失败！");
                    }
                }
            }
        }
        if token_ok {
            set_tip("访问令牌准备完毕！");
        } else {
            set_tip("访问令牌检查失败！");
        }

        if !config.theme.theme_background_image_path.is_empty() {
            set_tip("正在加载图片");
            tracing::info!("正在加载背景图片");
        }

        progress.remove();
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::InitFinished(Box::new(config)),
            Target::Global,
        );
        Ok(())
    });
}

/// 按官方行为切割全局 JVM/游戏参数。
fn split_launch_args(args: &str, context: &str) -> anyhow::Result<Vec<String>> {
    if args.is_empty() {
        return Ok(Vec::new());
    }
    scl_core::utils::split_command_line(args).ok_or_else(|| anyhow::anyhow!("{context}: {args:?}"))
}

#[cfg(test)]
mod tests {
    use super::{official_window_class_matches, official_window_title_allows, split_launch_args};

    #[test]
    fn official_window_class_exact_match() {
        // 三代 Minecraft 窗口类全部命中
        assert!(official_window_class_matches("GLFW30")); // LWJGL 3（现代版）
        assert!(official_window_class_matches("LWJGL")); // LWJGL 2（1.12 及更早）
        assert!(official_window_class_matches("SunAwtFrame")); // AWT/Swing 窗口
                                                               // 全等比较：前缀相同但多字符不命中
        assert!(!official_window_class_matches("GLFW3"));
        assert!(!official_window_class_matches("GLFW300"));
        assert!(!official_window_class_matches("LWJGLX"));
        // 大小写敏感
        assert!(!official_window_class_matches("glfw30"));
        assert!(!official_window_class_matches("sunawtframe"));
        // 其它常见窗口类不命中（弹窗类 #32770、控制台 ConsoleWindowClass 等）
        assert!(!official_window_class_matches("#32770"));
        assert!(!official_window_class_matches("ConsoleWindowClass"));
        assert!(!official_window_class_matches("Chrome_WidgetWin_1"));
        assert!(!official_window_class_matches(""));
    }

    #[test]
    fn official_window_title_exclusions() {
        // GLFW 前缀排除：GLFW 的错误提示弹窗不代表游戏启动成功
        assert!(!official_window_title_allows("GLFW"));
        assert!(!official_window_title_allows("GLFW Error"));
        assert!(!official_window_title_allows("GLFW Message"));
        // PopupMessageWindow 是全等排除：仅多一个字符就不排除
        assert!(!official_window_title_allows("PopupMessageWindow"));
        assert!(official_window_title_allows("PopupMessageWindows"));
        assert!(official_window_title_allows("popupmessagewindow"));
        assert!(official_window_title_allows("A PopupMessageWindow"));
        // 正常游戏窗口标题放行（含各版本标题格式与 Forge 早期窗口）
        assert!(official_window_title_allows("Minecraft* 1.20.4"));
        assert!(official_window_title_allows("Minecraft 1.7.10"));
        assert!(official_window_title_allows("FML"));
        assert!(official_window_title_allows("FML early loading window"));
        assert!(official_window_title_allows(""));
    }

    #[test]
    fn split_launch_args_matches_official() {
        // 空串：官方先判 len==0 直接跳过，得到空参数表
        assert!(split_launch_args("", "ctx").unwrap().is_empty());
        // 只有空白：shlex 视为无参数
        assert!(split_launch_args(" \n\t ", "ctx").unwrap().is_empty());
        // 普通空白切分
        assert_eq!(
            split_launch_args("-Xmx1G -Xss1M", "ctx").unwrap(),
            vec!["-Xmx1G".to_owned(), "-Xss1M".to_owned()]
        );
        // 双引号内的空格保留为一个参数
        assert_eq!(
            split_launch_args(r#"-Xmx1G "-Dfoo=bar baz""#, "ctx").unwrap(),
            vec!["-Xmx1G".to_owned(), "-Dfoo=bar baz".to_owned()]
        );
        // Windows 使用 winsplit（官方命令行解析规则），单引号与反斜杠不是转义符；
        // 以下两条仅适用于 Unix 的 shlex 语义。
        #[cfg(unix)]
        {
            // 单引号同样分组
            assert_eq!(
                split_launch_args("-Dq='a b'", "ctx").unwrap(),
                vec!["-Dq=a b".to_owned()]
            );
            // 反斜杠转义空格
            assert_eq!(
                split_launch_args(r"-Dfoo=a\ b", "ctx").unwrap(),
                vec!["-Dfoo=a b".to_owned()]
            );
        }
        // 多行输入按空白切分
        assert_eq!(
            split_launch_args("-a\n-b\t-c", "ctx").unwrap(),
            vec!["-a".to_owned(), "-b".to_owned(), "-c".to_owned()]
        );
        // 未闭合引号：官方走 Err 分支，错误信息 = 上下文 + Debug 原始串
        let err =
            split_launch_args(r#"-Xmx1G "unclosed"#, "解析切割 JVM 参数参数失败").unwrap_err();
        assert_eq!(
            err.to_string(),
            r#"解析切割 JVM 参数参数失败: "-Xmx1G \"unclosed""#
        );
        // 未闭合单引号同理（仅 Unix shlex 会把单引号当引号）
        #[cfg(unix)]
        assert!(split_launch_args("-Dq='unclosed", "ctx").is_err());
    }
}
