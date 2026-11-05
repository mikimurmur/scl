//! NeoForge 加载器版本选择子页。

use druid::{
    widget::{Flex, Scroll},
    LensExt, Widget, WidgetExt,
};

use crate::{
    app::NEOFORGE_DOWNLOAD_SELECT,
    app_state::{AppState, DownloadState},
    theme::icons::GAME_NEOFORGE,
    ui::download::{
        loader_folder, loader_latest_row, no_install_row, LoaderPageData, StringLoaderPageLens,
    },
    widgets::NetworkMaybe,
};

/// 创建 NeoForge 版本选择页
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let content = Scroll::new(
        Flex::column()
            .with_child(no_install_row(GAME_NEOFORGE, NEOFORGE_DOWNLOAD_SELECT))
            .with_child(loader_latest_row(
                GAME_NEOFORGE,
                "最新稳定版本：",
                false,
                NEOFORGE_DOWNLOAD_SELECT,
            ))
            .with_child(loader_latest_row(
                GAME_NEOFORGE,
                "最新测试版本：",
                true,
                NEOFORGE_DOWNLOAD_SELECT,
            ))
            .with_child(loader_folder(
                GAME_NEOFORGE,
                "所有稳定版本",
                NEOFORGE_DOWNLOAD_SELECT,
                LoaderPageData::<String>::state.then(DownloadState::<String>::avaliable_versions),
            ))
            .with_child(loader_folder(
                GAME_NEOFORGE,
                "所有测试版本",
                NEOFORGE_DOWNLOAD_SELECT,
                LoaderPageData::<String>::state
                    .then(DownloadState::<String>::avaliable_snapshot_versions),
            )),
    )
    .vertical()
    .expand();

    Box::new(NetworkMaybe::new(content).lens(StringLoaderPageLens::new("neoforge")))
}
