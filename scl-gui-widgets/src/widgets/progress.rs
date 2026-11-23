//! 进度条组件。

use iced::widget::progress_bar as iced_progress_bar;
use iced::{Element, Length};

use crate::theme::Theme;

/// 创建一个横向进度条，`value` 范围为 `0.0..=1.0`。
pub fn progress_bar<'a, Message: 'a>(
    theme: &Theme,
    value: f32,
    height: f32,
) -> Element<'a, Message> {
    let theme = *theme;

    iced_progress_bar(0.0..=1.0, value.clamp(0.0, 1.0))
        .length(Length::Fill)
        .girth(Length::Fixed(height))
        .style(move |_| iced::widget::progress_bar::Style {
            background: iced::Background::Color(theme.base.low),
            bar: iced::Background::Color(theme.main.primary),
            border: iced::border::Border {
                radius: (height / 2.0).into(),
                ..iced::border::Border::default()
            },
        })
        .into()
}

/// 创建一个不确定进度条（循环动画由外部状态驱动）。
pub fn indeterminate_progress<'a, Message: 'a>(
    theme: &Theme,
    phase: f32,
    height: f32,
) -> Element<'a, Message> {
    // Iced 自带进度条没有不确定模式；用一个往返移动的短条模拟。
    let theme = *theme;
    let width = 0.25;
    // 0→1 为从左到右，1→2 为从右到左，避免相位回绕时短条瞬间跳回左端。
    let cycle = phase.clamp(0.0, 1.0) * 2.0;
    let sweep = if cycle <= 1.0 { cycle } else { 2.0 - cycle };
    let position = (sweep * (1.0 - width)).clamp(0.0, 1.0 - width);

    iced::widget::stack![
        iced::widget::container(iced::widget::space())
            .width(Length::Fill)
            .height(Length::Fixed(height))
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme.base.low)),
                border: iced::border::Border {
                    radius: (height / 2.0).into(),
                    ..iced::border::Border::default()
                },
                ..iced::widget::container::Style::default()
            }),
        iced::widget::row![
            iced::widget::space().width(Length::FillPortion((position * 100.0) as u16)),
            iced::widget::container(iced::widget::space())
                .width(Length::FillPortion((width * 100.0) as u16))
                .height(Length::Fixed(height))
                .style(move |_| iced::widget::container::Style {
                    background: Some(iced::Background::Color(theme.main.primary)),
                    border: iced::border::Border {
                        radius: (height / 2.0).into(),
                        ..iced::border::Border::default()
                    },
                    ..iced::widget::container::Style::default()
                }),
            iced::widget::space().width(Length::FillPortion(
                ((1.0 - position - width) * 100.0) as u16
            )),
        ]
        .height(Length::Fixed(height)),
    ]
    .height(Length::Fixed(height))
    .into()
}
