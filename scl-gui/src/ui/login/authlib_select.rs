//! Authlib 登录的角色选择独立页面。

use iced::widget::{column, container};
use iced::{Element, Length};
use scl_gui_widgets::icons;

use crate::{
    app::{Message, SELECT_AUTHLIB_CHARACTER},
    app_state::AppState,
    ui::common,
};

/// 创建 Authlib 角色选择页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let mut list = column![
        container(common::label(
            &theme,
            "此账户包含了多个角色，请选择一个来添加："
        ))
        .padding(5)
    ];

    for (index, account) in state.login_authlib_available_characters.iter().enumerate() {
        list = list.push(common::large_list_item(
            &theme,
            icons::LOGIN_AUTHLIB,
            account.display_name(),
            account.uuid().to_owned(),
            common::command(SELECT_AUTHLIB_CHARACTER, index),
        ));
    }

    container(common::scroll(&theme, list))
        .padding(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
