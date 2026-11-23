//! 下载游戏页面。

pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod optifine;
pub mod quiltmc;
pub mod vanilla;

use iced::widget::{column, container};
use iced::{Element, Length};
use scl_gui_widgets::icons;
use scl_gui_widgets::theme::Theme;
use scl_gui_widgets::widgets::ButtonKind;

use crate::{
    app::{self, DOWNLOAD_GAME, Message},
    app_state::{AppState, DownloadState, NetworkData},
    ui::{
        PAGE_DOWNLOAD_FABRIC, PAGE_DOWNLOAD_FORGE, PAGE_DOWNLOAD_NEOFORGE, PAGE_DOWNLOAD_OPTIFINE,
        PAGE_DOWNLOAD_QUILTMC, PAGE_DOWNLOAD_VANILLA, PAGE_GAME_DIR,
        UiMessage::{SelectLoaderVersion, SelectVanillaVersion, Text},
        common,
    },
};

/// 一个加载器页面的定义。
pub(crate) struct LoaderPage {
    /// 加载器类型。
    pub kind: &'static str,
    /// 图标。
    pub icon: icons::Icon,
    /// 「不安装」上方的标签。
    pub no_install_prefix: &'static str,
    /// 「最新」行的标签。
    pub latest_labels: &'static [(&'static str, bool)],
    /// 所有版本分组的标签。
    pub folder_labels: &'static [(&'static str, bool)],
}

const FABRIC_PAGE: LoaderPage = LoaderPage {
    kind: "fabric",
    icon: icons::GAME_FABRIC,
    no_install_prefix: "",
    latest_labels: &[("最新正式版：", false), ("最新快照版：", true)],
    folder_labels: &[("所有正式版本", false), ("所有快照版本", true)],
};

const QUILTMC_PAGE: LoaderPage = LoaderPage {
    kind: "quiltmc",
    icon: icons::GAME_QUILTMC,
    no_install_prefix: "",
    latest_labels: &[("最新正式版：", false), ("最新快照版：", true)],
    folder_labels: &[("所有正式版本", false), ("所有快照版本", true)],
};

const FORGE_PAGE: LoaderPage = LoaderPage {
    kind: "forge",
    icon: icons::GAME_FORGE,
    no_install_prefix: "",
    latest_labels: &[("推荐版本：", false), ("最新版本：", false)],
    folder_labels: &[("所有版本", false)],
};

const NEOFORGE_PAGE: LoaderPage = LoaderPage {
    kind: "neoforge",
    icon: icons::GAME_NEOFORGE,
    no_install_prefix: "",
    latest_labels: &[("最新稳定版本：", false), ("最新测试版本：", true)],
    folder_labels: &[("所有稳定版本", false), ("所有测试版本", true)],
};

const OPTIFINE_PAGE: LoaderPage = LoaderPage {
    kind: "optifine",
    icon: icons::GAME_OPTIFINE,
    no_install_prefix: "",
    latest_labels: &[("最新版本：", false)],
    folder_labels: &[("所有版本", false), ("所有预览版本", true)],
};

/// Fabric 加载器页面定义。
pub(crate) const fn fabric_page() -> &'static LoaderPage {
    &FABRIC_PAGE
}

/// QuiltMC 加载器页面定义。
pub(crate) const fn quiltmc_page() -> &'static LoaderPage {
    &QUILTMC_PAGE
}

/// Forge 加载器页面定义。
pub(crate) const fn forge_page() -> &'static LoaderPage {
    &FORGE_PAGE
}

/// NeoForge 加载器页面定义。
pub(crate) const fn neoforge_page() -> &'static LoaderPage {
    &NEOFORGE_PAGE
}

/// Optifine 加载器页面定义。
pub(crate) const fn optifine_page() -> &'static LoaderPage {
    &OPTIFINE_PAGE
}

fn string_loader_state<'a>(
    data: &'a AppState,
    kind: &str,
) -> &'a NetworkData<DownloadState<String>> {
    match kind {
        "forge" => &data.download_state_forge,
        "neoforge" => &data.download_state_neoforge,
        "fabric" => &data.download_state_fabric,
        "quiltmc" => &data.download_state_quiltmc,
        "optifine" => &data.download_state_optifine,
        _ => &data.download_state_fabric,
    }
}

fn latest(versions: &im::Vector<String>, snapshot: bool) -> Option<String> {
    if snapshot {
        versions.front().cloned()
    } else {
        versions.front().cloned()
    }
}

fn version_rows<'a>(
    data: &AppState,
    page: &'static LoaderPage,
    theme: &Theme,
    snapshot: bool,
    versions: &im::Vector<String>,
) -> Vec<Element<'a, Message>> {
    let selected = data.loader_version(page.kind).to_owned();
    let mut rows = Vec::new();

    for version in versions.iter() {
        rows.push(common::select_row(
            theme,
            page.icon,
            version.clone(),
            *version == selected,
            Message::Ui(SelectLoaderVersion {
                kind: page.kind,
                version: version.clone(),
            }),
        ));
    }

    let _ = snapshot;
    rows
}

