//! 原版游戏版本选择子页。

use druid::{
    widget::{Flex, Scroll},
    LensExt, Widget, WidgetExt,
};

use crate::{
    app_state::{AppState, DownloadState},
    core::VersionInfo,
    ui::download::{vanilla_folder, vanilla_latest_row, LoaderPageData, VanillaLoaderPageLens},
    widgets::NetworkMaybe,
};

/// 创建原版版本选择页
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let content = Scroll::new(
        Flex::column()
            .with_child(vanilla_latest_row("最新正式版：", false))
            .with_child(vanilla_latest_row("最新快照版：", true))
            .with_child(vanilla_folder(
                "所有正式版本",
                LoaderPageData::<VersionInfo>::state
                    .then(DownloadState::<VersionInfo>::avaliable_versions),
            ))
            .with_child(vanilla_folder(
                "所有快照版本",
                LoaderPageData::<VersionInfo>::state
                    .then(DownloadState::<VersionInfo>::avaliable_snapshot_versions),
            ))
            .with_child(vanilla_folder(
                "所有旧版 Beta 版本",
                LoaderPageData::<VersionInfo>::state
                    .then(DownloadState::<VersionInfo>::avaliable_old_beta_versions),
            ))
            .with_child(vanilla_folder(
                "所有旧版 Alpha 版本",
                LoaderPageData::<VersionInfo>::state
                    .then(DownloadState::<VersionInfo>::avaliable_old_alpha_versions),
            )),
    )
    .vertical()
    .expand();

    Box::new(NetworkMaybe::new(content).lens(VanillaLoaderPageLens))
}
