//! 模组下载来源选择页面。

use druid::{
    widget::{CrossAxisAlignment, Flex},
    Target, Widget, WidgetExt,
};
use scl_gui_widgets::widgets::{label, LargeListItem};

use crate::{
    app_state::AppState,
    ui::{PAGE_MODS_DOWNLOAD_CURSEFORGE, PAGE_MODS_DOWNLOAD_MODRINTH, PUSH_PAGE},
};

fn source_item(
    icon: scl_gui_widgets::theme::icons::IconKeyPair,
    title: &'static str,
    desc: &'static str,
    page: &'static str,
) -> impl Widget<AppState> {
    LargeListItem::new(icon, title, desc).on_click(move |ctx, _data, _env| {
        ctx.submit_command(PUSH_PAGE.with(page).to(Target::Global));
    })
}

/// 创建模组下载来源选择页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(label::new("模组下载").padding(10.))
            .with_child(source_item(
                crate::theme::icons::CURSEFORGE,
                "CurseForge（不稳定）",
                "老牌的 Minecraft 模组分享平台",
                PAGE_MODS_DOWNLOAD_CURSEFORGE,
            ))
            .with_child(source_item(
                crate::theme::icons::MODRINTH,
                "Modrinth",
                "新兴的开源 Minecraft 模组分享平台",
                PAGE_MODS_DOWNLOAD_MODRINTH,
            )),
    )
}
