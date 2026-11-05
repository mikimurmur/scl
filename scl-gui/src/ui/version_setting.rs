//! 版本设置页面。
//!
//! 结构与官方 `ui::version_setting::ui_builder` 对齐：
//! 标题（固定 40 高）→ 图标/名称输入框/删除/文件夹一行 → 5 条菜单行
//! （独立设置、版本隔离、模组管理、地图管理、资源包管理）→ 底部
//! 「高级设置」「补全/修复版本文件」两个等宽按钮。
//! 页面没有「保存设置」按钮：官方在页面弹出时由
//! `app::version_setting::on_app_pop_page` 后台落盘。

use druid::{
    widget::{CrossAxisAlignment, Flex, TextBox},
    Env, LensExt, Target, Widget, WidgetExt,
};
use scl_core::version::structs::VersionInfo as CoreVersionInfo;
use scl_gui_widgets::{
    theme::{color::typography, icons::IconKeyPair},
    widgets::{label, Button, Icon, IconButton, MenuItem},
};

use crate::{
    app::FIX_VERSION,
    app_state::AppState,
    core::VersionType,
    theme::icons::{
        DELETE, FOLDER, GAME_FABRIC, GAME_FORGE, GAME_NEOFORGE, GAME_OPTIFINE, GAME_QUILTMC,
        GAME_VANILLA,
    },
    ui::{
        PAGE_VERSION_SETTING_ADVANCE, PAGE_VERSION_SETTING_MODS, PAGE_VERSION_SETTING_REMOVE,
        PUSH_PAGE,
    },
    utils::{open_url::open_url, prelude::should_version_independent, thread::thread_main},
};

/// 版本设置页标题高度，官方 `SizedBox` 固定 40。
const TITLE_HEIGHT: f64 = 40.;
/// 标题左右内边距，官方 `Padding((10., 0.))`。
const TITLE_H_PADDING: f64 = 10.;
/// 头部一行左右内边距，官方 `Padding((5., 0.))`。
const HEADER_H_PADDING: f64 = 5.;
/// 「高级设置」与「补全/修复版本文件」之间的间距。
const BUTTON_SPACING: f64 = 5.;
/// 底部按钮行内边距，官方 `Padding(5.)`。
const BOTTOM_PADDING: f64 = 5.;
/// 头部一行中图标与输入框之间的间距。
const HEADER_SPACING: f64 = 5.;

/// 版本类型（界面索引）对应的列表图标，与版本列表页保持一致。
fn version_type_icon(index: u8) -> IconKeyPair {
    match version_type_from_index(index) {
        VersionType::Forge => GAME_FORGE,
        VersionType::NeoForge => GAME_NEOFORGE,
        VersionType::Fabric => GAME_FABRIC,
        VersionType::QuiltMC => GAME_QUILTMC,
        VersionType::Optifine => GAME_OPTIFINE,
        VersionType::Vanilla | VersionType::Unknown => GAME_VANILLA,
    }
}

/// 把界面使用的数字转换为版本类型
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

/// 打开当前版本的游戏目录或其中的子目录（saves / resourcepacks）。
///
/// 官方在点击回调和底部图标回调里直接发起后台任务：加载 `VersionInfo`，
/// 没有独立配置时按全局「版本隔离方式」给出有效 `game_independent`，
/// 再由 `version_path()` 解析出目录；子目录不存在时先创建再打开。
fn open_version_dir(_ctx: &druid::EventCtx, data: &AppState, sub: Option<&'static str>) {
    let version_name = data.version_setting_version_name.to_owned();
    if version_name.is_empty() {
        return;
    }
    let version_base = data.versions_folder();
    let independence_method = data.config.version_independent_method;
    thread_main(move || {
        let mut info = CoreVersionInfo {
            version_base: version_base.to_string_lossy().to_string(),
            version: version_name,
            ..Default::default()
        };
        if info.load().is_ok() {
            if info.scl_launch_config.is_none() {
                let game_independent = should_version_independent(
                    independence_method,
                    &info.minecraft_version,
                    VersionType::from(info.version_type),
                );
                info.scl_launch_config = Some(scl_core::version::structs::SCLLaunchConfig {
                    game_independent,
                    ..Default::default()
                });
            }
            let mut dir = info.version_path();
            if let Some(sub) = sub {
                if sub == "resourcepacks"
                    && !dir.join("resourcepacks").exists()
                    && dir.join("texturepacks").exists()
                {
                    dir.push("texturepacks");
                } else {
                    dir.push(sub);
                }
                let _ = fs_err::create_dir_all(&dir);
            }
            if let Some(path) = dir.to_str() {
                let _ = open_url(path);
            }
        }
        Ok(())
    });
}

