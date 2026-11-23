//! 启动器更新页面。

use iced::widget::{column, container, space};
use iced::{Alignment, Element, Length};

use crate::{
    app::{Message, UPDATE_NOW},
    app_state::AppState,
    ui::common,
};

fn loading_page(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    container(
        column![
            space().height(Length::Fill),
            common::indeterminate(&theme, state.animation_phase, 4.0),
            space().height(Length::Fixed(10.0)),
            common::label(&theme, "正在获取更新记录"),
            space().height(Length::Fill),
        ]
        .spacing(4)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn loaded_page(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let version = state.update_available.as_deref().unwrap_or("最新版本");
    let logs = state
        .update_logs
        .clone()
        .unwrap_or_else(|| "正在获取更新记录".to_owned());

    let header = common::label(&theme, format!("自当前版本到  {version} 的更新记录："));

    column![
        container(header).padding(5),
        container(common::scroll(
            &theme,
            container(
                iced::widget::text(logs)
                    .size(theme.typography.body.size)
                    .font(theme.typography.body.font)
                    .color(theme.font_color)
            )
            .padding(5)
        ))
        .height(Length::Fill),
        container(common::accent_button(
            &theme,
            "立刻更新！",
            common::command(UPDATE_NOW, ())
        ))
        .padding([5, 12]),
    ]
    .padding([0, 12])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// 创建启动器更新页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    if state.update_logs.is_some() {
        loaded_page(state)
    } else {
        loading_page(state)
    }
}
