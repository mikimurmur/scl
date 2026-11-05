//! Microsoft 正版登录页面。

use druid::{
    widget::{Flex, ViewSwitcher},
    Env, Target, Widget, WidgetExt,
};
use scl_gui_widgets::{
    theme::color::main,
    widgets::{label, ProgressRing},
};

use crate::{app::OPEN_URL, app_state::AppState, panic::copy_to_clipboard};

/// Microsoft 登录页面的显示状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, druid::Data)]
enum PageState {
    /// 登录失败，显示错误信息
    Error,
    /// 正在获取用户代码
    Loading,
    /// 已获取用户代码，等待用户完成验证
    Code,
    /// 验证成功，正在登录
    Success,
}

fn page_state(data: &AppState, _: &Env) -> PageState {
    if data
        .login_error
        .as_ref()
        .is_some_and(|error| !error.is_empty())
    {
        PageState::Error
    } else if data.login_microsoft_code.is_empty() {
        PageState::Loading
    } else if data.login_microsoft_success {
        PageState::Success
    } else {
        PageState::Code
    }
}

/// 创建加载中 / 验证成功时显示的页面
fn progress_view(text: &'static str) -> Box<dyn Widget<AppState>> {
    Box::new(
        Flex::column()
            .with_flex_spacer(1.)
            .with_child(ProgressRing::new())
            .with_spacer(10.)
            .with_child(label::new(text))
            .with_flex_spacer(1.),
    )
}

/// 创建 Microsoft 登录页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let views = ViewSwitcher::new(
        page_state,
        |state: &PageState, _data: &AppState, _env: &Env| -> Box<dyn Widget<AppState>> {
            match state {
                PageState::Error => Box::new(
                    Flex::column()
                        .with_child(
                            label::dynamic(|data: &AppState, _: &Env| {
                                data.login_error.clone().unwrap_or_default()
                            })
                            .with_text_color(main::PRIMARY),
                        )
                        .with_flex_spacer(1.),
                ),
                PageState::Loading => progress_view("正在获取用户代码……"),
                PageState::Success => progress_view("验证成功！正在登录……"),
                PageState::Code => {
                    let code = label::dynamic(|data: &AppState, _: &Env| {
                        data.login_microsoft_code.clone()
                    })
                    .with_text_size(32.);

                    let hint = label::new(
                        "请点击下方按钮，在弹出的验证网页中输入用户代码并完成验证。\n\
                         验证期间请不要退出这个页面。",
                    );

                    let open_button = scl_gui_widgets::widgets::Button::new("打开验证网站")
                        .with_accent(true)
                        .on_click(|ctx, data: &mut AppState, _env| {
                            if !data.login_microsoft_url.is_empty() {
                                ctx.submit_command(
                                    OPEN_URL
                                        .with(data.login_microsoft_url.clone())
                                        .to(Target::Global),
                                );
                            }
                        })
                        .expand_width();

                    let copy_button = scl_gui_widgets::widgets::Button::new("复制用户代码")
                        .on_click(|_ctx, data: &mut AppState, _env| {
                            if !data.login_microsoft_code.is_empty() {
                                copy_to_clipboard(data.login_microsoft_code.clone());
                            }
                        })
                        .expand_width();

                    Box::new(
                        Flex::column()
                            .with_flex_spacer(1.)
                            .with_child(label::new("你的用户代码是"))
                            .with_spacer(10.)
                            .with_child(code)
                            .with_spacer(10.)
                            .with_child(hint)
                            .with_flex_spacer(1.)
                            .with_child(
                                Flex::row()
                                    .with_flex_child(open_button, 1.)
                                    .with_spacer(10.)
                                    .with_flex_child(copy_button, 1.),
                            ),
                    )
                }
            }
        },
    );

    Box::new(
        Flex::column()
            .with_flex_child(views, 1.)
            .expand()
            .padding(10.),
    )
}
