//! Authlib Injector 第三方登录页面。

use druid::{Target, Widget, WidgetExt};
use scl_gui_widgets::{
    theme::color::{main, typography},
    widgets::{label, Button, PasswordBox, PressKey},
    WidgetExt as _,
};

use crate::{app::LOGIN_AUTHLIB, app_state::AppState, ui::common};

fn on_enter(ctx: &mut druid::EventCtx, _data: &mut AppState, _env: &druid::Env) {
    ctx.submit_command(LOGIN_AUTHLIB.with(()).to(Target::Global));
}

/// 创建 Authlib 登录页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let title = label::new("Authlib Injector 登录")
        .with_font(typography::SUBTITLE)
        .with_text_size(20.);

    let note = label::new("注：由于 UI 框架限制，暂不支持拖拽输入验证服务器地址")
        .show_if(|data: &AppState, _: &druid::Env| data.login_error.is_none());

    let error = label::dynamic(|data: &AppState, _: &druid::Env| {
        data.login_error.clone().unwrap_or_default()
    })
    .with_text_color(main::PRIMARY)
    .show_if(|data: &AppState, _: &druid::Env| data.login_error.is_some());

    let server_label = common::field_label("验证服务器地址");
    let user_label = common::field_label("账户名 / 角色名 / 邮箱");
    let password_label = common::field_label("密码");

    let login = Button::new("登录")
        .with_accent(true)
        .on_click(|ctx, _data, _env| {
            ctx.submit_command(LOGIN_AUTHLIB.with(()).to(Target::Global));
        })
        .disabled_if(|data: &AppState, _: &druid::Env| {
            data.login_busy
                || data.login_authlib_server_url.is_empty()
                || data.login_authlib_username.is_empty()
                || data.login_authlib_password.is_empty()
        });

    Box::new(
        druid::widget::Flex::column()
            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Fill)
            .with_child(title)
            .with_default_spacer()
            .with_child(note)
            .with_child(error)
            .with_default_spacer()
            .with_child(server_label)
            .with_default_spacer()
            .with_child(
                druid::widget::TextBox::new()
                    .lens(AppState::login_authlib_server_url)
                    .controller(PressKey::new(druid::Code::Enter, on_enter)),
            )
            .with_default_spacer()
            .with_child(user_label)
            .with_default_spacer()
            .with_child(
                druid::widget::TextBox::new()
                    .lens(AppState::login_authlib_username)
                    .controller(PressKey::new(druid::Code::Enter, on_enter)),
            )
            .with_default_spacer()
            .with_child(password_label)
            .with_default_spacer()
            .with_child(
                PasswordBox::new()
                    .lens(AppState::login_authlib_password)
                    .controller(PressKey::new(druid::Code::Enter, on_enter)),
            )
            .with_flex_spacer(1.)
            .with_child(login)
            .expand()
            .padding(10.),
    )
}
