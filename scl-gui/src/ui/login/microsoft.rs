//! Microsoft 正版登录页面。

use iced::widget::{column, container, row, space};
use iced::{Alignment, Element, Length};

use crate::{
    app::{Message, OPEN_URL},
    app_state::AppState,
    ui::{UiMessage, common},
};

fn progress_view<'a>(state: &AppState, text: &str) -> Element<'a, Message> {
    let theme = state.theme;
    container(
        column![common::loading(&theme, state.animation_phase, text),]
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// 创建 Microsoft 登录页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    if let Some(error) = state.login_error.as_ref().filter(|error| !error.is_empty()) {
        return container(
            column![
                iced::widget::text(error.clone())
                    .size(theme.typography.body.size)
                    .font(theme.typography.body.font)
                    .color(theme.main.primary),
                space().height(Length::Fill),
            ]
            .padding(10),
        )
        .into();
    }

    if state.login_microsoft_code.is_empty() {
        return progress_view(state, "正在获取用户代码……");
    }

    if state.login_microsoft_success {
        return progress_view(state, "验证成功！正在登录……");
    }

    let code = iced::widget::text(state.login_microsoft_code.clone())
        .size(32.0)
        .font(theme.typography.base.font)
        .color(theme.font_color);

    let hint = iced::widget::text(
        "请点击下方按钮，在弹出的验证网页中输入用户代码并完成验证。\n验证期间请不要退出这个页面。",
    )
    .size(theme.typography.base.size)
    .font(theme.typography.base.font)
    .color(theme.font_color);

    let open = common::accent_button(
        &theme,
        "打开验证网站",
        common::command(OPEN_URL, state.login_microsoft_url.clone()),
    );
    let open = container(open).width(Length::FillPortion(1));

    let copy = common::button(
        &theme,
        "复制用户代码",
        scl_gui_widgets::widgets::ButtonKind::Standard,
        Message::Ui(UiMessage::CopyText(state.login_microsoft_code.clone())),
    );
    let copy = container(copy).width(Length::FillPortion(1));

    container(
        column![
            space().height(Length::Fill),
            common::label(&theme, "你的用户代码是"),
            space().height(Length::Fixed(10.0)),
            code,
            space().height(Length::Fixed(10.0)),
            hint,
            space().height(Length::Fill),
            row![open, space().width(Length::Fixed(10.0)), copy]
                .align_y(Alignment::Center)
                .width(Length::Fill),
        ]
        .spacing(4)
        .padding(10)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .into()
}
