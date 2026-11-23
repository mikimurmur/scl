//! SCL 图形组件库，基于 [Iced](https://iced.rs) 和 tiny-skia 软件渲染器，
//! 提供 WinUI 3 / Fluent Design 风格的界面组件。
//!
//! 组件库由旧的 Druid 版本就地迁移而来，`scl-gui` 和各页面现在都直接使用
//! 这里的声明式组件；不再依赖 Druid 或 wgpu。

#![forbid(missing_docs)]

pub mod icons;
pub mod theme;
pub mod widgets;

/// 组件库使用的元素类型别名。
pub type Element<'a, Message> = iced::Element<'a, Message, iced::Theme, iced::Renderer>;
