//! 版本设置页面。
//!
//! 结构与官方 `ui::version_setting::ui_builder` 对齐：
//! 标题 → 图标/名称输入框/删除/文件夹一行 → 5 条菜单行
//! （独立设置、版本隔离、模组管理、地图管理、资源包管理）→ 底部
//! 「高级设置」「补全/修复版本文件」两个等宽按钮。
//! 页面没有「保存设置」按钮：官方在页面弹出时由
//! `app::version_setting::on_app_pop_page` 异步落盘。

use iced::widget::{column, container, row, space};
use iced::{Alignment, Element, Length};
use scl_gui_widgets::icons;
use scl_gui_widgets::widgets::{self, ButtonKind};

use crate::{
    app::{FIX_VERSION, Message},
    app_state::AppState,
    core::VersionType,
    ui::{
        BoolField, PAGE_VERSION_SETTING_ADVANCE, PAGE_VERSION_SETTING_MODS,
        PAGE_VERSION_SETTING_REMOVE, TextField, UiMessage, common,
    },
};

/// 把界面使用的数字转换为版本类型。
fn version_type_from_index(index: u8) -> VersionType {
    match index {
        0 => VersionType::Vanilla,
        1 => VersionType::Forge,
        2 => VersionType::NeoForge,
        3 => VersionType::Fabric,
        4 => VersionType::QuiltMC,
        5 => VersionType::Optifine,
        _ => VersionType::Unknown,
    }
}

/// 版本类型（界面索引）对应的列表图标，与版本列表页保持一致。
fn version_type_icon(index: u8) -> scl_gui_widgets::icons::Icon {
    crate::ui::version::version_icon(version_type_from_index(index))
}

/// 创建版本设置页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let config = &state.version_setting_config;

    let title = container(common::title(&theme, "版本设置"))
        .height(Length::Fixed(40.0))
        .padding([0, 10]);

    let rename = common::text_input(
        &theme,
        "",
        &state.version_setting_version_name_rename,
        move |value| Message::Ui(UiMessage::Text(TextField::VersionRename, value)),
    );

    let delete = widgets::icon_button(
        &theme,
        icons::DELETE,
        16.0,
        ButtonKind::Flat,
        common::command(crate::app::PUSH_PAGE, PAGE_VERSION_SETTING_REMOVE),
    );
    let folder = widgets::icon_button(
        &theme,
        icons::FOLDER,
        16.0,
        ButtonKind::Flat,
        Message::Ui(UiMessage::OpenVersionDir(None)),
    );

    let header = row![
        version_type_icon(state.version_setting_version_type).view_default(&theme, 20.0),
        space().width(Length::Fixed(5.0)),
        container(rename).width(Length::Fill),
        space().width(Length::Fixed(5.0)),
        delete,
        folder,
    ]
    .align_y(Alignment::Center)
    .padding([0, 5]);

    let enable = common::menu_toggle(
        &theme,
        "使用独立版本设置",
        config.enable,
        true,
        Message::Ui(UiMessage::Bool(BoolField::VersionEnable, !config.enable)),
    );
    let isolation = common::menu_toggle(
        &theme,
        "使用版本隔离",
        config.game_independent,
        config.enable,
        Message::Ui(UiMessage::Bool(
            BoolField::VersionIndependent,
            !config.game_independent,
        )),
    );
    let mods = common::menu_item(
        &theme,
        "模组管理",
        state.version_setting_mods_desc.clone(),
        true,
        common::command(crate::app::PUSH_PAGE, PAGE_VERSION_SETTING_MODS),
    );
    let maps = common::menu_item(
        &theme,
        "地图管理",
        state.version_setting_maps_desc.clone(),
        true,
        Message::Ui(UiMessage::OpenVersionDir(Some("saves"))),
    );
    let resource_packs = common::menu_item(
        &theme,
        "资源包管理",
        state.version_setting_resource_packs_desc.clone(),
        true,
        Message::Ui(UiMessage::OpenVersionDir(Some("resourcepacks"))),
    );

    let advance = container(widgets::fill_button_maybe(
        &theme,
        "高级设置",
        ButtonKind::Standard,
        config.enable.then_some(common::command(
            crate::app::PUSH_PAGE,
            PAGE_VERSION_SETTING_ADVANCE,
        )),
    ))
    .width(Length::FillPortion(1));

    let fix = container(widgets::fill_button(
        &theme,
        "补全/修复版本文件",
        ButtonKind::Standard,
        common::command(FIX_VERSION, ()),
    ))
    .width(Length::FillPortion(1));

    let actions = row![advance, space().width(Length::Fixed(5.0)), fix,]
        .align_y(Alignment::Center)
        .padding(5);

    column![
        title,
        header,
        space().height(Length::Fixed(5.0)),
        enable,
        isolation,
        mods,
        maps,
        resource_packs,
        space().height(Length::Fill),
        actions,
    ]
    .spacing(0)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
