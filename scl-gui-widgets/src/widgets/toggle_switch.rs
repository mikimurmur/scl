//! 开关组件。

use iced::widget::toggler as iced_toggler;
use iced::{Element, Length};

use crate::theme::Theme;

/// 创建一个仿 WinUI 3 的开关。
pub fn toggle_switch<'a, Message: Clone + 'a>(
    theme: &Theme,
    value: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    let theme = *theme;

    iced::widget::container(iced_toggler(value).size(18.0).on_toggle(on_toggle).style(
        move |_, status| {
            let toggled = match status {
                iced::widget::toggler::Status::Active { is_toggled }
                | iced::widget::toggler::Status::Hovered { is_toggled } => is_toggled,
                iced::widget::toggler::Status::Disabled { is_toggled } => is_toggled,
            };

            iced::widget::toggler::Style {
                background: iced::Background::Color(if toggled {
                    theme.main.primary
                } else {
                    theme.base.low
                }),
                background_border_width: 1.0,
                background_border_color: if toggled {
                    theme.main.primary
                } else {
                    theme.base.medium
                },
                foreground: iced::Background::Color(if toggled {
                    theme.chrome.white_high
                } else {
                    theme.chrome.alt_low
                }),
                foreground_border_width: 1.0,
                foreground_border_color: theme.base.low,
                text_color: Some(theme.font_color),
                border_radius: None,
                padding_ratio: 0.12,
            }
        },
    ))
    .height(Length::Fixed(32.0))
    .align_y(iced::Center)
    .into()
}
