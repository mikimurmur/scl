//! 根据 Windows UI（原 Fluent Design）规范整理的颜色主题。

use iced::{Color, Theme as IcedTheme};

/// 主题模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// 亮色模式。
    #[default]
    Light,
    /// 暗色模式。
    Dark,
}

/// 基本颜色组合，通常用于在背景上显示文字、图标等内容。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaseColors {
    /// 默认情况下亮色为 `#000000 - 20%`，暗色为 `#FFFFFF - 20%`。
    pub low: Color,
    /// 默认情况下亮色为 `#000000 - 40%`，暗色为 `#FFFFFF - 40%`。
    pub medium_low: Color,
    /// 默认情况下亮色为 `#000000 - 60%`，暗色为 `#FFFFFF - 60%`。
    pub medium: Color,
    /// 默认情况下亮色为 `#000000 - 80%`，暗色为 `#FFFFFF - 80%`。
    pub medium_high: Color,
    /// 默认情况下亮色为 `#000000`，暗色为 `#FFFFFF`。
    pub high: Color,
}

/// 次颜色组合，颜色与 [`BaseColors`] 相反。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AltColors {
    /// 默认情况下亮色为 `#FFFFFF - 20%`，暗色为 `#000000 - 20%`。
    pub low: Color,
    /// 默认情况下亮色为 `#FFFFFF - 40%`，暗色为 `#000000 - 40%`。
    pub medium_low: Color,
    /// 默认情况下亮色为 `#FFFFFF - 60%`，暗色为 `#000000 - 60%`。
    pub medium: Color,
    /// 默认情况下亮色为 `#FFFFFF - 80%`，暗色为 `#000000 - 80%`。
    pub medium_high: Color,
    /// 默认情况下亮色为 `#FFFFFF`，暗色为 `#000000`。
    pub high: Color,
}

/// 外壳颜色组合。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChromeColors {
    /// 亮色 `#FFFFFF`，暗色 `#FFFFFF`。
    pub white_high: Color,
    /// 亮色 `#F2F2F2`，暗色 `#2B2B2B`。
    pub medium_low: Color,
    /// 亮色 `#F2F2F2`，暗色 `#171717`。
    pub low: Color,
    /// 亮色 `#CCCCCC`，暗色 `#333333`。
    pub disabled_high: Color,
    /// 亮色 `#E6E6E6`，暗色 `#1F1F1F`。
    pub medium: Color,
    /// 亮色 `#CCCCCC`，暗色 `#767676`。
    pub high: Color,
    /// 亮色 `#000000 - 20%`，暗色 `#000000 - 20%`。
    pub black_low: Color,
    /// 亮色 `#000000 - 40%`，暗色 `#000000 - 40%`。
    pub black_medium_low: Color,
    /// 亮色 `#7A7A7A`，暗色 `#858585`。
    pub disabled_low: Color,
    /// 亮色 `#000000 - 80%`，暗色 `#000000 - 80%`。
    pub black_medium: Color,
    /// 亮色 `#000000 - 40%`，暗色 `#F2F2F2`。
    pub alt_low: Color,
    /// 亮色 `#000000`，暗色 `#000000`。
    pub black_high: Color,
    /// 亮色 `#FFFFFF`，暗色 `#FFFFFF`。
    pub white: Color,
}

/// 列表颜色组合。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListColors {
    /// 强调色 20%。
    pub accent_high: Color,
    /// 强调色 40%。
    pub accent_medium: Color,
    /// 强调色 60%。
    pub accent_low: Color,
    /// 亮色 `#000000 - 10%`，暗色 `#000000 - 10%`。
    pub low: Color,
    /// 亮色 `#000000 - 20%`，暗色 `#000000 - 20%`。
    pub medium: Color,
}

/// 边框颜色组合。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderColors {
    /// 亮色 `#FFFFFF - 60%`，暗色 `#FFFFFF - 5%`。
    pub edge_highlight: Color,
    /// 亮色 `#000000 - 14%`，暗色 `#000000 - 36%`。
    pub transient: Color,
}

/// 主要强调色组合。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccentColors {
    /// 默认强调色。
    pub accent: Color,
    /// 较亮的强调色。
    pub accent_1: Color,
    /// 较暗的强调色。
    pub accent_dark_1: Color,
    /// 更亮的强调色。
    pub accent_light_1: Color,
}

/// SCL 自己的品牌色。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MainColors {
    /// 品牌主色，默认 `#F74C00`。
    pub primary: Color,
    /// 品牌次色，默认 `#CC3F00`。
    pub secondary: Color,
    /// 标题栏颜色。
    pub title_bar: Color,
}

