//! 游戏启动失败页面。

use iced::widget::{column, container, row, space};
use iced::{Alignment, Element, Length};
use scl_gui_widgets::widgets::ButtonKind;

use crate::{
    app::{Message, OPEN_URL},
    app_state::AppState,
    ui::{UiMessage, common},
};

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
    if latest.exists() { latest } else { logs }
}

/// 创建游戏启动失败页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let logs = collect_logs(state);

    let title = container(common::label(&theme, "游戏启动了，但是出错了……")).padding(5);

    let desc = container(common::label(
        &theme,
        "虽然 SCL 已经尽力保证游戏去顺利启动了，但问\n\
         题还是发生了。\n\
         你可以检查以下日志，以确认问题根源。\n\
         你也可以打开日志文件以向他人询问如何解决。",
    ))
    .padding(5);

    let logs_text = iced::widget::text(logs.clone())
        .size(theme.typography.base.size)
        .font(theme.typography.base.font)
        .color(theme.font_color);

    let open_log = container(common::button(
        &theme,
        "打开日志文件",
        ButtonKind::Standard,
        common::command(
            OPEN_URL,
            latest_log_path(state).to_string_lossy().to_string(),
        ),
    ))
    .width(Length::FillPortion(1));
    let copy_logs = container(common::button(
        &theme,
        "复制到剪切板",
        ButtonKind::Standard,
        Message::Ui(UiMessage::CopyText(logs)),
    ))
    .width(Length::FillPortion(1));

    let buttons =
        row![open_log, space().width(Length::Fixed(5.0)), copy_logs,].align_y(Alignment::Center);

    column![
        title,
        desc,
        container(common::scroll(&theme, container(logs_text).padding(5))).height(Length::Fill),
        buttons,
    ]
    .spacing(4)
    .padding(5)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
