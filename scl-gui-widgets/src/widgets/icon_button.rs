//! 图标按钮组件。

use iced::widget::button as iced_button;
use iced::{Element, Length};

use super::common::ButtonKind;
use crate::icons::Icon;
use crate::theme::Theme;

/// 创建一个图标按钮。
pub fn icon_button<'a, Message: Clone + 'a + 'static>(
    theme: &Theme,
    icon: Icon,
    size: f32,
    kind: ButtonKind,
    on_press: Message,
) -> Element<'a, Message> {
    let theme = *theme;
    let color = match kind {
        ButtonKind::Accent => theme.chrome.white_high,
        _ => theme.font_color,
    };

    iced_button(icon.view(&theme, size, color))
        .width(Length::Fixed(size + 14.0))
        .height(Length::Fixed(size + 14.0))
        .padding(7)
        .style(kind.style(theme))
        .on_press(on_press)
        .into()
}

/// 创建一个没有背景的图标按钮，常用于标题栏和工具栏。
pub fn flat_icon_button<'a, Message: Clone + 'a + 'static>(
    theme: &Theme,
    icon: Icon,
    size: f32,
    color: iced::Color,
    on_press: Message,
) -> Element<'a, Message> {
    let theme = *theme;

    iced_button(icon.view(&theme, size, color))
        .width(Length::Fixed(size + 12.0))
        .height(Length::Fixed(size + 12.0))
        .padding(6)
        .style(ButtonKind::Flat.style(theme))
        .on_press(on_press)
        .into()
}
