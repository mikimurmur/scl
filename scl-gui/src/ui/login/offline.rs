//! 离线登录页面。

use druid::{
    widget::{CrossAxisAlignment, Flex},
    Target, Widget, WidgetExt,
};
use scl_gui_widgets::{
    theme::color::{base, typography},
    widgets::{label, Button, PressKey},
};

use crate::{app::LOGIN_OFFLINE, app_state::AppState, ui::common};

fn on_enter(ctx: &mut druid::EventCtx, _data: &mut AppState, _env: &druid::Env) {
    ctx.submit_command(LOGIN_OFFLINE.with(()).to(Target::Global));
}

/// 创建离线登录页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let title = label::new("创建离线账户")
        .with_font(typography::SUBTITLE)
        .with_text_size(20.);

    let help = label::new(
        "如果你是通过其它启动器迁移到 SCL 的，你可能需要修改此处的标识来保证你存档物品的兼容\n\n\
         如果留空，则会根据用户名自动生成一个离线唯一标识（Bukkit 方式）",
    )
    .with_text_size(11.)
    .with_text_color(base::MEDIUM);

    let user_name = common::field_label("用户名");
    let uuid = common::field_label("用户统一标识（UUID）");

    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(title)
        .with_default_spacer()
        .with_child(user_name)
        .with_default_spacer()
        .with_child(
            druid::widget::TextBox::new()
                .lens(AppState::login_offline_user_name)
                .controller(PressKey::new(druid::Code::Enter, on_enter)),
        )
        .with_default_spacer()
        .with_child(uuid)
        .with_default_spacer()
        .with_child(help)
        .with_default_spacer()
        .with_child(
            druid::widget::TextBox::new()
                .lens(AppState::login_offline_uuid)
                .controller(PressKey::new(druid::Code::Enter, on_enter)),
        )
        .with_flex_spacer(1.)
        .with_child(
            Button::new("创建")
                .with_accent(true)
                .on_click(|ctx, _data, _env| {
                    ctx.submit_command(LOGIN_OFFLINE.with(()).to(Target::Global));
                }),
        )
        .expand()
        .padding(10.);

    Box::new(content)
}
