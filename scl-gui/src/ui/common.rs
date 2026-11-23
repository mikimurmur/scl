//! 各个页面共用的界面组件。

use iced::widget::{button as iced_button, column, container, row, scrollable, space, stack, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow};
use scl_core::utils::{MemoryStatus, get_mem_status};
use scl_gui_widgets::icons::{self, Icon};
use scl_gui_widgets::theme::{TextStyle, Theme};
use scl_gui_widgets::widgets::{self, ButtonKind};

use crate::app::{Message, Selector, Target};

/// 把一个带载荷的选择器包装成界面消息。
pub fn command<T: Send + Sync + 'static>(selector: Selector<T>, payload: T) -> Message {
    Message::Command(selector.with(payload).to(Target::Global))
}

/// 把页面控件消息包装成界面消息。
pub fn ui(message: crate::ui::UiMessage) -> Message {
    Message::Ui(message)
}

/// 普通正文标签。
pub fn label<'a>(theme: &Theme, content: impl Into<String>) -> Element<'a, Message> {
    text(content.into())
        .size(theme.typography.base.size)
        .font(theme.typography.base.font)
        .color(theme.font_color)
        .into()
}

/// 指定字体样式的标签。
pub fn styled<'a>(
    _theme: &Theme,
    content: impl Into<String>,
    style: TextStyle,
    color: Color,
) -> Element<'a, Message> {
    text(content.into())
        .size(style.size)
        .font(style.font)
        .color(color)
        .into()
}

/// 次级说明标签（13px）。
pub fn caption<'a>(theme: &Theme, content: impl Into<String>) -> Element<'a, Message> {
    styled(
        theme,
        content,
        theme.typography.caption_alt,
        theme.base.medium,
    )
}

/// 更小的说明标签（11px）。
pub fn hint<'a>(theme: &Theme, content: impl Into<String>) -> Element<'a, Message> {
    styled(
        theme,
        content,
        TextStyle::new(theme.typography.caption.font, 11.0),
        theme.base.medium,
    )
}

/// 页面标题。
pub fn title<'a>(theme: &Theme, content: impl Into<String>) -> Element<'a, Message> {
    styled(theme, content, theme.typography.title, theme.font_color)
}

/// 小标题。
pub fn section<'a>(theme: &Theme, content: impl Into<String>) -> Element<'a, Message> {
    styled(
        theme,
        content,
        TextStyle::new(theme.typography.body.font, 17.0),
        theme.font_color,
    )
}

/// 表单字段标题。
pub fn field_label<'a>(theme: &Theme, content: impl Into<String>) -> Element<'a, Message> {
    container(label(theme, content))
        .height(Length::Fixed(26.0))
        .into()
}

/// 普通按钮。
pub fn button<'a>(
    theme: &Theme,
    label_text: impl Into<String>,
    kind: ButtonKind,
    message: Message,
) -> Element<'a, Message> {
    widgets::button(theme, label_text, kind, message)
}

/// 可以禁用的按钮。
pub fn button_maybe<'a>(
    theme: &Theme,
    label_text: impl Into<String>,
    kind: ButtonKind,
    message: Option<Message>,
) -> Element<'a, Message> {
    widgets::button_maybe(theme, label_text, kind, message)
}

/// 强调色按钮。
pub fn accent_button<'a>(
    theme: &Theme,
    label_text: impl Into<String>,
    message: Message,
) -> Element<'a, Message> {
    button(theme, label_text, ButtonKind::Accent, message)
}

/// 文本输入框。
#[allow(clippy::too_many_arguments)]
pub fn text_input<'a>(
    theme: &Theme,
    placeholder: impl Into<String>,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    widgets::text_input(theme, placeholder, value, on_input)
}

/// 带回车提交的文本输入框。
pub fn text_input_submit<'a>(
    theme: &Theme,
    placeholder: impl Into<String>,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
    on_submit: Message,
) -> Element<'a, Message> {
    let theme = *theme;
    let placeholder = placeholder.into();

    iced::widget::text_input(&placeholder, value)
        .width(Length::Fill)
        .padding([6, 10])
        .size(theme.typography.body.size)
        .font(theme.typography.body.font)
        .on_input(on_input)
        .on_submit(on_submit)
        .style(move |_, status| text_input_style(theme, status))
        .into()
}

