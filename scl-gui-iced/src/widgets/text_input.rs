//! 文本输入框组件。

use iced::widget::text_input as iced_text_input;
use iced::{Element, Length};

use crate::theme::Theme;

/// 创建一个单行文本输入框。
pub fn text_input<'a, Message: Clone + 'a>(
    theme: &Theme,
    placeholder: impl Into<String>,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let theme = *theme;
    let placeholder = placeholder.into();

    iced_text_input(&placeholder, value)
        .width(Length::Fill)
        .padding([6, 10])
        .size(theme.typography.body.size)
        .font(theme.typography.body.font)
        .on_input(on_input)
        .style(move |_, status| {
            let focused = matches!(status, iced::widget::text_input::Status::Focused { .. });
            iced::widget::text_input::Style {
                background: iced::Background::Color(theme.chrome.medium_low),
                border: iced::border::Border {
                    radius: 4.0.into(),
                    width: if focused { 2.0 } else { 1.0 },
                    color: if focused {
                        theme.main.primary
                    } else {
                        theme.border.transient
                    },
                },
                icon: theme.base.medium,
                placeholder: theme.base.medium,
                value: theme.font_color,
                selection: theme.list.accent_medium,
            }
        })
        .into()
}

/// 创建一个密码输入框。
pub fn password_input<'a, Message: Clone + 'a>(
    theme: &Theme,
    placeholder: impl Into<String>,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let theme = *theme;
    let placeholder = placeholder.into();

    iced_text_input(&placeholder, value)
        .secure(true)
        .width(Length::Fill)
        .padding([6, 10])
        .size(theme.typography.body.size)
        .font(theme.typography.body.font)
        .on_input(on_input)
        .style(move |_, status| {
            let focused = matches!(status, iced::widget::text_input::Status::Focused { .. });
            iced::widget::text_input::Style {
                background: iced::Background::Color(theme.chrome.medium_low),
                border: iced::border::Border {
                    radius: 4.0.into(),
                    width: if focused { 2.0 } else { 1.0 },
                    color: if focused {
                        theme.main.primary
                    } else {
                        theme.border.transient
                    },
                },
                icon: theme.base.medium,
                placeholder: theme.base.medium,
                value: theme.font_color,
                selection: theme.list.accent_medium,
            }
        })
        .into()
}
