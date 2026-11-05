//! 登录方式选择页面。

use druid::{
    widget::{CrossAxisAlignment, Flex},
    Target, Widget,
};
use scl_gui_widgets::widgets::LargeListItem;

use crate::{
    app_state::AppState,
    ui::{PAGE_LOGIN_AUTHLIB, PAGE_LOGIN_MICROSOFT, PAGE_LOGIN_OFFLINE, PUSH_PAGE},
};

fn login_item(
    icon: scl_gui_widgets::theme::icons::IconKeyPair,
    title: &'static str,
    desc: &'static str,
    page: &'static str,
) -> impl Widget<AppState> {
    LargeListItem::new(icon, title, desc).on_click(move |ctx, _data, _env| {
        ctx.submit_command(PUSH_PAGE.with(page).to(Target::Global));
    })
}

/// 创建登录方式选择页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(login_item(
                crate::theme::icons::LOGIN_OFFLINE,
                "离线登录",
                "适用于没有正版账户的玩家",
                PAGE_LOGIN_OFFLINE,
            ))
            .with_child(login_item(
                crate::theme::icons::LOGIN_MICROSOFT,
                "Microsoft 正版登录",
                "适用于 Minecraft 正版玩家",
                PAGE_LOGIN_MICROSOFT,
            ))
            .with_child(login_item(
                crate::theme::icons::LOGIN_AUTHLIB,
                "Authlib Injector 第三方登录",
                "适用于使用皮肤站进行登录的玩家",
                PAGE_LOGIN_AUTHLIB,
            )),
    )
}
