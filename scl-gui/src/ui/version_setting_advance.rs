//! 版本高级设置页面。

use iced::widget::{column, container, row, space};
use iced::{Alignment, Element, Length};
use scl_gui_widgets::theme::Theme;

use crate::{
    app::Message,
    app_state::AppState,
    ui::{BoolField, TextField, UiMessage, common},
};

/// 一行「标签 + 输入框」。
fn field<'a>(
    theme: &Theme,
    label: &'static str,
    input: Element<'a, Message>,
) -> Element<'a, Message> {
    row![
        container(common::label(theme, label)).width(Length::Fixed(110.0)),
        space().width(Length::Fixed(5.0)),
        container(input).width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .into()
}

/// 创建版本高级设置页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let config = &state.version_setting_config;

    let status = common::memory_status();
    let used = status.max.saturating_sub(status.free);
    let game_memory = if config.override_max_mem {
        config
            .max_mem
            .or_else(|| config.max_mem_text.trim().parse::<usize>().ok())
            .unwrap_or(0) as f64
    } else {
        (status.free as f64 * 0.75).min(4000.0)
    };

    let memory_info = row![
        iced::widget::text(format!(
            "当前占用：{}",
            common::format_memory_text(used as f64)
        ))
        .size(13.0)
        .font(theme.typography.caption_alt.font)
        .color(theme.base.medium),
        space().width(Length::Fill),
        iced::widget::text(format!(
            "游戏内存：{}",
            common::format_memory_text(game_memory)
        ))
        .size(13.0)
        .font(theme.typography.caption_alt.font)
        .color(theme.base.medium),
    ]
    .align_y(Alignment::Center);

    let memory_input = if config.override_max_mem {
        common::text_input(&theme, "内存大小", &config.max_mem_text, move |value| {
            Message::Ui(UiMessage::Text(TextField::VersionMaxMemory, value))
        })
    } else {
        common::text_input_readonly(&theme, "内存大小", &config.max_mem_text)
    };

    let manual_memory_row = row![
        common::radio(
            &theme,
            "手动调节内存",
            config.override_max_mem,
            Message::Ui(UiMessage::Bool(BoolField::VersionOverrideMemory, true))
        ),
        space().width(Length::Fixed(12.0)),
        container(memory_input).width(Length::Fill),
        space().width(Length::Fixed(4.0)),
        common::label(&theme, "MB"),
    ]
    .align_y(Alignment::Center);

    let fields = column![
        field(
            &theme,
            "JVM 参数",
            common::text_input(&theme, "", &config.jvm_args, move |value| Message::Ui(
                UiMessage::Text(TextField::VersionJvmArgs, value)
            ))
        ),
        space().height(Length::Fixed(5.0)),
        field(
            &theme,
            "游戏参数",
            common::text_input(&theme, "", &config.game_args, move |value| Message::Ui(
                UiMessage::Text(TextField::VersionGameArgs, value)
            ))
        ),
        space().height(Length::Fixed(5.0)),
        field(
            &theme,
            "Java 路径",
            common::text_input(&theme, "", &config.java_path, move |value| Message::Ui(
                UiMessage::Text(TextField::VersionJavaPath, value)
            ))
        ),
        space().height(Length::Fixed(5.0)),
        field(
            &theme,
            "窗口标题",
            common::text_input(&theme, "", &config.window_title, move |value| Message::Ui(
                UiMessage::Text(TextField::VersionWindowTitle, value)
            ))
        ),
        space().height(Length::Fixed(5.0)),
        field(
            &theme,
            "包装执行程序路径",
            common::text_input(&theme, "", &config.wrapper_path, move |value| Message::Ui(
                UiMessage::Text(TextField::VersionWrapperPath, value)
            ))
        ),
        space().height(Length::Fixed(5.0)),
        field(
            &theme,
            "包装执行程序参数",
            common::text_input(&theme, "", &config.wrapper_args, move |value| Message::Ui(
                UiMessage::Text(TextField::VersionWrapperArgs, value)
            ))
        ),
    ]
    .spacing(0);

    let content = column![
        container(common::title(&theme, "高级设置")).height(Length::Fixed(40.0)),
        container(common::label(
            &theme,
            "以下 JVM 参数和游戏参数均会直接附加在已有参数后面"
        ))
        .height(Length::Fixed(32.0)),
        space().height(Length::Fixed(8.0)),
        fields,
        space().height(Length::Fixed(8.0)),
        common::section(&theme, "游戏运行内存"),
        space().height(Length::Fixed(8.0)),
        memory_info,
        space().height(Length::Fixed(8.0)),
        common::memory_bar(
            &theme,
            if config.override_max_mem {
                config
                    .max_mem
                    .or_else(|| config.max_mem_text.trim().parse().ok())
            } else {
                None
            }
        ),
        space().height(Length::Fixed(18.0)),
        common::radio(
            &theme,
            "遵循全局设置",
            !config.override_max_mem,
            Message::Ui(UiMessage::Bool(BoolField::VersionOverrideMemory, false))
        ),
        manual_memory_row,
        space().height(Length::Fixed(8.0)),
    ]
    .spacing(4)
    .padding(10);

    common::scroll(&theme, content)
}
