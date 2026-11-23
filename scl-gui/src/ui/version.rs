//! 已安装版本列表页面。

use iced::widget::{column, container, row, space};
use iced::{Alignment, Element, Length};
use im::Vector;
use scl_gui_widgets::icons::{self, Icon};
use scl_gui_widgets::theme::Theme;
use scl_gui_widgets::widgets::ButtonKind;

use crate::{
    app::{Message, SELECT_VERSION, SELECT_VERSION_SETTING},
    app_state::{AppState, LocalVersionInfo},
    core::VersionType,
    ui::{
        PAGE_DOWNLOAD, PAGE_GAME_DIR,
        UiMessage::{Text, ToggleSort},
        common,
    },
};

/// 根据版本类型返回列表中显示的图标
pub fn version_icon(version_type: VersionType) -> Icon {
    match version_type {
        VersionType::Vanilla | VersionType::Unknown => icons::GAME_VANILLA,
        VersionType::Forge => icons::GAME_FORGE,
        VersionType::NeoForge => icons::GAME_NEOFORGE,
        VersionType::Fabric => icons::GAME_FABRIC,
        VersionType::QuiltMC => icons::GAME_QUILTMC,
        VersionType::Optifine => icons::GAME_OPTIFINE,
    }
}

fn sort_icon(mode: u8) -> Icon {
    match mode {
        1 => icons::SORT_BY_VERSION_RELEASE_DATE,
        2 => icons::SORT_BY_DOWNLOAD_DATE,
        3 => icons::CATEGORIED_BY_VERSION_TYPE,
        _ => icons::SORT_BY_NAME,
    }
}

/// 版本类型在「按版本类型分类」排序中的权重。
fn version_type_rank(version_type: VersionType) -> u8 {
    match version_type {
        VersionType::Vanilla | VersionType::Unknown => 0,
        VersionType::Forge => 1,
        VersionType::Fabric => 2,
        VersionType::Optifine => 3,
        VersionType::QuiltMC => 4,
        VersionType::NeoForge => 5,
    }
}

fn sort_with_options(data: &AppState) -> Vector<LocalVersionInfo> {
    let filter = data.versions_search_filter.as_str();
    let versions: Vector<LocalVersionInfo> = data
        .versions
        .iter()
        .filter(|version| filter.is_empty() || version.name.contains(filter))
        .cloned()
        .collect();
    sort_versions(versions, data.versions_sort_mode)
}

fn sort_versions(mut versions: Vector<LocalVersionInfo>, mode: u8) -> Vector<LocalVersionInfo> {
    match mode {
        0 | 1 => versions.sort_by(|a, b| a.name.cmp(&b.name)),
        2 => versions.sort_by(|a, b| {
            b.created_date
                .cmp(&a.created_date)
                .then_with(|| a.name.cmp(&b.name))
        }),
        3 => versions.sort_by(|a, b| {
            version_type_rank(a.version_type)
                .cmp(&version_type_rank(b.version_type))
                .then_with(|| a.name.cmp(&b.name))
        }),
        _ => versions.sort_by(|a, b| a.name.cmp(&b.name)),
    }
    versions
}

