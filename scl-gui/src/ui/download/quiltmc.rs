//! QuiltMC 加载器版本选择子页。

use druid::{
    widget::{Flex, Scroll},
    LensExt, Widget, WidgetExt,
};

use crate::{
    app::QUILTMC_DOWNLOAD_SELECT,
    app_state::{AppState, DownloadState},
    theme::icons::GAME_QUILTMC,
    ui::download::{
        loader_folder, loader_latest_row, no_install_row, LoaderPageData, StringLoaderPageLens,
    },
    widgets::NetworkMaybe,
};

/// 创建 QuiltMC 版本选择页
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let content = Scroll::new(
        Flex::column()
            .with_child(no_install_row(GAME_QUILTMC, QUILTMC_DOWNLOAD_SELECT))
            .with_child(loader_latest_row(
                GAME_QUILTMC,
                "最新正式版：",
                false,
                QUILTMC_DOWNLOAD_SELECT,
            ))
            .with_child(loader_latest_row(
                GAME_QUILTMC,
                "最新快照版：",
                true,
                QUILTMC_DOWNLOAD_SELECT,
            ))
            .with_child(loader_folder(
                GAME_QUILTMC,
                "所有正式版本",
                QUILTMC_DOWNLOAD_SELECT,
                LoaderPageData::<String>::state.then(DownloadState::<String>::avaliable_versions),
            ))
            .with_child(loader_folder(
                GAME_QUILTMC,
                "所有快照版本",
                QUILTMC_DOWNLOAD_SELECT,
                LoaderPageData::<String>::state
                    .then(DownloadState::<String>::avaliable_snapshot_versions),
            )),
    )
    .vertical()
    .expand();

    Box::new(NetworkMaybe::new(content).lens(StringLoaderPageLens::new("quiltmc")))
}
