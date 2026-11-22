//! 字体配置，对应原组件库里的 `typography` 颜色键组。

use iced::font::{Family, Weight};
use iced::{Font, Pixels};

/// 一个字体样式：字体 + 字号。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    /// 字体。
    pub font: Font,
    /// 字号。
    pub size: Pixels,
}

impl TextStyle {
    /// 创建一个新的字体样式。
    pub const fn new(font: Font, size: f32) -> Self {
        Self {
            font,
            size: Pixels(size),
        }
    }

    /// 修改字号。
    pub const fn with_size(mut self, size: f32) -> Self {
        self.size = Pixels(size);
        self
    }
}

/// 一套完整的字体配置。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Typography {
    /// 副标题字体，34px。
    pub subheader: TextStyle,
    /// 大标题字体，46px。
    pub header: TextStyle,
    /// 标题字体，24px。
    pub title: TextStyle,
    /// 说明文字，12px。
    pub caption: TextStyle,
    /// 替代说明文字，13px。
    pub caption_alt: TextStyle,
    /// 正文字体，14px。
    pub body: TextStyle,
    /// 替代副标题，18px。
    pub subtitle_alt: TextStyle,
    /// 副标题，20px。
    pub subtitle: TextStyle,
    /// 基本字体，14px。
    pub base: TextStyle,
    /// 替代基本字体，14px 半粗。
    pub base_alt: TextStyle,
}

impl Typography {
    /// 创建默认字体配置。
    pub const fn new() -> Self {
        let regular = font(Weight::Normal);
        let light = font(Weight::Light);
        let extra_light = font(Weight::ExtraLight);
        let semibold = font(Weight::Semibold);

        Self {
            subheader: TextStyle::new(extra_light, 34.0),
            header: TextStyle::new(extra_light, 46.0),
            title: TextStyle::new(extra_light, 24.0),
            caption: TextStyle::new(light, 12.0),
            caption_alt: TextStyle::new(light, 13.0),
            body: TextStyle::new(regular, 14.0),
            subtitle_alt: TextStyle::new(regular, 18.0),
            subtitle: TextStyle::new(regular, 20.0),
            base: TextStyle::new(regular, 14.0),
            base_alt: TextStyle::new(semibold, 14.0),
        }
    }

    /// 获取指定字号的正文字体。
    pub const fn body_with_size(self, size: f32) -> TextStyle {
        self.body.with_size(size)
    }

    /// 获取指定字号的说明字体。
    pub const fn caption_with_size(self, size: f32) -> TextStyle {
        self.caption.with_size(size)
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self::new()
    }
}

/// 系统 UI 字体。
pub const fn font(weight: Weight) -> Font {
    Font {
        family: Family::SansSerif,
        weight,
        ..Font::DEFAULT
    }
}