/// 一套完整的 SCL 主题。
///
/// 所有字段都是 [`Copy`]，因此可以很方便地被 Iced 的样式闭包捕获。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// 当前是亮色还是暗色模式。
    pub mode: Mode,
    /// 基本颜色。
    pub base: BaseColors,
    /// 次颜色。
    pub alt: AltColors,
    /// 外壳颜色。
    pub chrome: ChromeColors,
    /// 列表颜色。
    pub list: ListColors,
    /// 边框颜色。
    pub border: BorderColors,
    /// 强调色。
    pub accent: AccentColors,
    /// 品牌色。
    pub main: MainColors,
    /// 正文字体颜色。
    pub font_color: Color,
    /// 窗口背景色。
    pub window_background: Color,
    /// 字体配置。
    pub typography: crate::theme::Typography,
}

impl Theme {
    /// 亮色主题。
    pub const fn light() -> Self {
        Self {
            mode: Mode::Light,
            base: BaseColors {
                low: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
                medium_low: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                medium: Color::from_rgba(0.0, 0.0, 0.0, 0.6),
                medium_high: Color::from_rgba(0.0, 0.0, 0.0, 0.8),
                high: Color::BLACK,
            },
            alt: AltColors {
                low: Color::from_rgba(1.0, 1.0, 1.0, 0.2),
                medium_low: Color::from_rgba(1.0, 1.0, 1.0, 0.4),
                medium: Color::from_rgba(1.0, 1.0, 1.0, 0.6),
                medium_high: Color::from_rgba(1.0, 1.0, 1.0, 0.8),
                high: Color::WHITE,
            },
            chrome: ChromeColors {
                white_high: Color::from_rgb8(0xFF, 0xFF, 0xFF),
                medium_low: Color::from_rgb8(0xF2, 0xF2, 0xF2),
                low: Color::from_rgb8(0xF2, 0xF2, 0xF2),
                disabled_high: Color::from_rgb8(0xCC, 0xCC, 0xCC),
                medium: Color::from_rgb8(0xE6, 0xE6, 0xE6),
                high: Color::from_rgb8(0xCC, 0xCC, 0xCC),
                black_low: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
                black_medium_low: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                disabled_low: Color::from_rgb8(0x7A, 0x7A, 0x7A),
                black_medium: Color::from_rgba(0.0, 0.0, 0.0, 0.8),
                alt_low: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                black_high: Color::BLACK,
                white: Color::WHITE,
            },
            list: ListColors {
                accent_high: Color::from_rgba(0.0, 0.471, 0.831, 0.2),
                accent_medium: Color::from_rgba(0.0, 0.471, 0.831, 0.4),
                accent_low: Color::from_rgba(0.0, 0.471, 0.831, 0.6),
                low: Color::from_rgba(0.0, 0.0, 0.0, 0.1),
                medium: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
            },
            border: BorderColors {
                edge_highlight: Color::from_rgba(1.0, 1.0, 1.0, 0.6),
                transient: Color::from_rgba(0.0, 0.0, 0.0, 0.14),
            },
            accent: AccentColors {
                accent: Color::from_rgb8(0xF7, 0x4C, 0x00),
                accent_1: Color::from_rgb8(0xFF, 0x72, 0x33),
                accent_dark_1: Color::from_rgb8(0xD2, 0x41, 0x00),
                accent_light_1: Color::from_rgb8(0xFF, 0x72, 0x33),
            },
            main: MainColors {
                primary: Color::from_rgb8(0xF7, 0x4C, 0x00),
                secondary: Color::from_rgb8(0xCC, 0x3F, 0x00),
                title_bar: Color::from_rgba(1.0, 1.0, 1.0, 0.0),
            },
            font_color: Color::from_rgb8(0x20, 0x1F, 0x1E),
            window_background: Color::from_rgb8(0xF3, 0xF3, 0xF3),
            typography: crate::theme::Typography::new(),
        }
    }

