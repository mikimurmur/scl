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

/// 两个 Java 路径是否指向同一个可执行文件。
///
/// sdkman 的 `current` 是符号链接，与具体版本目录的路径字符串不同，
/// 但实际是同一个文件；只按字符串比较会重复添加。
fn same_java_path(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// 官方 `JavaRuntimeInfo::is_same_rt` 的比较规则：
pub fn same_runtime(a: &JavaRuntimeInfo, b: &JavaRuntimeInfo) -> bool {
    // 配置线只持久化主版本号，完整版本号在回读后会变成主版本数字符串，
    // 这里复用配置层的身份键（path + main_version），避免扫描结果与配置项
    // 因版本号形态不同而重复累积。
    a.is_same_rt(b)
        // 符号链接（如 sdkman 的 current）与真实版本目录指向同一文件，
        // 路径字符串不同但仍是同一个运行时。
        || (a.main_version == b.main_version && same_java_path(&a.path, &b.path))
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

/// 合并扫描结果并清理重复项。
///
/// 旧版本配置里可能已经累积了同一条运行时的多份拷贝（PATH 中重复目录会导致
/// 扫描到同一个可执行文件多次，问题 ⑩），因此这里不仅对本次扫描结果去重，
/// 也把配置里已有的重复项一并折叠掉；后出现的条目覆盖先出现的同名条目。
fn merged_java_list(
    existing: Vec<JavaRuntimeInfo>,
    incoming: Vec<JavaRuntimeInfo>,
) -> Vec<JavaRuntimeInfo> {
    let mut unique: Vec<JavaRuntimeInfo> = Vec::with_capacity(existing.len() + incoming.len());
    for info in existing.into_iter().chain(incoming) {
        if let Some(current) = unique.iter_mut().find(|x| same_runtime(x, &info)) {
            *current = info;
        } else {
            unique.push(info);
        }
    }
    unique.sort_by_key(|a| a.main_version);
    unique
}

/// 生成一次扫描后的界面提示。
///
/// 解析失败的候选项不能只进日志（问题 ⑤）：返回 `Some` 时 Java 运行时页面会
/// 展示对应提示，全部成功时返回 `None` 不额外占位。
pub fn scan_notice(loaded: usize, failed: usize) -> Option<String> {
    match (loaded, failed) {
        (0, 0) => Some("没有找到可用的 Java 运行时".into()),
        (0, failed) => Some(format!(
            "未能识别任何 Java 运行时：{failed} 个候选无法解析，详情见日志"
        )),
        (_, 0) => None,
        (_, failed) => Some(format!(
            "已跳过 {failed} 个无法识别的 Java 候选，详情见日志"
        )),
    }
}

/// 合并一次 Java 扫描结果，保留手动添加的运行时并去重
pub fn merge_java_list(data: &mut AppState, javas: Vec<JavaRuntimeInfo>) {
    let existing = data
        .config
        .javas
        .javapaths
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let list = merged_java_list(existing, javas);
    data.config.javas.javapaths = list.into();
    fallback_selected(data);
    save_config(data);
}

/// 添加一个 Java 运行时（用于手动添加和扫描事件）
pub fn add_java(data: &mut AppState, info: JavaRuntimeInfo) {
    let existing = data
        .config
        .javas
        .javapaths
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let list = merged_java_list(existing, vec![info.clone()]);
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
    thread_main(move || {
        let result = JavaRuntime::from_java_path(&path);
        progress.remove();
        let runtime = result?;
        let info = runtime_info(&runtime);
        let _ = sink.submit_command(APP_EVENT, AppEvent::JavaAdded(info), Target::Global);
        Ok(())
    });
}

/// 按版本元数据要求选择 Java 运行时。
///
/// 优先沿用用户已选中的运行时（只要它满足要求）；否则从已发现的运行时里
/// 取满足要求的最低主版本；没有任何满足项时返回带当前发现列表的错误，
/// 不再硬塞一个不满足要求的运行时。
pub fn select_java_for_requirement<'a>(
    javas: &'a [JavaRuntimeInfo],
    required: u8,
    preferred_path: &str,
) -> anyhow::Result<&'a JavaRuntimeInfo> {
    let satisfies = |info: &JavaRuntimeInfo| info.main_version > 0 && info.main_version >= required;
    if let Some(preferred) = javas
        .iter()
        .find(|info| info.path == preferred_path && satisfies(info))
    {
        return Ok(preferred);
    }
    if let Some(lowest) = javas
        .iter()
        .filter(|info| satisfies(info))
        .min_by_key(|x| x.main_version)
    {
        return Ok(lowest);
    }
    let mut found: Vec<u8> = javas
        .iter()
        .map(|info| info.main_version)
        .filter(|version| *version > 0)
        .collect();
    found.sort_unstable();
    found.dedup();
    let found = if found.is_empty() {
        "无".to_owned()
    } else {
        found
            .iter()
            .map(|version| format!("Java {version}"))
            .collect::<Vec<_>>()
            .join("、")
    };
    anyhow::bail!("需要 Java {required} 或更高，已发现的运行时都不满足（已发现：{found}）")
}

