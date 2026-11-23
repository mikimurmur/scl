//! 登录方式选择页面。

use iced::widget::{column, container};
use iced::{Element, Length};
use scl_gui_widgets::icons;

use crate::{
    app::Message,
    app_state::AppState,
    ui::{PAGE_LOGIN_AUTHLIB, PAGE_LOGIN_MICROSOFT, PAGE_LOGIN_OFFLINE, common},
};

/// 创建登录方式选择页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let list = column![
        common::large_list_item(
            &theme,
            icons::LOGIN_OFFLINE,
            "离线登录",
            "适用于没有正版账户的玩家",
            common::command(crate::app::PUSH_PAGE, PAGE_LOGIN_OFFLINE),
        ),
        common::large_list_item(
            &theme,
            icons::LOGIN_MICROSOFT,
            "Microsoft 正版登录",
            "适用于 Minecraft 正版玩家",
            common::command(crate::app::PUSH_PAGE, PAGE_LOGIN_MICROSOFT),
        ),
        common::large_list_item(
            &theme,
            icons::LOGIN_AUTHLIB,
            "Authlib Injector 第三方登录",
            "适用于使用皮肤站进行登录的玩家",
            common::command(crate::app::PUSH_PAGE, PAGE_LOGIN_AUTHLIB),
        ),
    ]
    .spacing(0);

    container(common::scroll(&theme, list))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