/// 只读文本输入框。
pub fn text_input_readonly<'a>(
    theme: &Theme,
    placeholder: impl Into<String>,
    value: &str,
) -> Element<'a, Message> {
    let theme = *theme;
    let placeholder = placeholder.into();

    iced::widget::text_input(&placeholder, value)
        .width(Length::Fill)
        .padding([6, 10])
        .size(theme.typography.body.size)
        .font(theme.typography.body.font)
        .style(move |_, status| text_input_style(theme, status))
        .into()
}

fn text_input_style(
    theme: Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let focused = matches!(status, iced::widget::text_input::Status::Focused { .. });
    iced::widget::text_input::Style {
        background: Background::Color(theme.chrome.medium_low),
        border: Border {
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
}

/// 密码输入框。
pub fn password_input<'a>(
    theme: &Theme,
    placeholder: impl Into<String>,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
    on_submit: Message,
) -> Element<'a, Message> {
    let theme = *theme;
    let placeholder = placeholder.into();

    iced::widget::text_input(&placeholder, value)
        .secure(true)
        .width(Length::Fill)
        .padding([6, 10])
        .size(theme.typography.body.size)
        .font(theme.typography.body.font)
        .on_input(on_input)
        .on_submit(on_submit)
        .style(move |_, status| text_input_style(theme, status))
        .into()
}

/// 可滚动区域。
pub fn scroll<'a>(theme: &Theme, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    scrollable(content.into())
        .style(scrollable_style(*theme))
        .height(Length::Fill)
        .into()
}

fn scrollable_style(
    theme: Theme,
) -> impl Fn(&iced::Theme, iced::widget::scrollable::Status) -> iced::widget::scrollable::Style {
    move |_, status| {
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
            border: Border {
                radius: 2.0.into(),
                ..Border::default()
            },
            scroller: iced::widget::scrollable::Scroller {
                background: Background::Color(if active {
                    theme.main.primary
                } else {
                    theme.chrome.high
                }),
                border: Border {
                    radius: 2.0.into(),
                    ..Border::default()
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
                border: Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: theme.border.transient,
                },
                shadow: Shadow::default(),
                icon: theme.font_color,
            },
        }
    }
}

/// 列表行按钮样式。
fn row_style(
    theme: Theme,
    selected: bool,
) -> impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style {
    move |_, status| {
        let background = match status {
            iced::widget::button::Status::Hovered => theme.list.low,
            iced::widget::button::Status::Pressed => theme.base.low,
            _ if selected => theme.list.accent_high,
            _ => Color::TRANSPARENT,
        };

        iced::widget::button::Style {
            background: Some(Background::Color(background)),
            text_color: theme.font_color,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// 一个带图标的模块行。
pub fn module_row<'a>(
    theme: &Theme,
    icon: Icon,
    title: impl Into<String>,
    desc: impl Into<String>,
    message: Message,
) -> Element<'a, Message> {
    row_button(theme, icon, title, desc, None, false, Some(message))
}

/// 一个带选中圆点的选择行。
pub fn select_row<'a>(
    theme: &Theme,
    icon: Icon,
    title: impl Into<String>,
    selected: bool,
    message: Message,
) -> Element<'a, Message> {
    row_button(theme, icon, title, "", None, selected, Some(message))
}

/// 大型列表项（登录方式、模组来源等）。
pub fn large_list_item<'a>(
    theme: &Theme,
    icon: Icon,
    title: impl Into<String>,
    desc: impl Into<String>,
    message: Message,
) -> Element<'a, Message> {
    let theme = *theme;
    let labels = column![
        text(title.into())
            .size(theme.typography.body.size)
            .font(theme.typography.body.font)
            .color(theme.base.medium),
        text(desc.into())
            .size(theme.typography.caption_alt.size)
            .font(theme.typography.caption_alt.font)
            .color(theme.base.medium),
    ]
    .spacing(2);

    let icon = container(icon.view_default(&theme, 20.0))
        .width(Length::Fixed(40.0))
        .center_x(Length::Fixed(40.0));

    iced_button(row![icon, labels].spacing(4).align_y(Alignment::Center))
        .width(Length::Fill)
        .padding([8, 10])
        .style(row_style(theme, false))
        .on_press(message)
        .into()
}

