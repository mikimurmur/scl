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
    // 配置线只持久化主版本号，完整版本号在回读后会变成主版本数字符串，
    // 这里复用配置层的身份键（path + main_version），避免扫描结果与配置项
    // 因版本号形态不同而重复累积。
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
    use super::{merged_java_list, scan_notice, JavaRuntimeInfo};

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
}
