//! 游戏启动失败页面。

use druid::{
    widget::{Flex, Scroll},
    Env, Widget, WidgetExt,
};
use scl_gui_widgets::widgets::label;

use crate::{app_state::AppState, ui::common, utils::open_url::open_url};

fn collect_logs(data: &AppState) -> String {
    if data.launch_error_logs.is_empty() {
        data.launch_error.clone().unwrap_or_default()
    } else {
        data.launch_error_logs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

fn latest_log_path(data: &AppState) -> std::path::PathBuf {
    // 与原版一致：优先打开当前版本的 logs/latest.log，其次回退到游戏目录 logs。
    let game_dir = data.config.minecraft_folder.to_path_buf();
    let version_log = game_dir
        .join("versions")
        .join(data.config.selected_version.as_str())
        .join("logs")
        .join("latest.log");
    if version_log.exists() {
        return version_log;
    }
    let logs = game_dir.join("logs");
    let latest = logs.join("latest.log");
    if latest.exists() {
        latest
    } else {
        logs
    }
}

/// 创建游戏启动失败页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    // 原版失败页的三个文本都是 label::new 的默认样式（BASE 13px、自动换行），
    // 没有额外设置字体/字号/颜色；padding 统一为 5。
    let title = label::new("游戏启动了，但是出错了……").padding(5.);

    let desc = label::new(
        "虽然 SCL 已经尽力保证游戏去顺利启动了，但问\n\
         题还是发生了。\n\
         你可以检查以下日志，以确认问题根源。\n\
         你也可以打开日志文件以向他人询问如何解决。",
    )
    .padding(5.);

    let logs = label::dynamic(|data: &AppState, _: &Env| collect_logs(data)).padding(5.);

    let open_log = common::button("打开日志文件").on_click(|_ctx, data, _env| {
        let path = latest_log_path(data);
        let _ = open_url(&path.to_string_lossy());
    });
    let copy_logs = common::button("复制到剪切板").on_click(|_ctx, data, _env| {
        crate::panic::copy_to_clipboard(collect_logs(data));
    });

    let buttons = Flex::row()
        .with_flex_child(open_log.expand_width(), 1.0)
        .with_spacer(5.)
        .with_flex_child(copy_logs.expand_width(), 1.0);

    let content = Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(title)
        .with_child(desc)
        .with_flex_child(Scroll::new(logs).vertical().expand(), 1.0)
        .with_child(buttons);

    Box::new(content.padding(5.))
}
