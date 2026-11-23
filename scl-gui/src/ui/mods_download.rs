//! 模组下载来源选择页面。

use iced::widget::{column, container};
use iced::{Element, Length};
use scl_gui_widgets::icons;

use crate::{
    app::Message,
    app_state::AppState,
    ui::{PAGE_MODS_DOWNLOAD_CURSEFORGE, PAGE_MODS_DOWNLOAD_MODRINTH, common},
};

/// 创建模组下载来源选择页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let list = column![
        container(common::label(&theme, "模组下载")).padding(10),
        common::large_list_item(
            &theme,
            icons::CURSEFORGE,
            "CurseForge（不稳定）",
            "老牌的 Minecraft 模组分享平台",
            common::command(crate::app::PUSH_PAGE, PAGE_MODS_DOWNLOAD_CURSEFORGE),
        ),
        common::large_list_item(
            &theme,
            icons::MODRINTH,
            "Modrinth",
            "新兴的开源 Minecraft 模组分享平台",
            common::command(crate::app::PUSH_PAGE, PAGE_MODS_DOWNLOAD_MODRINTH),
        ),
    ];

    container(list)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
