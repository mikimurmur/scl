//! SCL 图形组件库的 [Iced](https://iced.rs)（tiny-skia）迁移实验。
//!
//! 这个 crate 是 `scl-gui-widgets` 的并行迁移版本。旧组件库仍然基于
//! Druid，方便现有启动器继续工作；这里则用 Iced 的声明式 UI 和
//! tiny-skia 软件渲染器重新实现同一套 WinUI 3 / Fluent Design 风格。
//!
//! 当前迁移状态见 [`README.md`](../README.md)。

#![forbid(missing_docs)]

pub mod icons;
pub mod theme;
pub mod widgets;

/// 组件库使用的元素类型别名。
pub type Element<'a, Message> = iced::Element<'a, Message, iced::Theme, iced::Renderer>;