/// 列表项。
pub fn list_item<'a>(
    theme: &Theme,
    icon: Option<Icon>,
    title: impl Into<String>,
    subtitle: Option<String>,
    selected: bool,
    message: Option<Message>,
) -> Element<'a, Message> {
    let theme_copy = *theme;
    let title = title.into();
    let mut labels = column![
        text(title)
            .size(theme.typography.base.size)
            .font(theme.typography.base.font)
            .color(theme.font_color)
    ]
    .spacing(1);

    if let Some(subtitle) = subtitle {
        labels = labels.push(
            text(subtitle)
                .size(theme.typography.caption.size)
                .font(theme.typography.caption.font)
                .color(theme.base.medium),
        );
    }

    let mut content = row![].spacing(8).align_y(Alignment::Center);
    if let Some(icon) = icon {
        content = content.push(icon.view_default(theme, 20.0));
    }
    content = content.push(labels).push(space().width(Length::Fill));

    let mut widget = iced_button(content)
        .width(Length::Fill)
        .padding([5, 8])
        .style(row_style(theme_copy, selected));

    if let Some(message) = message {
        widget = widget.on_press(message);
    }

    widget.into()
}

/// 菜单行，右侧显示箭头。
pub fn menu_item<'a>(
    theme: &Theme,
    title: impl Into<String>,
    desc: impl Into<String>,
    enabled: bool,
    message: Message,
) -> Element<'a, Message> {
    let theme = *theme;
    let title = title.into();
    let desc = desc.into();

    let content = row![
        text(title)
            .size(theme.typography.base.size)
            .font(theme.typography.base.font)
            .color(if enabled {
                theme.font_color
            } else {
                theme.base.medium_low
            }),
        space().width(Length::Fill),
        text(desc)
            .size(theme.typography.caption.size)
            .font(theme.typography.caption.font)
            .color(theme.base.medium),
        icons::CHEVRON_RIGHT.view(&theme, 12.0, theme.base.medium),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut widget = iced_button(content)
        .width(Length::Fill)
        .height(Length::Fixed(32.0))
        .padding([0, 8])
        .style(row_style(theme, false));

    if enabled {
        widget = widget.on_press(message);
    }

    widget.into()
}

/// 菜单行，右侧显示勾选框。
pub fn menu_toggle<'a>(
    theme: &Theme,
    title: impl Into<String>,
    value: bool,
    enabled: bool,
    message: Message,
) -> Element<'a, Message> {
    let theme = *theme;
    let title = title.into();

    let mark = if value {
        container(icons::CHECK.view(&theme, 12.0, theme.chrome.white_high))
            .width(Length::Fixed(18.0))
            .height(Length::Fixed(18.0))
            .center(18)
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(theme.main.primary)),
                border: Border {
                    radius: 3.5.into(),
                    width: 1.0,
                    color: theme.main.secondary,
                },
                ..iced::widget::container::Style::default()
            })
    } else {
        container(space())
            .width(Length::Fixed(18.0))
            .height(Length::Fixed(18.0))
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border {
                    radius: 3.5.into(),
                    width: 1.0,
                    color: theme.base.medium,
                },
                ..iced::widget::container::Style::default()
            })
    };

    let content = row![
        text(title)
            .size(theme.typography.base.size)
            .font(theme.typography.base.font)
            .color(if enabled {
                theme.font_color
            } else {
                theme.base.medium_low
            }),
        space().width(Length::Fill),
        mark,
    ]
    .align_y(Alignment::Center);

    let mut widget = iced_button(content)
        .width(Length::Fill)
        .height(Length::Fixed(32.0))
        .padding([0, 8])
        .style(row_style(theme, false));

    if enabled {
        widget = widget.on_press(message);
    }

    widget.into()
}