    /// 暗色主题。
    pub const fn dark() -> Self {
        Self {
            mode: Mode::Dark,
            base: BaseColors {
                low: Color::from_rgba(1.0, 1.0, 1.0, 0.2),
                medium_low: Color::from_rgba(1.0, 1.0, 1.0, 0.4),
                medium: Color::from_rgba(1.0, 1.0, 1.0, 0.6),
                medium_high: Color::from_rgba(1.0, 1.0, 1.0, 0.8),
                high: Color::WHITE,
            },
            alt: AltColors {
                low: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
                medium_low: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                medium: Color::from_rgba(0.0, 0.0, 0.0, 0.6),
                medium_high: Color::from_rgba(0.0, 0.0, 0.0, 0.8),
                high: Color::BLACK,
            },
            chrome: ChromeColors {
                white_high: Color::from_rgb8(0xFF, 0xFF, 0xFF),
                medium_low: Color::from_rgb8(0x2B, 0x2B, 0x2B),
                low: Color::from_rgb8(0x17, 0x17, 0x17),
                disabled_high: Color::from_rgb8(0x33, 0x33, 0x33),
                medium: Color::from_rgb8(0x1F, 0x1F, 0x1F),
                high: Color::from_rgb8(0x76, 0x76, 0x76),
                black_low: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
                black_medium_low: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                disabled_low: Color::from_rgb8(0x85, 0x85, 0x85),
                black_medium: Color::from_rgba(0.0, 0.0, 0.0, 0.8),
                alt_low: Color::from_rgb8(0xF2, 0xF2, 0xF2),
                black_high: Color::BLACK,
                white: Color::WHITE,
            },
            list: ListColors {
                accent_high: Color::from_rgba(0.0, 0.471, 0.831, 0.2),
                accent_medium: Color::from_rgba(0.0, 0.471, 0.831, 0.4),
                accent_low: Color::from_rgba(0.0, 0.471, 0.831, 0.6),
                low: Color::from_rgba(0.0, 0.0, 0.0, 0.1),
                medium: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
            },
            border: BorderColors {
                edge_highlight: Color::from_rgba(1.0, 1.0, 1.0, 0.6),
                transient: Color::from_rgba(0.0, 0.0, 0.0, 0.14),
            },
            accent: AccentColors {
                accent: Color::from_rgb8(0xF7, 0x4C, 0x00),
                accent_1: Color::from_rgb8(0xFF, 0x72, 0x33),
                accent_dark_1: Color::from_rgb8(0xD2, 0x41, 0x00),
                accent_light_1: Color::from_rgb8(0xFF, 0x72, 0x33),
            },
            main: MainColors {
                primary: Color::from_rgb8(0xF7, 0x4C, 0x00),
                secondary: Color::from_rgb8(0xCC, 0x3F, 0x00),
                title_bar: Color::from_rgba(0.0, 0.0, 0.0, 0.0),
            },
            font_color: Color::WHITE,
            window_background: Color::from_rgb8(0x20, 0x20, 0x20),
            typography: crate::theme::Typography::new(),
        }
    }

    /// 是否处于暗色模式。
    pub const fn is_dark(&self) -> bool {
        matches!(self.mode, Mode::Dark)
    }

    /// 切换亮色/暗色模式。
    pub const fn toggled(&self) -> Self {
        match self.mode {
            Mode::Light => Self::dark(),
            Mode::Dark => Self::light(),
        }
    }

    /// 修改品牌主色。
    pub const fn with_primary(mut self, color: Color) -> Self {
        self.main.primary = color;
        self
    }

    /// 修改品牌次色。
    pub const fn with_secondary(mut self, color: Color) -> Self {
        self.main.secondary = color;
        self
    }

    /// 修改强调色。
    pub const fn with_accent(mut self, color: Color) -> Self {
        self.accent.accent = color;
        self
    }

    /// 修改标题栏颜色。
    pub const fn with_title_bar(mut self, color: Color) -> Self {
        self.main.title_bar = color;
        self
    }

    /// 修改窗口背景色。
    pub const fn with_window_background(mut self, color: Color) -> Self {
        self.window_background = color;
        self
    }

    /// 转换成 Iced 自己的主题，供内置组件和滚动条使用。
    pub fn iced_theme(&self) -> IcedTheme {
        IcedTheme::custom(
            if self.is_dark() {
                "SCL Dark"
            } else {
                "SCL Light"
            },
            iced::theme::Palette {
                background: self.window_background,
                text: self.font_color,
                primary: self.main.primary,
                success: Color::from_rgb8(0x10, 0x7C, 0x41),
                warning: Color::from_rgb8(0xF7, 0x63, 0x0C),
                danger: Color::from_rgb8(0xC4, 0x2B, 0x1C),
            },
        )
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

/// 根据背景色计算一个对比度足够高的前景色（YIQ 算法）。
pub fn contrast_color(background: Color) -> Color {
    let yiq = background.r * 0.299 + background.g * 0.587 + background.b * 0.114;
    if yiq >= 0.5 {
        Color::BLACK
    } else {
        Color::WHITE
    }
}
