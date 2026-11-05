//! 启动器更新页面。

use druid::{
    widget::{CrossAxisAlignment, Flex, Scroll, ViewSwitcher},
    Env, Target, Widget, WidgetExt,
};
use scl_gui_widgets::widgets::{label, Button};

use crate::{app::UPDATE_NOW, app_state::AppState};

fn loading_page() -> Box<dyn Widget<AppState>> {
    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .with_flex_spacer(1.0)
            .with_child(label::new("正在获取更新记录"))
            .with_flex_spacer(1.0),
    )
}

fn loaded_page() -> Box<dyn Widget<AppState>> {
    let header = label::dynamic(|data: &AppState, _env: &Env| {
        let version = data.update_available.as_deref().unwrap_or("最新版本");
        format!("自当前版本到  {} 的更新记录：", version)
    })
    .padding(5.);

    let logs = label::dynamic(|data: &AppState, _env: &Env| {
        data.update_logs
            .clone()
            .unwrap_or_else(|| "正在获取更新记录".to_owned())
    })
    .padding(5.);

    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(header)
        .with_flex_child(Scroll::new(logs).vertical().expand_width(), 1.0)
        .with_child(
            Button::new("立刻更新！")
                .with_accent(true)
                .expand_width()
                .on_click(|ctx: &mut druid::EventCtx, _data: &mut AppState, _env| {
                    ctx.submit_command(UPDATE_NOW.with(()).to(Target::Global));
                }),
        )
        .padding(12.);

    Box::new(content.boxed())
}

/// 创建启动器更新页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    Box::new(ViewSwitcher::new(
        |data: &AppState, _env| data.update_logs.is_some(),
        |loaded: &bool, _data: &AppState, _env| {
            if *loaded {
                loaded_page()
            } else {
                loading_page()
            }
        },
    ))
}
