//! 各个页面共用的界面组件。

use druid::{
    widget::{Button as DruidButton, CrossAxisAlignment, Flex, LabelText, TextBox},
    Widget, WidgetExt,
};
use scl_gui_widgets::widgets::Button as SclButton;

use crate::app_state::{AppState, ModState};

/// 创建一个普通按钮
pub fn button(text: impl Into<LabelText<AppState>>) -> DruidButton<AppState> {
    DruidButton::new(text)
}

/// 创建一个强调色按钮
pub fn accent_button(text: impl Into<LabelText<AppState>>) -> SclButton<AppState> {
    SclButton::new(text).with_accent(true)
}

/// 创建一个文本输入框
pub fn text_box(placeholder: &'static str) -> TextBox<String> {
    TextBox::new().with_placeholder(placeholder)
}

/// 创建一个纵向排列的容器
pub fn column() -> Flex<AppState> {
    Flex::column().cross_axis_alignment(CrossAxisAlignment::Start)
}

/// 创建一个表单字段标题标签
///
/// 原版页面把字段标题强制为 26 逻辑像素高，保证不同字体下垂直间距一致。
pub fn field_label(text: &'static str) -> Box<dyn Widget<AppState>> {
    Box::new(scl_gui_widgets::widgets::label::new(text).fix_height(26.))
}

/// 创建一个模组信息展示块：图标 + 名称（含中文名）+ 单行简介
pub fn mod_info() -> Box<dyn Widget<ModState>> {
    Box::new(crate::widgets::ModListItem::new_static())
}
