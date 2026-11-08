//! 关于页面。

use druid::{
    widget::{CrossAxisAlignment, Flex},
    Env, EventCtx, LifeCycleCtx, Target, Widget, WidgetExt,
};
use scl_gui_widgets::{
    theme::color,
    widgets::{label, Button},
};

use crate::{
    app::{CHECK_UPDATE, OPEN_URL},
    app_state::AppState,
    ui::{settings::caption, PAGE_UPDATE_LOG},
};

fn link_button(text: &'static str, url: &'static str) -> impl Widget<AppState> {
    Button::new(text).expand_width().on_click(
        move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
            ctx.submit_command(OPEN_URL.with(url.to_owned()).to(Target::Global));
        },
    )
}

/// 触发一次启动器更新检查
fn on_check_update(ctx: &mut EventCtx, data: &mut AppState, _env: &Env) {
    data.update_available = None;
    data.update_logs = None;
    ctx.submit_command(CHECK_UPDATE.with(()).to(Target::Global));
}

fn dependency(
    name: &'static str,
    author: &'static str,
    license: &'static str,
) -> Box<dyn Widget<AppState>> {
    let mut column = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(label::new(name));

    for text in [author, license] {
        if !text.is_empty() {
            column.add_child(
                label::new(text)
                    .with_font(color::typography::CAPTION_ALT)
                    .with_text_color(color::base::MEDIUM),
            );
        }
    }

    Box::new(column)
}

/// 创建关于页面（不含外层 padding / scroll）
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let update_status =
        label::dynamic(|data: &AppState, _env: &Env| match &data.update_available {
            Some(version) if version == env!("CARGO_PKG_VERSION") => "已是最新版本".to_owned(),
            Some(version) => format!("有可用版本更新：{version}"),
            None => "检查更新中……".to_owned(),
        });

    let update_row = Flex::row()
        .with_flex_child(
            Button::new("检查版本")
                .expand_width()
                .disabled_if(|data: &AppState, _env| {
                    data.update_available.as_deref().map_or(true, str::is_empty)
                })
                .on_click(|ctx: &mut EventCtx, data: &mut AppState, env: &Env| {
                    on_check_update(ctx, data, env);
                }),
            1.0,
        )
        .with_spacer(5.)
        .with_flex_child(
            Button::new("立即更新")
                .with_accent(true)
                .expand_width()
                .disabled_if(
                    |data: &AppState, _env| match data.update_available.as_deref() {
                        None => true,
                        Some(version) => version.is_empty() || version == env!("CARGO_PKG_VERSION"),
                    },
                )
                .on_click(|ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                    ctx.submit_command(
                        crate::ui::PUSH_PAGE
                            .with(PAGE_UPDATE_LOG)
                            .to(Target::Global),
                    );
                }),
            1.0,
        );

    let links_row = |left: (&'static str, &'static str), right: (&'static str, &'static str)| {
        Flex::row()
            .with_flex_child(link_button(left.0, left.1), 1.0)
            .with_spacer(5.)
            .with_flex_child(link_button(right.0, right.1), 1.0)
    };

    let dependencies = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(dependency("surf", "By dignifiedquire", "MIT OR Apache-2.0"))
        .with_child(dependency(
            "bangbang93's minecraft launcher api",
            "By bangbang93",
            "https://bmclapidoc.bangbang93.com/",
        ))
        .with_child(dependency(
            "druid",
            "By Colin Rofls",
            "Under Apache-2.0 licence",
        ))
        .with_child(dependency("image", "", "Under MIT licence"))
        .with_child(dependency("Rust Programming Language", "By Mozilla", ""))
        .with_child(dependency(
            "serde serde_json",
            "By David Tolnay",
            "Under MIT OR Apache-2.0 licence",
        ))
        .with_child(dependency("smol", "By smol-rs", "Under Apache-2.0 OR MIT"));

    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(label::new("Sharp Craft Launcher 1.2.0"))
            .with_child(label::new("By SteveXMH"))
            .with_child(label::new(concat!("编译日期：", env!("SCL_BUILD_DATE"))))
            .with_child(label::new("操作系统：Linux"))
            .with_child(label::new("软件架构：x86_64"))
            .with_spacer(8.)
            .with_child(update_status)
            .with_spacer(8.)
            .with_child(update_row)
            .with_spacer(5.)
            .with_child(links_row(
                (
                    "启动器主页",
                    "https://steve-xmh.github.io/scl/index.html",
                ),
                ("Github 仓库", "https://github.com/Steve-xmh/scl"),
            ))
            .with_spacer(5.)
            .with_child(links_row(
                ("爱发电", "http://afdian.net/@SteveXMH"),
                ("赞助 BMCLAPI", "http://afdian.net/@bangbang93"),
            ))
            .with_spacer(8.)
            .with_child(label::new("鸣谢").with_font(color::typography::TITLE))
            .with_spacer(8.)
            .with_child(caption(
                "由于位置有限，只能展示主要使用的几个框架/库/服务，如果缺失了您的名字还请与作者私聊添加。感谢以下框架的开发者们！",
            ))
            .with_spacer(8.)
            .with_child(dependencies)
            .on_added(|_widget, ctx: &mut LifeCycleCtx, data: &AppState, _env: &Env| {
                if data.update_available.is_none() {
                    ctx.submit_command(CHECK_UPDATE.with(()).to(Target::Global));
                }
            }),
    )
}