fn loader_content<'a>(
    data: &AppState,
    page: &'static LoaderPage,
    theme: &Theme,
) -> Element<'a, Message> {
    let network = string_loader_state(data, page.kind);
    let Some(state) = network.get() else {
        if let Some(error) = &network.error {
            return common::error_state(theme, format!("加载失败：{error}"));
        }
        return common::loading(theme, data.animation_phase, "正在加载……");
    };

    let selected = data.loader_version(page.kind).to_owned();
    let mut content = column![].spacing(0);

    content = content.push(common::select_row(
        theme,
        page.icon,
        format!("{}不安装", page.no_install_prefix),
        selected.is_empty(),
        Message::Ui(SelectLoaderVersion {
            kind: page.kind,
            version: String::new(),
        }),
    ));

    for (label, snapshot) in page.latest_labels {
        let versions = if *snapshot {
            &state.avaliable_snapshot_versions
        } else {
            &state.avaliable_versions
        };
        let latest = latest(versions, *snapshot);
        let text = format!("{label}{}", latest.as_deref().unwrap_or("无"));
        let is_selected = latest.is_some() && latest.as_deref() == Some(selected.as_str());
        let message = match latest {
            Some(version) => Message::Ui(SelectLoaderVersion {
                kind: page.kind,
                version,
            }),
            None => Message::Ui(SelectLoaderVersion {
                kind: page.kind,
                version: String::new(),
            }),
        };
        content = content.push(common::select_row(
            theme,
            page.icon,
            text,
            is_selected,
            message,
        ));
    }

    for (label, snapshot) in page.folder_labels {
        let versions = if *snapshot {
            &state.avaliable_snapshot_versions
        } else {
            &state.avaliable_versions
        };
        let rows = version_rows(data, page, theme, *snapshot, versions);
        let key = format!("{}:{label}", page.kind);
        content = content.push(common::folder(
            theme,
            &key,
            label,
            data.collapsed_folders.contains(&key),
            rows,
        ));
    }

    common::scroll(theme, content)
}

/// 一个字符串加载器页面的视图。
pub(crate) fn string_loader_view<'a>(
    state: &'a AppState,
    page: &'static LoaderPage,
) -> Element<'a, Message> {
    loader_content(state, page, &state.theme)
}

/// 原版版本页的视图。
pub(crate) fn vanilla_view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let Some(download_state) = state.download_state_vanilla.get() else {
        if let Some(error) = &state.download_state_vanilla.error {
            return common::error_state(&theme, format!("加载失败：{error}"));
        }
        return common::loading(&theme, state.animation_phase, "正在加载……");
    };

    let selected = state
        .selected_download_version
        .as_ref()
        .map(|version| version.name.clone())
        .unwrap_or_default();

    let mut content = column![].spacing(0);

    for (label, snapshot) in [("最新正式版：", false), ("最新快照版：", true)] {
        let versions = if snapshot {
            &download_state.avaliable_snapshot_versions
        } else {
            &download_state.avaliable_versions
        };
        let latest = versions.front();
        let text = format!(
            "{label}{}",
            latest.map(|version| version.name.as_str()).unwrap_or("无")
        );
        let message = match latest {
            Some(version) => Message::Ui(SelectVanillaVersion(version.name.clone())),
            None => Message::Ui(SelectVanillaVersion(String::new())),
        };
        content = content.push(common::select_row(
            &theme,
            icons::GAME_VANILLA,
            text,
            latest.is_some_and(|version| version.name == selected),
            message,
        ));
    }

    for (label, versions) in [
        ("所有正式版本", &download_state.avaliable_versions),
        ("所有快照版本", &download_state.avaliable_snapshot_versions),
        (
            "所有旧版 Beta 版本",
            &download_state.avaliable_old_beta_versions,
        ),
        (
            "所有旧版 Alpha 版本",
            &download_state.avaliable_old_alpha_versions,
        ),
    ] {
        let rows = versions
            .iter()
            .map(|version| {
                common::select_row(
                    &theme,
                    icons::GAME_VANILLA,
                    version.name.clone(),
                    version.name == selected,
                    Message::Ui(SelectVanillaVersion(version.name.clone())),
                )
            })
            .collect();
        let key = format!("vanilla:{label}");
        content = content.push(common::folder(
            &theme,
            &key,
            label,
            state.collapsed_folders.contains(&key),
            rows,
        ));
    }

    common::scroll(&theme, content)
}

fn vanilla_status(data: &AppState) -> String {
    data.selected_download_version
        .as_ref()
        .map(|version| version.name.clone())
        .unwrap_or_else(|| "加载中".to_owned())
}