/// 创建版本设置页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    // 标题：官方固定 40 高，左右各 10 内边距（`Padding((10., 0.))`）。
    let title = label::new("版本设置")
        .with_font(typography::TITLE)
        .fix_height(TITLE_HEIGHT)
        .padding((TITLE_H_PADDING, 0.));

    // 版本图标随版本类型变化（官方 `Icon<u8>` 经 lens 绑定 version_setting_version_type）。
    let version_icon = Icon::<u8>::dynamic(|index: &u8, _: &Env| version_type_icon(*index))
        .lens(AppState::version_setting_version_type);

    // 版本名称输入框：官方无 placeholder，扩展宽度并作为 1.0 弹性子项。
    let rename = TextBox::new()
        .lens(AppState::version_setting_version_name_rename)
        .expand_width();

    // 官方两个图标按钮是无底色的扁平按钮。
    let delete = IconButton::new(DELETE)
        .with_flat(true)
        .on_click(|ctx, _data, _env| {
            ctx.submit_command(
                PUSH_PAGE
                    .with(PAGE_VERSION_SETTING_REMOVE)
                    .to(Target::Global),
            );
        });
    let folder =
        IconButton::new(FOLDER)
            .with_flat(true)
            .on_click(|ctx, data: &mut AppState, _env| {
                open_version_dir(ctx, data, None);
            });

    let header = Flex::row()
        .with_child(version_icon)
        .with_spacer(HEADER_SPACING)
        .with_flex_child(rename, 1.0)
        .with_spacer(HEADER_SPACING)
        .with_child(delete)
        .with_child(folder)
        .padding((HEADER_H_PADDING, 0.));

    let enable = MenuItem::new("使用独立版本设置")
        .lens(AppState::version_setting_config.then(crate::core::SCLLaunchConfig::enable));
    let isolation = MenuItem::new("使用版本隔离")
        .lens(AppState::version_setting_config.then(crate::core::SCLLaunchConfig::game_independent))
        .disabled_if(|data: &AppState, _: &Env| !data.version_setting_config.enable);

    let mods = MenuItem::new("模组管理")
        .lens(AppState::version_setting_mods_desc)
        .on_click(|ctx, _data, _env| {
            ctx.submit_command(PUSH_PAGE.with(PAGE_VERSION_SETTING_MODS).to(Target::Global));
        });
    let maps = MenuItem::new("地图管理")
        .lens(AppState::version_setting_maps_desc)
        .on_click(|ctx, data: &mut AppState, _env| {
            open_version_dir(ctx, data, Some("saves"));
        });
    let resource_packs = MenuItem::new("资源包管理")
        .lens(AppState::version_setting_resource_packs_desc)
        .on_click(|ctx, data: &mut AppState, _env| {
            open_version_dir(ctx, data, Some("resourcepacks"));
        });

    // 底部：两个等宽按钮。高级设置在未启用独立设置时禁用（官方 `DisabledIf`）。
    let advance = Button::new("高级设置")
        .on_click(|ctx, _data, _env| {
            ctx.submit_command(
                PUSH_PAGE
                    .with(PAGE_VERSION_SETTING_ADVANCE)
                    .to(Target::Global),
            );
        })
        .disabled_if(|data: &AppState, _: &Env| !data.version_setting_config.enable)
        .expand_width();
    let fix = Button::new("补全/修复版本文件")
        .on_click(|ctx, _data, _env| {
            ctx.submit_command(FIX_VERSION.with(()).to(Target::Global));
        })
        .expand_width();

    let actions = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_flex_child(advance, 1.0)
        .with_spacer(BUTTON_SPACING)
        .with_flex_child(fix, 1.0)
        .padding(BOTTOM_PADDING);

    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(title)
        .with_child(header)
        .with_spacer(HEADER_SPACING)
        .with_child(enable)
        .with_child(isolation)
        .with_child(mods)
        .with_child(maps)
        .with_child(resource_packs)
        .with_flex_spacer(1.0)
        .with_child(actions)
        .expand_width()
        .disabled_if(|data: &AppState, _: &Env| data.starting_tip.is_some());

    Box::new(content)
}
