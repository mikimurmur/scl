//! Java 运行时的选择和管理。

use druid::{EventCtx, Target};
use scl_core::{
    java::{search_for_java, JavaRuntime},
    progress::{ReportState, Reporter},
};

use crate::{
    app::{save_config, AppEvent, APP_EVENT},
    app_state::AppState,
    config::JavaRuntimeInfo,
    utils::{progress::Progress, thread::thread_main},
};

/// 官方 `JavaRuntimeInfo::is_same_rt` 的比较规则：
pub fn same_runtime(a: &JavaRuntimeInfo, b: &JavaRuntimeInfo) -> bool {
    // 配置线已把 JavaRuntimeInfo::is_same_rt 对齐到 path+version+main_version，
    // 这里直接复用，避免页面层和配置层出现两套比较规则。
    a.is_same_rt(b)
}

/// PATH 默认项的路径（官方 JavaItem 用这两个字符串决定 Label 的
/// 50/100 宽度收缩，而不是行高）
pub fn is_default_path(path: &str) -> bool {
    path == "java" || path == "javaw"
}

/// 按平台还原 `JavaRuntimeInfo::display_name` 的后缀规则。
pub fn runtime_display_name(info: &JavaRuntimeInfo) -> String {
    #[allow(unused_mut)]
    let mut name = if info.name.is_empty() {
        info.path.clone()
    } else {
        info.name.clone()
    };
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    if !info.is_64bit && !name.contains(" (32 位)") {
        name.push_str(" (32 位)");
    }
    #[cfg(target_os = "windows")]
    {
        let file = std::path::Path::new(&info.path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        if file == "javaw" && !name.contains(" (无窗口)") {
            name.push_str(" (无窗口)");
        }
    }
    name
}

fn runtime_info(runtime: &JavaRuntime) -> JavaRuntimeInfo {
    JavaRuntimeInfo {
        path: runtime.path().to_owned(),
        name: format!("Java {} ({})", runtime.version(), runtime.main_version()),
        version: runtime.version().to_owned(),
        is_64bit: runtime.is_64bit(),
        main_version: runtime.main_version(),
    }
}

fn fallback_selected(data: &mut AppState) {
    let legacy = if data
        .config
        .javas
        .javapaths
        .iter()
        .any(|x| x.path == data.config.javas.selected_leagzy)
    {
        None
    } else {
        Some(
            data.config
                .javas
                .javapaths
                .iter()
                .find(|x| x.main_version < 16)
                .or_else(|| data.config.javas.javapaths.front())
                .map(|x| x.path.clone())
                .unwrap_or_else(|| "java".into()),
        )
    };
    if let Some(legacy) = legacy {
        data.config.javas.selected_leagzy = legacy;
    }

    let modern = if data
        .config
        .javas
        .javapaths
        .iter()
        .any(|x| x.path == data.config.javas.selected_modern)
    {
        None
    } else {
        Some(
            data.config
                .javas
                .javapaths
                .iter()
                .find(|x| x.main_version >= 16)
                .or_else(|| data.config.javas.javapaths.front())
                .map(|x| x.path.clone())
                .unwrap_or_else(|| "java".into()),
        )
    };
    if let Some(modern) = modern {
        data.config.javas.selected_modern = modern;
    }
}

/// 选择指定 Java 运行时（官方按整条 `JavaRuntimeInfo` 匹配）
pub fn select_java(data: &mut AppState, info: &JavaRuntimeInfo) {
    let Some(runtime) = data
        .config
        .javas
        .javapaths
        .iter()
        .find(|x| same_runtime(x, info))
        .cloned()
    else {
        return;
    };
    if runtime.main_version >= 16 {
        data.config.javas.selected_modern = info.path.clone();
    } else {
        data.config.javas.selected_leagzy = info.path.clone();
    }
    save_config(data);
}

/// 删除指定 Java 运行时
pub fn delete_java(data: &mut AppState, info: &JavaRuntimeInfo) {
    let mut remaining = data
        .config
        .javas
        .javapaths
        .iter()
        .filter(|x| !same_runtime(x, info))
        .cloned()
        .collect::<Vec<_>>();
    remaining.sort_by_key(|a| a.main_version);
    data.config.javas.javapaths = remaining.into();
    fallback_selected(data);
    save_config(data);
}

/// 合并一次 Java 扫描结果，保留手动添加的运行时并去重
pub fn merge_java_list(data: &mut AppState, javas: Vec<JavaRuntimeInfo>) {
    if javas.is_empty() {
        return;
    }
    let mut list = data
        .config
        .javas
        .javapaths
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    for info in javas {
        if let Some(existing) = list.iter_mut().find(|x| same_runtime(x, &info)) {
            *existing = info;
        } else {
            list.push(info);
        }
    }
    list.sort_by_key(|a| a.main_version);
    data.config.javas.javapaths = list.into();
    fallback_selected(data);
    save_config(data);
}

/// 添加一个 Java 运行时（用于手动添加和扫描事件）
pub fn add_java(data: &mut AppState, info: JavaRuntimeInfo) {
    let mut list = data
        .config
        .javas
        .javapaths
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    if let Some(existing) = list.iter_mut().find(|x| same_runtime(x, &info)) {
        *existing = info.clone();
    } else {
        list.push(info.clone());
    }
    list.sort_by_key(|a| a.main_version);
    data.config.javas.javapaths = list.into();
    if info.main_version >= 16 {
        data.config.javas.selected_modern = info.path;
    } else {
        data.config.javas.selected_leagzy = info.path;
    }
    save_config(data);
}

/// 手动添加一个 Java 运行时
pub fn manually_add_java(ctx: &EventCtx, _data: &mut AppState, path: &str) {
    if path.trim().is_empty() {
        return;
    }
    let path = path.trim().to_owned();
    let sink = ctx.get_external_handle();
    let progress = Progress::new(sink.clone());
    progress.send(ReportState::SetMessage("正在检索运行时信息".into()));
    thread_main(async move {
        let result = JavaRuntime::from_java_path(&path).await;
        progress.remove();
        let runtime = result?;
        let info = runtime_info(&runtime);
        let _ = sink.submit_command(APP_EVENT, AppEvent::JavaAdded(info), Target::Global);
        Ok(())
    });
}

/// 搜索可用的 Java 运行时；结果通过 [`AppEvent::JavaLoaded`] 合并回配置
pub fn search_java(ctx: &EventCtx, _data: &AppState) {
    let sink = ctx.get_external_handle();
    let progress = Progress::new(sink.clone());
    progress.send(ReportState::SetMessage(
        "正在尝试搜索已有 Java 运行时".into(),
    ));
    thread_main(async move {
        let mut candidates = search_for_java().await;
        if candidates.is_empty() {
            candidates.push("java".into());
        }
        progress.send(ReportState::SetMessage("正在检索运行时信息".into()));
        let mut result = Vec::new();
        for path in candidates {
            match JavaRuntime::from_java_path(&path).await {
                Ok(runtime) => result.push(runtime_info(&runtime)),
                Err(err) => tracing::error!("无法解析 Java 运行时: {err}"),
            }
        }
        result.sort_by_key(|a| a.main_version);
        let mut unique = Vec::with_capacity(result.len());
        for info in result {
            if !unique.iter().any(|x| same_runtime(x, &info)) {
                unique.push(info);
            }
        }
        progress.remove();
        let _ = sink.submit_command(APP_EVENT, AppEvent::JavaLoaded(unique), Target::Global);
        Ok(())
    });
}
