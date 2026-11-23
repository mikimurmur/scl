//! 滑块组件。

use iced::Element;
use iced::widget::slider as iced_slider;

use crate::theme::Theme;

/// 创建一个水平滑块。
pub fn slider<'a, Message: Clone + 'a>(
    theme: &Theme,
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    let theme = *theme;

    iced_slider(range, value, on_change)
        .step(1.0_f32)
        .style(move |_, _| iced::widget::slider::Style {
            rail: iced::widget::slider::Rail {
                backgrounds: (
                    iced::Background::Color(theme.base.low),
                    iced::Background::Color(theme.main.primary),
                ),
                width: 4.0,
                border: iced::border::Border {
                    radius: 2.0.into(),
                    ..iced::border::Border::default()
                },
            },
            handle: iced::widget::slider::Handle {
                shape: iced::widget::slider::HandleShape::Circle { radius: 7.0 },
                background: iced::Background::Color(theme.main.primary),
                border_width: 2.0,
                border_color: theme.chrome.white_high,
            },
        })
        .into()
}