fn row_button<'a>(
    theme: &Theme,
    icon: Icon,
    title: impl Into<String>,
    desc: impl Into<String>,
    _trailing: Option<Element<'a, Message>>,
    selected: bool,
    message: Option<Message>,
) -> Element<'a, Message> {
    let theme = *theme;
    let title = title.into();
    let desc = desc.into();

    let mut content = row![
        icon.view_default(&theme, 20.0),
        text(title)
            .size(theme.typography.body.size)
            .font(theme.typography.body.font)
            .color(theme.base.medium),
        space().width(Length::Fill),
        text(desc)
            .size(theme.typography.caption_alt.size)
            .font(theme.typography.caption_alt.font)
            .color(theme.base.medium),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    if selected {
        content = content.push(
            container(space())
                .width(Length::Fixed(8.0))
                .height(Length::Fixed(8.0))
                .style(move |_| iced::widget::container::Style {
                    background: Some(Background::Color(theme.base.high)),
                    border: Border {
                        radius: 4.0.into(),
                        ..Border::default()
                    },
                    ..iced::widget::container::Style::default()
                }),
        );
    } else {
        content = content.push(space().width(Length::Fixed(8.0)));
    }

    let mut widget = iced_button(content)
        .width(Length::Fill)
        .height(Length::Fixed(40.0))
        .padding([0, 10])
        .style(row_style(theme, selected));

    if let Some(message) = message {
        widget = widget.on_press(message);
    }

    widget.into()
}

/// 单选按钮。
pub fn radio<'a>(
    theme: &Theme,
    label_text: impl Into<String>,
    selected: bool,
    message: Message,
) -> Element<'a, Message> {
    let theme = *theme;
    let label_text = label_text.into();
    let value = if selected { 0usize } else { 1usize };

    iced::widget::radio(label_text, value, Some(0usize), move |_| message)
        .size(18.0)
        .spacing(8)
        .style(move |_, status| {
            let selected = matches!(
                status,
                iced::widget::radio::Status::Active { is_selected: true }
                    | iced::widget::radio::Status::Hovered { is_selected: true }
            );
            iced::widget::radio::Style {
                background: Background::Color(if selected {
                    theme.main.primary
                } else {
                    Color::TRANSPARENT
                }),
                dot_color: theme.chrome.white_high,
                border_width: 1.5,
                border_color: if selected {
                    theme.main.primary
                } else {
                    theme.base.medium
                },
                text_color: Some(if selected {
                    theme.base.high
                } else {
                    theme.base.medium
                }),
            }
        })
        .into()
}

/// 开关。
pub fn toggle_switch<'a>(
    theme: &Theme,
    value: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    widgets::toggle_switch(theme, value, on_toggle)
}

/// 进度条。
pub fn progress<'a>(theme: &Theme, value: f32, height: f32) -> Element<'a, Message> {
    widgets::progress_bar(theme, value, height)
}

/// 不确定进度条。
pub fn indeterminate<'a>(theme: &Theme, phase: f32, height: f32) -> Element<'a, Message> {
    widgets::indeterminate_progress(theme, phase, height)
}