fn empty_state<'a>(theme: &Theme) -> Element<'a, Message> {
    container(
        column![
            iced::widget::text(
                "没有找到任何版本，请检查当前目录是否存在游戏\n如果确实没有下载游戏的话，要不要现在来下载一个呢？"
            )
            .size(theme.typography.caption.size)
            .font(theme.typography.caption.font)
            .color(theme.font_color),
            space().height(Length::Fixed(24.0)),
            common::accent_button(
                theme,
                "带我下载游戏吧！",
                common::command(crate::app::PUSH_PAGE, PAGE_DOWNLOAD),
            ),
            common::button(
                theme,
                "切换游戏目录",
                ButtonKind::Standard,
                common::command(crate::app::PUSH_PAGE, PAGE_GAME_DIR),
            ),
        ]
        .spacing(8)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .padding(12)
    .into()
}

fn version_list_page<'a>(state: &AppState, theme: &Theme) -> Element<'a, Message> {
    let sort_button = scl_gui_widgets::widgets::icon_button(
        theme,
        sort_icon(state.versions_sort_mode),
        18.0,
        ButtonKind::Accent,
        Message::Ui(ToggleSort),
    );

    let folder = scl_gui_widgets::widgets::icon_button(
        theme,
        icons::FOLDER,
        18.0,
        ButtonKind::Standard,
        common::command(crate::app::PUSH_PAGE, PAGE_GAME_DIR),
    );

    let search = common::text_input(
        theme,
        "搜索……",
        &state.versions_search_filter,
        move |value| Message::Ui(Text(crate::ui::TextField::VersionsFilter, value)),
    );

    let toolbar = container(
        row![
            sort_button,
            space().width(Length::Fixed(4.0)),
            container(search).width(Length::Fill),
            space().width(Length::Fixed(4.0)),
            folder,
        ]
        .align_y(Alignment::Center),
    )
    .padding([8, 8]);

    let mut list = column![].spacing(0);

    for version in sort_with_options(state) {
        let icon = version_icon(version.version_type);
        let setting_message = common::command(SELECT_VERSION_SETTING, version.name.clone());
        let select_message = common::command(SELECT_VERSION, version.name.clone());

        let icon_button = scl_gui_widgets::widgets::icon_button(
            &state.theme,
            icon,
            20.0,
            ButtonKind::Flat,
            setting_message,
        );

        let row = row![
            icon_button,
            space().width(Length::Fixed(4.0)),
            common::list_item(
                &state.theme,
                None,
                version.name.clone(),
                None,
                false,
                Some(select_message)
            ),
        ]
        .align_y(Alignment::Center)
        .padding([2, 0]);

        list = list.push(row);
    }

    column![
        toolbar,
        container(common::scroll(theme, container(list).padding([0, 8]))).height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// 创建版本列表页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    if state.versions.is_empty() {
        empty_state(&theme)
    } else {
        version_list_page(state, &theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(name: &str, version_type: VersionType, created_date: i64) -> LocalVersionInfo {
        LocalVersionInfo {
            name: name.to_owned(),
            version_type,
            path: String::new(),
            created_date,
            access_date: created_date,
        }
    }

    fn names(versions: &Vector<LocalVersionInfo>) -> Vec<&str> {
        versions
            .iter()
            .map(|version| version.name.as_str())
            .collect()
    }

    #[test]
    fn mode_0_and_1_both_sort_by_name() {
        let versions = im::vector![
            version("beta", VersionType::Forge, 300),
            version("alpha", VersionType::Vanilla, 200),
            version("Alpha", VersionType::Vanilla, 100),
        ];
        let expected = vec!["Alpha", "alpha", "beta"];
        assert_eq!(names(&sort_versions(versions.clone(), 0)), expected);
        assert_eq!(names(&sort_versions(versions, 1)), expected);
    }

    #[test]
    fn mode_2_sorts_by_created_desc_then_name() {
        let versions = im::vector![
            version("a", VersionType::Vanilla, 100),
            version("c", VersionType::Vanilla, 300),
            version("b", VersionType::Vanilla, 300),
        ];
        assert_eq!(names(&sort_versions(versions, 2)), vec!["b", "c", "a"]);
    }

    #[test]
    fn mode_3_uses_official_type_weight_order() {
        let versions = im::vector![
            version("neoforge", VersionType::NeoForge, 1),
            version("fabric", VersionType::Fabric, 2),
            version("quilt", VersionType::QuiltMC, 3),
            version("optifine", VersionType::Optifine, 4),
            version("forge", VersionType::Forge, 5),
            version("unknown", VersionType::Unknown, 6),
            version("vanilla", VersionType::Vanilla, 7),
        ];
        assert_eq!(
            names(&sort_versions(versions, 3)),
            vec![
                "unknown", "vanilla", "forge", "fabric", "optifine", "quilt", "neoforge"
            ]
        );
        assert!(version_type_rank(VersionType::Fabric) < version_type_rank(VersionType::NeoForge));
    }
}
