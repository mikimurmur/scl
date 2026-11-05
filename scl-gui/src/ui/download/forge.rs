//! Forge 加载器版本选择子页。

use druid::{
    widget::{Flex, Scroll},
    LensExt, Widget, WidgetExt,
};

use crate::{
    app::FORGE_DOWNLOAD_SELECT,
    app_state::{AppState, DownloadState},
    theme::icons::GAME_FORGE,
    ui::download::{
        loader_folder, loader_latest_row, no_install_row, LoaderPageData, StringLoaderPageLens,
    },
    widgets::NetworkMaybe,
};

/// 创建 Forge 版本选择页
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let content = Scroll::new(
        Flex::column()
            .with_child(no_install_row(GAME_FORGE, FORGE_DOWNLOAD_SELECT))
            .with_child(loader_latest_row(
                GAME_FORGE,
                "推荐版本：",
                false,
                FORGE_DOWNLOAD_SELECT,
            ))
            .with_child(loader_latest_row(
                GAME_FORGE,
                "最新版本：",
                false,
                FORGE_DOWNLOAD_SELECT,
            ))
            .with_child(loader_folder(
                GAME_FORGE,
                "所有版本",
                FORGE_DOWNLOAD_SELECT,
                LoaderPageData::<String>::state.then(DownloadState::<String>::avaliable_versions),
            )),
    )
    .vertical()
    .expand();

    Box::new(NetworkMaybe::new(content).lens(StringLoaderPageLens::new("forge")))
}
