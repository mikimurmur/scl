//! 模组安装目标版本选择页面。

use iced::widget::{column, container};
use iced::{Element, Length};
use scl_gui_widgets::icons;

use crate::{
    app::Message,
    app_state::AppState,
    core::VersionType,
    ui::{UiMessage, common},
};

fn version_icon(version_type: VersionType) -> scl_gui_widgets::icons::Icon {
    match version_type {
        VersionType::Forge => icons::GAME_FORGE,
        VersionType::NeoForge => icons::GAME_NEOFORGE,
        VersionType::Fabric => icons::GAME_FABRIC,
        VersionType::QuiltMC => icons::GAME_QUILTMC,
        VersionType::Optifine => icons::GAME_OPTIFINE,
        VersionType::Vanilla | VersionType::Unknown => icons::GAME_VANILLA,
    }
}

/// 创建模组安装目标版本选择页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let info = match &state.mod_file_downloads_info {
        Some(info) => common::mod_info_row(&theme, info, None),
        None => iced::widget::space().into(),
    };

    let save_local = common::module_row(&theme, icons::DESKTOP, "保存到本地", "", Message::SaveMod);

    let mut list = column![].spacing(0);
    for version in crate::app::mod_download_dest::filter_versions(state) {
        list = list.push(common::list_item(
            &theme,
            Some(version_icon(version.version_type)),
            version.name.clone(),
            None,
            version.name == state.mod_download_install_version,
            Some(Message::Ui(UiMessage::InstallMod(version.name))),
        ));
    }

    let content = common::scroll(&theme, list);

    column![
        info,
        save_local,
        container(common::label(&theme, "安装模组至：")).padding(12),
        container(content).height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
