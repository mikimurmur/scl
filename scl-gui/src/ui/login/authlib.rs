//! Authlib Injector 第三方登录页面。

use druid::{widget::LineBreaking, Target, Widget, WidgetExt};
use scl_gui_widgets::{
    theme::color::{main, typography},
    widgets::{label, Button, PasswordBox, PressKey},
    WidgetExt as _,
};

use crate::{app::LOGIN_AUTHLIB, app_state::AppState, ui::common};

fn on_enter(ctx: &mut druid::EventCtx, _data: &mut AppState, _env: &druid::Env) {
    ctx.submit_command(LOGIN_AUTHLIB.with(()).to(Target::Global));
}

/// 把后台错误整理成单行内联提示。
///
/// 登录失败的错误文本可能是多行的 `Debug` 输出，直接交给可换行的 label
/// 会把整页撑高并把底部「登录」按钮挤出窗口（问题 ④）。这里先折叠全部
/// 空白与换行，再按字符数截断，保证内联提示始终只占一行。
fn inline_error_text(error: &str) -> String {
    /// 内联提示允许展示的最大字符数（超出部分用省略号代替）
    const MAX_CHARS: usize = 96;
    let collapsed = error.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= MAX_CHARS {
        collapsed
    } else {
        let mut text: String = collapsed.chars().take(MAX_CHARS).collect();
        text.push('…');
        text
    }
}

/// 创建 Authlib 登录页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let title = label::new("Authlib Injector 登录")
        .with_font(typography::SUBTITLE)
        .with_text_size(20.);

    let note = label::new("注：由于 UI 框架限制，暂不支持拖拽输入验证服务器地址")
        .show_if(|data: &AppState, _: &druid::Env| data.login_error.is_none());

    let error = label::dynamic(|data: &AppState, _: &druid::Env| {
        inline_error_text(&data.login_error.clone().unwrap_or_default())
    })
    .with_text_color(main::PRIMARY)
    // 单行裁剪：错误文本再长也不会改变行数，布局与正常态保持一致。
    .with_line_break_mode(LineBreaking::Clip)
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

#[cfg(test)]
mod tests {
    use super::inline_error_text;

    #[test]
    fn collapses_multiline_error_into_single_line() {
        let error = "登录失败：\nError(ForbiddenOperationException(\n    \"用户名或密码错误。Incorrect username or password.\"\n))";
        assert_eq!(
            inline_error_text(error),
            "登录失败： Error(ForbiddenOperationException( \"用户名或密码错误。Incorrect username or password.\" ))"
        );
        assert!(!inline_error_text(error).contains('\n'));
    }

    #[test]
    fn truncates_long_error_on_char_boundary() {
        // 用中文构造超长错误，确认按字符（而不是字节）截断且不越界。
        let error = "错".repeat(200);
        let text = inline_error_text(&error);
        assert_eq!(text.chars().count(), 97, "96 个字符 + 省略号");
        assert!(text.ends_with('…'));
        // 截断后的内容应当全部是合法 UTF-8 中文字符
        assert_eq!(text.trim_end_matches('…'), "错".repeat(96));
    }

    #[test]
    fn keeps_short_error_unchanged() {
        assert_eq!(
            inline_error_text("请填写服务器地址、用户名和密码"),
            "请填写服务器地址、用户名和密码"
        );
        assert_eq!(inline_error_text(""), "");
    }
}
