//! Authlib Injector 第三方登录页面。

use iced::widget::{column, container, space};
use iced::{Element, Length};
use scl_gui_widgets::widgets::ButtonKind;

use crate::{
    app::{LOGIN_AUTHLIB, Message},
    app_state::AppState,
    ui::{TextField, UiMessage, common},
};

/// 创建 Authlib 登录页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let login = common::command(LOGIN_AUTHLIB, ());

    let disabled = state.login_busy
        || state.login_authlib_server_url.is_empty()
        || state.login_authlib_username.is_empty()
        || state.login_authlib_password.is_empty();

    let mut content = column![
        common::title(&theme, "Authlib Injector 登录"),
        space().height(Length::Fixed(8.0)),
        common::hint(&theme, "注：暂不支持拖拽输入验证服务器地址"),
    ]
    .spacing(4);

    if let Some(error) = &state.login_error {
        content = content.push(
            iced::widget::text(error.clone())
                .size(theme.typography.body.size)
                .font(theme.typography.body.font)
                .color(theme.main.primary),
        );
    }

    content = content
        .push(space().height(Length::Fixed(8.0)))
        .push(common::field_label(&theme, "验证服务器地址"))
        .push(common::text_input_submit(
            &theme,
            "",
            &state.login_authlib_server_url,
            move |value| Message::Ui(UiMessage::Text(TextField::AuthlibServer, value)),
            login.clone(),
        ))
        .push(space().height(Length::Fixed(8.0)))
        .push(common::field_label(&theme, "账户名 / 角色名 / 邮箱"))
        .push(common::text_input_submit(
            &theme,
            "",
            &state.login_authlib_username,
            move |value| Message::Ui(UiMessage::Text(TextField::AuthlibUser, value)),
            login.clone(),
        ))
        .push(space().height(Length::Fixed(8.0)))
        .push(common::field_label(&theme, "密码"))
        .push(common::password_input(
            &theme,
            "",
            &state.login_authlib_password,
            move |value| Message::Ui(UiMessage::Text(TextField::AuthlibPassword, value)),
            login.clone(),
        ))
        .push(space().height(Length::Fill))
        .push(common::button_maybe(
            &theme,
            "登录",
            ButtonKind::Accent,
            (!disabled).then_some(login),
        ));

    container(content.padding(10))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
