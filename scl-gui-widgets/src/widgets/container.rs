//! 容器类组件。

use iced::widget::{container as iced_container, scrollable as iced_scrollable};
use iced::{Background, Element, Length};

use super::common::{card_style, page_style};
use crate::theme::Theme;

/// 创建一个卡片容器。
pub fn card<'a, Message: 'a>(
    theme: &Theme,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    iced_container(content.into())
        .width(Length::Fill)
        .padding(12)
        .style(card_style(*theme))
        .into()
}

/// 创建一个铺满窗口的页面容器。
pub fn page<'a, Message: 'a>(
    theme: &Theme,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    iced_container(content.into())
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .style(page_style(*theme))
        .into()
}

/// 创建一个使用主题颜色的滚动容器。
///
/// 返回原始的 `Scrollable`，因此仍然可以继续链式设置尺寸等属性。
pub fn scrollable<'a, Message: 'a>(
    theme: &Theme,
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Scrollable<'a, Message> {
    let theme = *theme;

    iced_scrollable(content).style(move |_, status| {
        let active = match status {
            iced::widget::scrollable::Status::Hovered {
                is_horizontal_scrollbar_hovered,
                is_vertical_scrollbar_hovered,
                ..
            } => is_horizontal_scrollbar_hovered || is_vertical_scrollbar_hovered,
            iced::widget::scrollable::Status::Dragged {
                is_horizontal_scrollbar_dragged,
                is_vertical_scrollbar_dragged,
                ..
            } => is_horizontal_scrollbar_dragged || is_vertical_scrollbar_dragged,
            _ => false,
        };

        let rail = iced::widget::scrollable::Rail {
            background: Some(Background::Color(theme.base.low)),
            border: iced::border::Border {
                radius: 2.0.into(),
                ..iced::border::Border::default()
            },
            scroller: iced::widget::scrollable::Scroller {
                background: Background::Color(if active {
                    theme.main.primary
                } else {
                    theme.chrome.high
                }),
                border: iced::border::Border {
                    radius: 2.0.into(),
                    ..iced::border::Border::default()
                },
            },
        };

        iced::widget::scrollable::Style {
            container: iced::widget::container::Style::default(),
            vertical_rail: rail,
            horizontal_rail: rail,
            gap: None,
            auto_scroll: iced::widget::scrollable::AutoScroll {
                background: Background::Color(theme.chrome.medium_low),
                border: iced::border::Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: theme.border.transient,
                },
                shadow: iced::Shadow::default(),
                icon: theme.font_color,
            },
        }
    })
}
