//! 删除版本确认页面。

use iced::widget::{column, space};
use iced::{Element, Length};

use crate::{
    app::{Message, REMOVE_VERSION},
    app_state::AppState,
    ui::common,
};

/// 创建删除版本确认页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let content = column![
        common::title(&theme, "你认真的？"),
        space().height(Length::Fixed(10.0)),
        common::label(
            &theme,
            format!("确定要删除版本 {} 吗？", state.version_setting_version_name)
        ),
        space().height(Length::Fixed(10.0)),
        iced::widget::text("这个操作不可撤销！")
            .size(theme.typography.base.size)
            .font(theme.typography.base.font)
            .color(iced::Color::from_rgb8(255, 0, 0)),
        space().height(Length::Fill),
        common::button(
            &theme,
            "删除版本",
            scl_gui_widgets::widgets::ButtonKind::Danger,
            common::command(REMOVE_VERSION, ()),
        ),
    ]
    .spacing(0)
    .padding(10)
    .width(Length::Fill)
    .height(Length::Fill);

    content.into()
}
