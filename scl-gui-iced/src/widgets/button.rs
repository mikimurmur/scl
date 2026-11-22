//! 常用按钮组件。

use iced::widget::{button as iced_button, column};
use iced::{Element, Length};

use super::common::{ButtonKind, styled_text};
use crate::theme::Theme;

/// 创建一个普通文本按钮。
pub fn button<'a, Message: Clone + 'a>(
    theme: &Theme,
    label: impl Into<String>,
    kind: ButtonKind,
    on_press: Message,
) -> Element<'a, Message> {
    button_maybe(theme, label, kind, Some(on_press))
}

/// 创建一个可以禁用的文本按钮。
pub fn button_maybe<'a, Message: Clone + 'a>(
    theme: &Theme,
    label: impl Into<String>,
    kind: ButtonKind,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let theme = *theme;
    let content = iced::widget::text(label.into())
        .size(theme.typography.body.size)
        .font(theme.typography.body.font);

    let mut widget = iced_button(content)
        .padding([6, 16])
        .style(kind.style(theme));

    if let Some(message) = on_press {
        widget = widget.on_press(message);
    }

    widget.into()
}

/// 创建一个宽大的启动按钮。
pub fn start_button<'a, Message: Clone + 'a>(
    theme: &Theme,
    title: impl Into<String>,
    subtitle: impl Into<String>,
    enabled: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let theme = *theme;
    let content = column![
        iced::widget::text(title.into())
            .size(18.0)
            .font(theme.typography.base_alt.font),
        iced::widget::text(subtitle.into())
            .size(11.0)
            .font(theme.typography.caption.font),
    ]
    .spacing(1)
    .align_x(iced::Center);

    let mut widget = iced_button(content)
        .width(Length::Fill)
        .height(Length::Fixed(62.0))
        .padding([8, 16])
        .style(ButtonKind::Accent.style(theme));

    if enabled {
        widget = widget.on_press(on_press);
    }

    widget.into()
}

/// 创建一个只有文字的扁平按钮。
pub fn flat_button<'a, Message: Clone + 'a>(
    theme: &Theme,
    label: impl Into<String>,
    on_press: Message,
) -> Element<'a, Message> {
    button(theme, label, ButtonKind::Flat, on_press)
}

/// 创建一个带标题和说明文字的设置项。
pub fn setting_row<'a, Message: Clone + 'a>(
    theme: &Theme,
    title: impl Into<String>,
    description: impl Into<String>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let theme = *theme;
    let text_column = column![
        styled_text(title.into(), theme.typography.body, theme.font_color),
        styled_text(
            description.into(),
            theme.typography.caption,
            theme.base.medium
        ),
    ]
    .spacing(2);

    iced::widget::row![text_column.width(Length::Fill), control.into()]
        .align_y(iced::Center)
        .spacing(12)
        .into()
}
