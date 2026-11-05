//! Optifine 版本选择子页。

use druid::{
    widget::{Flex, Scroll},
    LensExt, Widget, WidgetExt,
};

use crate::{
    app::OPTIFINE_DOWNLOAD_SELECT,
    app_state::{AppState, DownloadState},
    theme::icons::GAME_OPTIFINE,
    ui::download::{
        loader_folder, loader_latest_row, no_install_row, LoaderPageData, StringLoaderPageLens,
    },
    widgets::NetworkMaybe,
};

/// 创建 Optifine 版本选择页
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let content = Scroll::new(
        Flex::column()
            .with_child(no_install_row(GAME_OPTIFINE, OPTIFINE_DOWNLOAD_SELECT))
            .with_child(loader_latest_row(
                GAME_OPTIFINE,
                "最新版本：",
                false,
                OPTIFINE_DOWNLOAD_SELECT,
            ))
            .with_child(loader_folder(
                GAME_OPTIFINE,
                "所有版本",
                OPTIFINE_DOWNLOAD_SELECT,
                LoaderPageData::<String>::state.then(DownloadState::<String>::avaliable_versions),
            ))
            .with_child(loader_folder(
                GAME_OPTIFINE,
                "所有预览版本",
                OPTIFINE_DOWNLOAD_SELECT,
                LoaderPageData::<String>::state
                    .then(DownloadState::<String>::avaliable_snapshot_versions),
            )),
    )
    .vertical()
    .expand();

    Box::new(NetworkMaybe::new(content).lens(StringLoaderPageLens::new("optifine")))
}