fn loader_status(
    data: &AppState,
    state: &NetworkData<DownloadState<String>>,
    selected: &str,
) -> String {
    if data.selected_download_version.is_none() {
        return "不可用".to_owned();
    }
    if state.get().is_none() {
        if state.error.is_some() {
            "请求失败".to_owned()
        } else {
            "查询中".to_owned()
        }
    } else if !selected.is_empty() {
        selected.to_owned()
    } else if state
        .get()
        .map(|state| state.all_versions().next().is_none())
        .unwrap_or(false)
    {
        "无可用版本".to_owned()
    } else {
        "可安装".to_owned()
    }
}

fn install_dir_status(data: &AppState) -> String {
    match &data.config.minecraft_folder {
        crate::config::MinecraftFolder::CurrentDirectory => "当前游戏文件夹".to_owned(),
        crate::config::MinecraftFolder::OfficalDirectory => "官方游戏文件夹".to_owned(),
        crate::config::MinecraftFolder::Other(path) => path.clone(),
    }
}

/// 生成安装请求；没有选中原版版本时返回 `None`。
pub(crate) fn make_request(data: &AppState) -> Option<app::DownloadRequest> {
    let vanilla = data.selected_download_version.clone()?;
    let mut request = app::DownloadRequest::vanilla(vanilla.clone());
    request.forge = data.download_forge_version.clone();
    request.neoforge = data.download_neoforge_version.clone();
    request.fabric = data.download_fabric_version.clone();
    request.quiltmc = data.download_quiltmc_version.clone();
    request.optifine = data.download_optifine_version.clone();
    request.version_name = if data.download_version_name.trim().is_empty() {
        app::download::format_version_name(
            &vanilla.name,
            &request.fabric,
            &request.quiltmc,
            &request.forge,
            &request.neoforge,
            &request.optifine,
        )
    } else {
        data.download_version_name.trim().to_owned()
    };
    Some(request)
}

/// 创建下载游戏根页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let version_name = common::text_input(&theme, "", &state.download_version_name, move |value| {
        Message::Ui(Text(crate::ui::TextField::DownloadVersionName, value))
    });

    let mut loader_rows = column![].spacing(0);
    let rows = [
        (
            icons::GAME_VANILLA,
            "Minecraft",
            vanilla_status(state),
            PAGE_DOWNLOAD_VANILLA,
        ),
        (
            icons::GAME_FORGE,
            "Forge",
            loader_status(
                state,
                &state.download_state_forge,
                &state.download_forge_version,
            ),
            PAGE_DOWNLOAD_FORGE,
        ),
        (
            icons::GAME_NEOFORGE,
            "NeoForge",
            loader_status(
                state,
                &state.download_state_neoforge,
                &state.download_neoforge_version,
            ),
            PAGE_DOWNLOAD_NEOFORGE,
        ),
        (
            icons::GAME_FABRIC,
            "Fabric",
            loader_status(
                state,
                &state.download_state_fabric,
                &state.download_fabric_version,
            ),
            PAGE_DOWNLOAD_FABRIC,
        ),
        (
            icons::GAME_QUILTMC,
            "QuiltMC",
            loader_status(
                state,
                &state.download_state_quiltmc,
                &state.download_quiltmc_version,
            ),
            PAGE_DOWNLOAD_QUILTMC,
        ),
        (
            icons::GAME_OPTIFINE,
            "Optifine",
            loader_status(
                state,
                &state.download_state_optifine,
                &state.download_optifine_version,
            ),
            PAGE_DOWNLOAD_OPTIFINE,
        ),
    ];

    for (icon, title, desc, page) in rows {
        loader_rows = loader_rows.push(common::module_row(
            &theme,
            icon,
            title,
            desc,
            common::command(crate::app::PUSH_PAGE, page),
        ));
    }

    let install_dir_row = common::module_row(
        &theme,
        icons::FOLDER,
        "安装目录",
        install_dir_status(state),
        common::command(crate::app::PUSH_PAGE, PAGE_GAME_DIR),
    );

    let install_button = match make_request(state) {
        Some(request) => scl_gui_widgets::widgets::fill_button(
            &theme,
            "安装游戏",
            ButtonKind::Accent,
            common::command(DOWNLOAD_GAME, request),
        ),
        None => scl_gui_widgets::widgets::fill_button_maybe(
            &theme,
            "安装游戏",
            ButtonKind::Accent,
            None,
        ),
    };

    let install_button = container(install_button).width(Length::Fill).padding(10);

    column![
        container(common::label(&theme, "游戏版本名称")).padding(10),
        container(version_name).padding([0, 10]),
        container(common::label(&theme, "游戏组件")).padding(10),
        container(common::scroll(
            &theme,
            container(loader_rows).padding([0, 10]),
        ))
        .height(Length::Fill),
        container(install_dir_row).padding([0, 10]),
        install_button,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::latest;

    #[test]
    fn latest_returns_none_for_empty_list() {
        assert!(latest(&im::Vector::new(), false).is_none());
    }
}
