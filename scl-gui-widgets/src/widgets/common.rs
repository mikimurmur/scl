//! 组件共用的样式辅助函数。

use iced::border::Border;
use iced::widget::{button, container};
use iced::{Background, Color, Shadow, Theme as IcedTheme};

use crate::theme::{TextStyle, Theme};

/// 按钮外观。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonKind {
    /// 普通按钮。
    #[default]
    Standard,
    /// 使用品牌主色的强调按钮。
    Accent,
    /// 没有背景和边框的扁平按钮。
    Flat,
    /// 危险操作按钮。
    Danger,
}

impl ButtonKind {
    /// 返回按钮在指定状态下的样式。
    pub fn style(self, theme: Theme) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
        move |_iced_theme, status| {
            let disabled = matches!(status, button::Status::Disabled);
            let hovered = matches!(status, button::Status::Hovered);
            let pressed = matches!(status, button::Status::Pressed);

            let (background, text_color, border_color) = match self {
                Self::Accent => {
                    let background = if disabled {
                        theme.base.low
                    } else if pressed {
                        theme.accent.accent_dark_1
                    } else if hovered {
                        theme.accent.accent_light_1
                    } else {
                        theme.accent.accent
                    };
                    (background, theme.chrome.white_high, Color::TRANSPARENT)
                }
                Self::Danger => {
                    let background = if disabled {
                        theme.base.low
                    } else if pressed {
                        Color::from_rgb8(0x9A, 0x1F, 0x14)
                    } else if hovered {
                        Color::from_rgb8(0xD9, 0x3A, 0x2B)
                    } else {
                        Color::from_rgb8(0xC4, 0x2B, 0x1C)
                    };
                    (background, theme.chrome.white_high, Color::TRANSPARENT)
                }
                Self::Flat => {
                    let background = if disabled {
                        Color::TRANSPARENT
                    } else if pressed {
                        theme.base.low
                    } else if hovered {
                        theme.list.low
                    } else {
                        Color::TRANSPARENT
                    };
                    (
                        background,
                        if disabled {
                            theme.base.medium_low
                        } else {
                            theme.font_color
                        },
                        Color::TRANSPARENT,
                    )
                }
                Self::Standard => {
                    let background = if disabled {
                        theme.base.low
                    } else if pressed {
                        theme.base.medium_low
                    } else if hovered {
                        theme.list.low
                    } else {
                        theme.base.low
                    };
                    (
                        background,
                        if disabled {
                            theme.base.medium_low
                        } else {
                            theme.font_color
                        },
                        theme.border.transient,
                    )
                }
            };

            button::Style {
                background: Some(Background::Color(background)),
                text_color,
                border: Border {
                    radius: 4.0.into(),
                    width: if matches!(self, Self::Flat) { 0.0 } else { 1.0 },
                    color: border_color,
                },
                shadow: Shadow::default(),
                snap: true,
            }
        }
    }
}

/// 创建一个带主题字体的文本组件。
pub(crate) fn styled_text<'a>(
    content: impl Into<String>,
    style: TextStyle,
    color: Color,
) -> iced::widget::Text<'a, IcedTheme, iced::Renderer> {
    iced::widget::text(content.into())
        .size(style.size)
        .font(style.font)
        .color(color)
}

/// 卡片容器的样式。
pub(crate) fn card_style(theme: Theme) -> impl Fn(&IcedTheme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(theme.chrome.medium_low)),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: theme.border.transient,
        },
        ..container::Style::default()
    }
}

/// 页面背景容器的样式。
pub(crate) fn page_style(theme: Theme) -> impl Fn(&IcedTheme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(theme.window_background)),
        ..container::Style::default()
    }
}
