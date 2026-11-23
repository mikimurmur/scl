//! 仿 Windows 小窗口标题栏和窗口外壳。

use iced::widget::{button as iced_button, container, mouse_area, row};
use iced::{Element, Length, Task};

use super::common::styled_text;
use crate::icons::{self, Icon};
use crate::theme::{Theme, contrast_color};

/// 标题栏发出的消息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMessage {
    /// 返回上一页。
    Back,
    /// 开始拖动窗口。
    Drag,
    /// 最小化窗口。
    Minimize,
    /// 关闭窗口。
    Close,
}

/// 标题栏高度。
pub const TITLE_BAR_HEIGHT: f32 = 32.0;

/// 创建一个仿 Windows 风格的标题栏。
pub fn title_bar<'a>(
    theme: &Theme,
    title: impl Into<String>,
    show_back: bool,
) -> Element<'a, WindowMessage> {
    let theme = *theme;
    let background = if theme.main.title_bar.a > 0.0 {
        theme.main.title_bar
    } else {
        theme.window_background
    };
    let foreground = if theme.main.title_bar.a > 0.0 {
        contrast_color(background)
    } else {
        theme.font_color
    };

    let title_text = styled_text(title.into(), theme.typography.caption, foreground);

    let title_area = mouse_area(
        row![title_text]
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::Center)
            .padding([0, 8]),
    )
    .on_press(WindowMessage::Drag)
    .interaction(iced::mouse::Interaction::Grab);

    let mut bar = row![].height(Length::Fixed(TITLE_BAR_HEIGHT)).spacing(0);

    if show_back {
        bar = bar.push(control_button(
            theme,
            icons::BACK,
            foreground,
            WindowMessage::Back,
        ));
    }

    bar = bar
        .push(title_area)
        .push(control_button(
            theme,
            icons::MINIMIZE,
            foreground,
            WindowMessage::Minimize,
        ))
        .push(control_button(
            theme,
            icons::CLOSE,
            foreground,
            WindowMessage::Close,
        ));

    container(bar)
        .width(Length::Fill)
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(background)),
            border: iced::border::Border {
                width: 0.0,
                ..iced::border::Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

fn control_button<'a>(
    theme: Theme,
    icon: Icon,
    color: iced::Color,
    message: WindowMessage,
) -> Element<'a, WindowMessage> {
    iced_button(icon.view(&theme, 12.0, color))
        .width(Length::Fixed(TITLE_BAR_HEIGHT))
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .padding(10)
        .style(move |_, status| {
            let background = match status {
                iced::widget::button::Status::Hovered => theme.base.low,
                iced::widget::button::Status::Pressed => theme.base.medium_low,
                _ => iced::Color::TRANSPARENT,
            };

            iced::widget::button::Style {
                background: Some(iced::Background::Color(background)),
                text_color: color,
                border: iced::border::Border::default(),
                shadow: iced::Shadow::default(),
                snap: true,
            }
        })
        .on_press(message)
        .into()
}

/// 把标题栏和页面内容组合成一个完整窗口。
pub fn window_shell<'a, Message: 'a>(
    theme: &Theme,
    title: impl Into<String>,
    show_back: bool,
    content: impl Into<Element<'a, Message>>,
    map: impl Fn(WindowMessage) -> Message + 'a,
) -> Element<'a, Message> {
    let bar = title_bar(theme, title, show_back).map(map);

    iced::widget::column![bar, content.into()]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// 处理标题栏发出的窗口操作消息。
///
/// `Back` 不在这里处理，应用需要自己维护页面栈。
pub fn update_window<Message: 'static>(
    message: WindowMessage,
    window: iced::window::Id,
) -> Task<Message> {
    match message {
        WindowMessage::Back => Task::none(),
        WindowMessage::Drag => iced::window::drag(window),
        WindowMessage::Minimize => iced::window::minimize(window, true),
        WindowMessage::Close => iced::window::close(window),
    }
}
