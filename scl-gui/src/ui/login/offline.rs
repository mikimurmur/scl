//! 离线登录页面。

use iced::widget::{column, container, space};
use iced::{Element, Length};

use crate::{
    app::{LOGIN_OFFLINE, Message},
    app_state::AppState,
    ui::{TextField, UiMessage, common},
};

/// 创建离线登录页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let login = common::command(LOGIN_OFFLINE, ());

    let content = column![
        common::title(&theme, "创建离线账户"),
        space().height(Length::Fixed(8.0)),
        common::field_label(&theme, "用户名"),
        common::text_input_submit(
            &theme,
            "",
            &state.login_offline_user_name,
            move |value| Message::Ui(UiMessage::Text(TextField::OfflineName, value)),
            login.clone(),
        ),
        space().height(Length::Fixed(8.0)),
        common::field_label(&theme, "用户统一标识（UUID）"),
        common::hint(
            &theme,
            "如果你是通过其它启动器迁移到 SCL 的，你可能需要修改此处的标识来保证你存档物品的兼容\n\n\
             如果留空，则会根据用户名自动生成一个离线唯一标识（Bukkit 方式）"
        ),
        space().height(Length::Fixed(8.0)),
        common::text_input_submit(
            &theme,
            "",
            &state.login_offline_uuid,
            move |value| Message::Ui(UiMessage::Text(TextField::OfflineUuid, value)),
            login.clone(),
        ),
        space().height(Length::Fill),
        common::accent_button(&theme, "创建", login),
    ]
    .spacing(4)
    .padding(10)
    .width(Length::Fill)
    .height(Length::Fill);

    container(content).into()
}
