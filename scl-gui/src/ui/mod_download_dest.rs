//! 模组安装目标版本选择页面。

use druid::{
    widget::{CrossAxisAlignment, Flex, List, Scroll, SizedBox},
    Env, FileDialogOptions, Lens, Target, Widget, WidgetExt,
};
use im::Vector;
use scl_gui_widgets::{
    theme::icons::IconKeyPair,
    widgets::{label, DownloadModuleItem},
};

use crate::{
    app::{INSTALL_MOD, SAVE_MOD},
    app_state::{AppState, LocalVersionInfo},
    core::VersionType,
    theme::icons::{GAME_FABRIC, GAME_FORGE, GAME_OPTIFINE, GAME_VANILLA},
    ui::{common, QUERY_POP_PAGE},
    widgets::OptionView,
};

/// 可安装模组的版本列表镜头
pub struct InstallableVersionLens;

impl Lens<AppState, Vector<LocalVersionInfo>> for InstallableVersionLens {
    fn with<V, F: FnOnce(&Vector<LocalVersionInfo>) -> V>(&self, data: &AppState, f: F) -> V {
        let versions: Vector<LocalVersionInfo> =
            crate::app::mod_download_dest::filter_versions(data)
                .into_iter()
                .collect();
        f(&versions)
    }

    fn with_mut<V, F: FnOnce(&mut Vector<LocalVersionInfo>) -> V>(
        &self,
        data: &mut AppState,
        f: F,
    ) -> V {
        let mut versions: Vector<LocalVersionInfo> =
            crate::app::mod_download_dest::filter_versions(data)
                .into_iter()
                .collect();
        f(&mut versions)
    }
}

fn install_version_icon(data: &LocalVersionInfo, _env: &Env) -> IconKeyPair {
    match data.version_type {
        VersionType::Forge => GAME_FORGE,
        VersionType::Fabric => GAME_FABRIC,
        VersionType::Optifine => GAME_OPTIFINE,
        _ => GAME_VANILLA,
    }
}

fn install_version_name(data: &LocalVersionInfo, _: &Env) -> String {
    data.name.clone()
}

fn install_version_desc(_data: &LocalVersionInfo, _: &Env) -> String {
    String::new()
}

fn install_version_row() -> impl Widget<LocalVersionInfo> {
    DownloadModuleItem::dynamic_icon(
        install_version_icon,
        install_version_name,
        install_version_desc,
    )
    .with_theme_icon(true)
    .on_click(|ctx, data, _env| {
        ctx.submit_command(INSTALL_MOD.with(data.name.clone()).to(Target::Global));
        ctx.submit_command(QUERY_POP_PAGE.with("").to(Target::Global));
    })
}

fn save_local_item() -> impl Widget<AppState> {
    DownloadModuleItem::new(crate::theme::icons::DESKTOP, "保存到本地").on_click(
        |ctx, data: &mut AppState, _env| {
            let file_name = data
                .mod_file_downloads_selected
                .as_ref()
                .or_else(|| {
                    data.mod_file_downloads_files
                        .as_ref()
                        .and_then(|files| files.front())
                })
                .map(|file| file.file_name.clone())
                .unwrap_or_default();
            let mod_name = data
                .mod_file_downloads_info
                .as_ref()
                .map(|info| info.name.clone())
                .unwrap_or_default();
            ctx.submit_command(
                druid::commands::SHOW_SAVE_PANEL.with(
                    FileDialogOptions::new()
                        .default_name(file_name)
                        .title(format!("请选择保存模组  {mod_name}的路径"))
                        .accept_command(SAVE_MOD),
                ),
            );
        },
    )
}

/// 创建模组安装目标版本选择页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    // 顶部信息行与文件页共用 AppState::mod_file_downloads_info；
    // 用稳定的 OptionView 替代 Maybe，避免 info None↔Some 切换触发 druid
    // BuildFocusChain-before-WidgetAdded 断言（见 widgets/option_view.rs）。
    let info = OptionView::new(common::mod_info(), SizedBox::empty())
        .lens(AppState::mod_file_downloads_info);

    let list = List::new(install_version_row)
        .lens(InstallableVersionLens)
        .expand_width();

    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(save_local_item())
        .with_child(list);

    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(info)
            .with_child(label::new("安装模组至：").padding(12.))
            .with_flex_child(Scroll::new(content).vertical().expand_width(), 1.0)
            .expand(),
    )
}