/// 居中显示的加载状态。
pub fn loading<'a>(theme: &Theme, phase: f32, message: impl Into<String>) -> Element<'a, Message> {
    let theme = *theme;
    container(
        column![
            container(indeterminate(&theme, phase, 4.0)).width(Length::Fixed(120.0)),
            caption(&theme, message),
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

/// 居中的空结果提示。
pub fn empty_state<'a>(theme: &Theme, message: impl Into<String>) -> Element<'a, Message> {
    let theme = *theme;
    container(label(&theme, message))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// 居中的错误提示。
pub fn error_state<'a>(theme: &Theme, message: impl Into<String>) -> Element<'a, Message> {
    let theme = *theme;
    container(
        iced::widget::text(message.into())
            .size(16)
            .color(theme.main.primary),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

/// 一个可折叠版本分组。
pub fn folder<'a>(
    theme: &Theme,
    key: &str,
    title: &str,
    collapsed: bool,
    rows: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let theme = *theme;

    let header = row![
        icons::FOLDER.view_default(&theme, 18.0),
        text(title.to_owned())
            .size(theme.typography.body.size)
            .font(theme.typography.body.font)
            .color(theme.font_color),
        space().width(Length::Fill),
        text(if collapsed { "▸" } else { "▾" })
            .size(13.0)
            .color(theme.base.medium),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let header = iced_button(header)
        .width(Length::Fill)
        .height(Length::Fixed(36.0))
        .padding([0, 10])
        .style(row_style(theme, false))
        .on_press(ui(crate::ui::UiMessage::ToggleFolder(key.to_owned())));

    let mut content = column![header].spacing(0);
    if !collapsed {
        for row in rows {
            content = content.push(row);
        }
    }

    content.into()
}

/// 内存状态（支持测试用 `SCL_MOCK_MEM`）。
pub fn memory_status() -> MemoryStatus {
    if let Ok(value) = std::env::var("SCL_MOCK_MEM") {
        if let Some((max, free)) = value.split_once(',') {
            if let (Ok(max), Ok(free)) = (max.trim().parse(), free.trim().parse()) {
                return MemoryStatus { max, free };
            }
        }
    }
    get_mem_status()
}

/// 按官方截图格式化内存文本。
pub fn format_memory_text(mb: f64) -> String {
    if mb <= 0.0 {
        "0 B".to_owned()
    } else if mb >= 1000.0 {
        format!("{:.1} GB", mb / 1000.0)
    } else {
        format!("{:.1} MB", mb)
    }
}

/// 内存条：显示系统占用与游戏分配。
pub fn memory_bar<'a>(theme: &Theme, game_mb: Option<usize>) -> Element<'a, Message> {
    let theme = *theme;
    let status = memory_status();
    let total = status.max.max(1) as f32;
    let used = status.max.saturating_sub(status.free) as f32;
    let game = match game_mb {
        Some(value) => value as f32,
        None => (status.free as f64 * 0.75).min(4000.0) as f32,
    };

    let used_portion = ((used / total) * 1000.0).max(0.0) as u16;
    let game_portion = ((game.min(total - used) / total) * 1000.0).max(0.0) as u16;
    let free_portion = 1000u16
        .saturating_sub(used_portion)
        .saturating_sub(game_portion)
        .max(1);

    let track = iced::widget::row![
        container(space())
            .width(Length::FillPortion(used_portion.max(1)))
            .height(Length::Fixed(6.0))
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(theme.main.primary)),
                ..iced::widget::container::Style::default()
            }),
        container(space())
            .width(Length::FillPortion(game_portion.max(1)))
            .height(Length::Fixed(6.0))
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(Color {
                    a: 0.4,
                    ..theme.main.primary
                })),
                ..iced::widget::container::Style::default()
            }),
        container(space())
            .width(Length::FillPortion(free_portion))
            .height(Length::Fixed(6.0))
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(theme.base.medium_low)),
                ..iced::widget::container::Style::default()
            }),
    ]
    .height(Length::Fixed(6.0));

    container(track)
        .width(Length::Fill)
        .height(Length::Fixed(6.0))
        .clip(true)
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(theme.base.low)),
            border: Border {
                radius: 3.0.into(),
                ..Border::default()
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

/// 默认头像像素。
pub(crate) const DEFAULT_HEAD: [u8; 8 * 8 * 4] = [
    51, 36, 17, 255, 51, 36, 17, 255, 63, 42, 21, 255, 63, 42, 21, 255, 63, 42, 21, 255, 63, 42,
    21, 255, 51, 36, 17, 255, 43, 30, 13, 255, 36, 24, 8, 255, 51, 36, 17, 255, 51, 36, 17, 255,
    63, 42, 21, 255, 63, 42, 21, 255, 51, 36, 17, 255, 63, 42, 21, 255, 51, 36, 17, 255, 43, 30,
    13, 255, 155, 99, 73, 255, 179, 121, 94, 255, 183, 131, 107, 255, 179, 121, 94, 255, 170, 114,
    89, 255, 155, 99, 73, 255, 52, 37, 18, 255, 155, 99, 73, 255, 170, 114, 89, 255, 179, 121, 94,
    255, 179, 121, 94, 255, 170, 114, 89, 255, 170, 114, 89, 255, 170, 114, 89, 255, 155, 99, 73,
    255, 170, 114, 89, 255, 255, 255, 255, 255, 82, 61, 137, 255, 170, 114, 89, 255, 155, 99, 73,
    255, 82, 61, 137, 255, 255, 255, 255, 255, 170, 114, 89, 255, 155, 99, 73, 255, 170, 114, 89,
    255, 170, 114, 89, 255, 106, 64, 48, 255, 106, 64, 48, 255, 170, 114, 89, 255, 170, 114, 89,
    255, 155, 99, 73, 255, 144, 89, 63, 255, 143, 94, 62, 255, 73, 37, 16, 255, 119, 66, 53, 255,
    119, 66, 53, 255, 66, 29, 10, 255, 143, 94, 62, 255, 129, 83, 57, 255, 148, 96, 62, 255, 129,
    83, 57, 255, 66, 29, 10, 255, 73, 37, 16, 255, 66, 29, 10, 255, 73, 37, 16, 255, 129, 83, 57,
    255, 143, 94, 62, 255,
];

/// 一个 8×8 头像。
pub fn head_image<'a>(_theme: &Theme, skin: &[u8], size: f32) -> Element<'a, Message> {
    let skin = if skin.len() == 8 * 8 * 4 {
        skin
    } else {
        &DEFAULT_HEAD
    };
    let handle = iced::widget::image::Handle::from_rgba(8, 8, skin.to_vec());
    let image = iced::widget::image(handle)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .filter_method(iced::widget::image::FilterMethod::Nearest);

    container(image)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

/// 带帽子层的头像。
pub fn head_with_hat<'a>(
    theme: &Theme,
    skin: &[u8],
    hat: &[u8],
    size: f32,
) -> Element<'a, Message> {
    let head = head_image(theme, skin, size);
    if hat.len() == 8 * 8 * 4 {
        let handle = iced::widget::image::Handle::from_rgba(8, 8, hat.to_vec());
        let hat = iced::widget::image(handle)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .filter_method(iced::widget::image::FilterMethod::Nearest);
        stack![head, hat].into()
    } else {
        head
    }
}

