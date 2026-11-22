//! 仿 WinUI 3 Pivot 的导航分页控件。

use iced::widget::{button as iced_button, column, container, row};
use iced::{Element, Length};

use super::common::{ButtonKind, styled_text};
use crate::theme::Theme;

/// 导航栏中的一个页面项。
#[derive(Debug, Clone)]
pub struct NavigationItem<Message> {
    /// 页面名称。
    pub label: String,
    /// 点击时发送的消息。
    pub message: Message,
}

impl<Message> NavigationItem<Message> {
    /// 创建一个页面项。
    pub fn new(label: impl Into<String>, message: Message) -> Self {
        Self {
            label: label.into(),
            message,
        }
    }
}

/// 创建一个横向导航分页控件。
pub fn navigation_control<'a, Message: Clone + 'a>(
    theme: &Theme,
    items: impl IntoIterator<Item = NavigationItem<Message>>,
    active: usize,
) -> Element<'a, Message> {
    let theme = *theme;
    let mut bar = row![].spacing(4).height(Length::Fixed(40.0));

    for (index, item) in items.into_iter().enumerate() {
        let is_active = index == active;
        let color = if is_active {
            theme.base.high
        } else {
            theme.base.medium
        };

        let label = styled_text(item.label, theme.typography.body, color);

        let item_button = iced_button(label)
            .padding([8, 12])
            .style(ButtonKind::Flat.style(theme))
            .on_press(item.message);

        let underline = container(iced::widget::space())
            .width(Length::Fill)
            .height(Length::Fixed(2.0))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(if is_active {
                    theme.main.primary
                } else {
                    iced::Color::TRANSPARENT
                })),
                border: iced::border::Border {
                    radius: 1.0.into(),
                    ..iced::border::Border::default()
                },
                ..container::Style::default()
            });

        bar = bar.push(column![item_button, underline].spacing(0));
    }

    bar.into()
}