/// 搜索可用的 Java 运行时；结果通过 [`AppEvent::JavaLoaded`] 合并回配置
pub fn search_java(ctx: &EventCtx, _data: &AppState) {
    let sink = ctx.get_external_handle();
    let progress = Progress::new(sink.clone());
    progress.send(ReportState::SetMessage(
        "正在尝试搜索已有 Java 运行时".into(),
    ));
    thread_main(move || {
        let mut candidates = search_for_java();
        if candidates.is_empty() {
            candidates.push("java".into());
        }
        // PATH 中重复的目录会让同一个可执行文件被扫描多次；先按路径去重，
        // 避免对同一文件重复执行 `java -version`。
        candidates.sort();
        candidates.dedup();
        // 符号链接（sdkman 的 current）与真实版本目录是同一个文件，
        // 按解析后的真实路径再去重一次，避免重复探测与重复结果。
        let mut canonical_paths = std::collections::HashSet::new();
        candidates.retain(|path| {
            let canonical =
                std::fs::canonicalize(path).unwrap_or_else(|_| std::path::PathBuf::from(path));
            canonical_paths.insert(canonical)
        });
        progress.send(ReportState::SetMessage("正在检索运行时信息".into()));
        let mut result = Vec::new();
        let mut failed = 0usize;
        for path in candidates {
            match JavaRuntime::from_java_path(&path) {
                Ok(runtime) => result.push(runtime_info(&runtime)),
                Err(err) => {
                    failed += 1;
                    tracing::error!("无法解析 Java 运行时 {path}: {err}");
                }
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
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::JavaLoaded {
                javas: unique,
                failed,
            },
            Target::Global,
        );
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::{
        merged_java_list, same_runtime, scan_notice, select_java_for_requirement, JavaRuntimeInfo,
    };

    fn runtime(path: &str, version: &str, main_version: u8, name: &str) -> JavaRuntimeInfo {
        JavaRuntimeInfo {
            path: path.to_owned(),
            name: name.to_owned(),
            version: version.to_owned(),
            is_64bit: true,
            main_version,
        }
    }

    #[test]
    fn collapses_persisted_duplicates_with_different_version_shape() {
        // 配置回读的 version 是主版本数字符串，重新扫描得到的是完整版本号；
        // 两者必须被视为同一运行时，否则每次扫描都会累积一条（问题 ⑩）。
        let existing = vec![runtime("/home/link/.sdkman/bin/java", "21", 21, "旧名称")];
        let incoming = vec![runtime(
            "/home/link/.sdkman/bin/java",
            "21.0.11",
            21,
            "Java 21.0.11 (21)",
        )];
        let merged = merged_java_list(existing, incoming);
        assert_eq!(merged.len(), 1, "同一路径与主版本号应折叠为一条");
        assert_eq!(merged[0].version, "21.0.11", "应保留最新扫描到的完整版本号");
        assert_eq!(merged[0].name, "Java 21.0.11 (21)");
    }

    #[test]
    fn collapses_multiple_existing_duplicates() {
        let existing = vec![
            runtime("/tmp/fakebin/java", "0", 0, "Java  (0)"),
            runtime("/tmp/fakebin/java", "", 0, "Java  (0)"),
            runtime("/tmp/fakebin/java", "0", 0, "Java  (0)"),
        ];
        let merged = merged_java_list(existing, Vec::new());
        assert_eq!(merged.len(), 1, "历史配置里的重复项应被一并清理");
    }

    #[test]
    fn keeps_same_path_with_different_main_version() {
        // 默认配置里 Java 8+ 与 Java 16+ 的 path 都是 "java"，不能误合并。
        let existing = vec![
            runtime("java", "1.8", 8, "PATH 环境变量里的 Java 8+"),
            runtime("java", "16", 16, "PATH 环境变量里的 Java 16+"),
        ];
        let merged = merged_java_list(existing, Vec::new());
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].main_version, 8);
        assert_eq!(merged[1].main_version, 16);
    }

    #[test]
    fn scan_notice_reports_failures_and_distinguishes_empty_result() {
        assert_eq!(scan_notice(3, 0), None);
        assert!(scan_notice(0, 2)
            .unwrap()
            .contains("未能识别任何 Java 运行时"));
        assert!(scan_notice(2, 1).unwrap().contains("已跳过 1 个"));
        assert_eq!(scan_notice(0, 0).unwrap(), "没有找到可用的 Java 运行时");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_java_path_is_same_runtime() {
        use std::os::unix::fs::symlink;

        let base =
            std::env::temp_dir().join(format!("scl-java-symlink-test-{}", std::process::id()));
        let version_dir = base.join("21.0.11-zulu");
        let bin = version_dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("java"), b"").unwrap();
        let current = base.join("current");
        symlink(&version_dir, &current).unwrap();

        let version = runtime(
            &bin.join("java").to_string_lossy(),
            "21.0.11",
            21,
            "版本目录",
        );
        let via_current = runtime(
            &current.join("bin/java").to_string_lossy(),
            "21.0.11",
            21,
            "current 符号链接",
        );
        assert!(
            same_runtime(&version, &via_current),
            "current 符号链接与版本目录应视为同一运行时"
        );
        // sdkman 的 current 与版本目录同时被搜到时，合并后只能保留一条。
        let merged = merged_java_list(vec![via_current], vec![version]);
        assert_eq!(merged.len(), 1);

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn same_main_version_different_paths_are_distinct() {
        let a = runtime("/opt/jdk-a/bin/java", "17.0.1", 17, "A");
        let b = runtime("/opt/jdk-b/bin/java", "17.0.2", 17, "B");
        assert!(
            !same_runtime(&a, &b),
            "不同路径的 JDK 即使主版本相同也不能合并"
        );
    }

    fn java_set() -> Vec<JavaRuntimeInfo> {
        vec![
            runtime("/java/8/bin/java", "1.8.0_504", 8, "Java 8"),
            runtime("/java/16/bin/java", "16.0.2", 16, "Java 16"),
            runtime("/java/21/bin/java", "21.0.11", 21, "Java 21"),
            runtime("/java/25/bin/java", "25.0.3", 25, "Java 25"),
        ]
    }

    #[test]
    fn required_java_picks_lowest_satisfying_runtime() {
        // 26.x 元数据要求 Java 25：机器上有 25 时必须选它，而不是 Java 8。
        let set = java_set();
        let selected = select_java_for_requirement(&set, 25, "").expect("应找到 Java 25");
        assert_eq!(selected.main_version, 25);

        // 要求 17 时取满足要求的最低主版本（21），不会跳去更高的 25。
        let selected = select_java_for_requirement(&set, 17, "").expect("应找到 Java 21");
        assert_eq!(selected.main_version, 21);
    }

    #[test]
    fn required_java_keeps_selected_runtime_when_it_satisfies() {
        let set = java_set();
        // 用户已选 Java 25，要求 16 时沿用用户选择，不强行降到 21。
        let selected =
            select_java_for_requirement(&set, 16, "/java/25/bin/java").expect("沿用已选");
        assert_eq!(selected.main_version, 25);

        // 已选的 Java 8 不满足 25，必须改选满足要求的最低版本。
        let selected = select_java_for_requirement(&set, 25, "/java/8/bin/java").expect("改选");
        assert_eq!(selected.main_version, 25);
    }

    #[test]
    fn required_java_rejects_when_nothing_satisfies() {
        let set = java_set();
        let err = select_java_for_requirement(&set, 30, "").expect_err("不应硬塞运行时");
        let message = format!("{err}");
        assert!(message.contains("需要 Java 30"), "{message}");
        assert!(message.contains("Java 8"), "{message}");
        assert!(message.contains("Java 25"), "{message}");
    }
}
