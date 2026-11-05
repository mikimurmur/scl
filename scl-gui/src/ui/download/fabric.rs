//! Fabric 加载器版本选择子页。

use druid::{
    widget::{Flex, Scroll},
    LensExt, Widget, WidgetExt,
};

use crate::{
    app::FABRIC_DOWNLOAD_SELECT,
    app_state::{AppState, DownloadState},
    theme::icons::GAME_FABRIC,
    ui::download::{
        loader_folder, loader_latest_row, no_install_row, LoaderPageData, StringLoaderPageLens,
    },
    widgets::NetworkMaybe,
};

/// 创建 Fabric 版本选择页
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let content = Scroll::new(
        Flex::column()
            .with_child(no_install_row(GAME_FABRIC, FABRIC_DOWNLOAD_SELECT))
            .with_child(loader_latest_row(
                GAME_FABRIC,
                "最新正式版：",
                false,
                FABRIC_DOWNLOAD_SELECT,
            ))
            .with_child(loader_latest_row(
                GAME_FABRIC,
                "最新快照版：",
                true,
                FABRIC_DOWNLOAD_SELECT,
            ))
            .with_child(loader_folder(
                GAME_FABRIC,
                "所有正式版本",
                FABRIC_DOWNLOAD_SELECT,
                LoaderPageData::<String>::state.then(DownloadState::<String>::avaliable_versions),
            ))
            .with_child(loader_folder(
                GAME_FABRIC,
                "所有快照版本",
                FABRIC_DOWNLOAD_SELECT,
                LoaderPageData::<String>::state
                    .then(DownloadState::<String>::avaliable_snapshot_versions),
            )),
    )
    .vertical()
    .expand();

    Box::new(NetworkMaybe::new(content).lens(StringLoaderPageLens::new("fabric")))
}