/// 一个模组图标。
pub fn mod_icon<'a>(theme: &Theme, icon: &str, size: f32) -> Element<'a, Message> {
    let theme = *theme;
    if !icon.is_empty() {
        if let Ok((width, height, pixels)) = crate::utils::image::decode_image_buf(icon) {
            let handle =
                iced::widget::image::Handle::from_rgba(width as u32, height as u32, pixels);
            return iced::widget::image(handle)
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .filter_method(iced::widget::image::FilterMethod::Linear)
                .into();
        }
    }

    container(icons::MODS.view_default(&theme, size * 0.7))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(theme.base.low)),
            border: Border {
                radius: 4.0.into(),
                ..Border::default()
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

/// 一个模组信息行。
pub fn mod_info_row<'a>(
    theme: &Theme,
    info: &crate::app_state::ModState,
    message: Option<Message>,
) -> Element<'a, Message> {
    let theme = *theme;
    let title = if info.cname.is_empty() {
        info.name.clone()
    } else {
        format!("[{}] {}", info.cname, info.name)
    };

    let content = row![
        mod_icon(&theme, &info.icon, 32.0),
        column![
            iced::widget::text(title)
                .size(theme.typography.body.size)
                .font(theme.typography.body.font)
                .color(theme.base.high),
            iced::widget::text(info.description.clone())
                .size(theme.typography.caption_alt.size)
                .font(theme.typography.caption_alt.font)
                .color(theme.base.medium),
        ]
        .spacing(2)
        .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([8, 8])
    .width(Length::Fill);

    let mut widget = iced_button(content)
        .width(Length::Fill)
        .height(Length::Fixed(48.0))
        .padding(0)
        .style(row_style(theme, false));

    if let Some(message) = message {
        widget = widget.on_press(message);
    }

    widget.into()
}
