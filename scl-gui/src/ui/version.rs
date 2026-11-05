//! 已安装版本列表页面。

use druid::{
    widget::{CrossAxisAlignment, Flex, List, Scroll, TextBox, ViewSwitcher},
    Env, Insets, Lens, Target, Widget, WidgetExt,
};
use im::Vector;
use scl_gui_widgets::{
    theme::{color::typography, icons::IconKeyPair},
    widgets::{label, Button, IconButton, ListItem},
};

use crate::{
    app::{SELECT_VERSION, SELECT_VERSION_SETTING},
    app_state::{AppState, LocalVersionInfo},
    core::VersionType,
    theme::icons::{
        CATEGORIED_BY_VERSION_TYPE, FOLDER, GAME_FABRIC, GAME_FORGE, GAME_NEOFORGE, GAME_OPTIFINE,
        GAME_QUILTMC, GAME_VANILLA, SORT_BY_DOWNLOAD_DATE, SORT_BY_NAME,
        SORT_BY_VERSION_RELEASE_DATE,
    },
    ui::{PAGE_DOWNLOAD, PAGE_GAME_DIR, PUSH_PAGE},
};

/// 根据版本类型返回列表中显示的图标
fn version_icon(data: &LocalVersionInfo, _env: &Env) -> IconKeyPair {
    match data.version_type {
        VersionType::Vanilla | VersionType::Unknown => GAME_VANILLA,
        VersionType::Forge => GAME_FORGE,
        VersionType::NeoForge => GAME_NEOFORGE,
        VersionType::Fabric => GAME_FABRIC,
        VersionType::QuiltMC => GAME_QUILTMC,
        VersionType::Optifine => GAME_OPTIFINE,
    }
}

/// 排序按钮当前对应的图标
///
/// 原版按「名称 → 版本发布时间 → 下载时间 → 按版本类型分类」循环，共 4 种模式
/// （0~3，初始为 0）。模式 1 的图标虽是「版本发布时间」，但官方该档未实现时间序，
/// 实际结果与模式 0 相同（见 [`sort_versions`]）。
fn sort_icon(data: &AppState, _env: &Env) -> IconKeyPair {
    match data.versions_sort_mode {
        1 => SORT_BY_VERSION_RELEASE_DATE,
        2 => SORT_BY_DOWNLOAD_DATE,
        3 => CATEGORIED_BY_VERSION_TYPE,
        _ => SORT_BY_NAME,
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
    // 原版 `SearchVersionLens` 直接对名称调用 `str::contains`，没有转小写；
    // 因此这里的搜索与排序名称比较都保持大小写敏感。
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
        // 模式 0 与模式 1 结果相同：官方模式 1 未实现时间序（OB-14）。
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

/// 根据搜索关键词过滤并排序版本列表
pub struct SearchVersionLens;

impl Lens<AppState, Vector<LocalVersionInfo>> for SearchVersionLens {
    fn with<V, F: FnOnce(&Vector<LocalVersionInfo>) -> V>(&self, data: &AppState, f: F) -> V {
        f(&sort_with_options(data))
    }

    fn with_mut<V, F: FnOnce(&mut Vector<LocalVersionInfo>) -> V>(
        &self,
        data: &mut AppState,
        f: F,
    ) -> V {
        f(&mut sort_with_options(data))
    }
}

/// 版本列表中的一行
///
/// 与官方 `ui::version` 的控件组合一致：`Flex::row` 内放置
/// `IconButton<VersionState>`（点击打开版本设置页）与
/// `ListItem<VersionState>`（点击选择版本并回主页）。
/// 官方图标按钮为 32px，名称区为 `ListItem`（32px 高），
/// 行上下各留 2px（行间距 36px），列表内容左右各 8px。
fn version_item() -> impl Widget<LocalVersionInfo> {
    Flex::row()
        .with_child(IconButton::dynamic(version_icon).with_flat(true).on_click(
            |ctx, data: &mut LocalVersionInfo, _env| {
                // 官方入口：点击版本图标后台加载该版本配置并推入
                ctx.submit_command(
                    SELECT_VERSION_SETTING
                        .with(data.name.clone())
                        .to(Target::Global),
                );
            },
        ))
        .with_spacer(4.)
        .with_flex_child(
            ListItem::dynamic(|data: &LocalVersionInfo, _env| data.name.clone())
                .on_click(|ctx, data: &mut LocalVersionInfo, _env| {
                    ctx.submit_command(SELECT_VERSION.with(data.name.clone()).to(Target::Global));
                })
                // 本地 druid 的 Flex 不会把弹性子项拉满，需显式扩展宽度才能
                // 与官方一致：名称区覆盖图标右侧整行，悬停/点击都生效。
                .expand_width(),
            1.0,
        )
        .padding(Insets::new(0., 2., 0., 2.))
}

/// 没有版本时显示的空状态
fn empty_state() -> Box<dyn Widget<AppState>> {
    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_flex_spacer(1.0)
            .with_child(
                label::new("没有找到任何版本，请检查当前目录是否存在游戏\n如果确实没有下载游戏的话，要不要现在来下载一个呢？")
                    .with_font(typography::CAPTION)
                    .with_text_color(typography::FONT_COLOR)
                    .padding(4.),
            )
            .with_spacer(32.)
            .with_child(
                Button::new("带我下载游戏吧！")
                    .with_accent(true)
                    .on_click(|ctx, _data, _env| {
                        ctx.submit_command(PUSH_PAGE.with(PAGE_DOWNLOAD).to(Target::Global));
                    }),
            )
            .with_spacer(4.)
            .with_child(Button::new("切换游戏目录").on_click(|ctx, _data, _env| {
                ctx.submit_command(PUSH_PAGE.with(PAGE_GAME_DIR).to(Target::Global));
            }))
            .with_flex_spacer(1.0)
            .padding(4.),
    )
}

/// 有版本时显示的列表页面
fn version_list_page() -> Box<dyn Widget<AppState>> {
    let sort = IconButton::dynamic(sort_icon).with_accent(true).on_click(
        |_ctx, data: &mut AppState, _env| {
            // 官方循环顺序：0 → 1 → 2 → 3 → 0。
            data.versions_sort_mode = match data.versions_sort_mode {
                0 => 1,
                1 => 2,
                2 => 3,
                _ => 0,
            };
        },
    );
    let folder = IconButton::new(FOLDER).on_click(|ctx, _data, _env| {
        ctx.submit_command(PUSH_PAGE.with(PAGE_GAME_DIR).to(Target::Global));
    });
    let search = TextBox::new()
        .with_placeholder("搜索……")
        .lens(AppState::versions_search_filter)
        .expand_width();

    let toolbar = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(sort)
        .with_spacer(4.)
        .with_flex_child(search, 1.0)
        .with_spacer(4.)
        .with_child(folder)
        .padding(Insets::new(8., 8., 8., 5.83));

    let list = List::new(version_item).lens(SearchVersionLens);

    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(toolbar)
            .with_flex_child(
                Scroll::new(list.padding(Insets::new(8., 0., 8., 0.))).vertical(),
                1.0,
            ),
    )
}

/// 创建版本列表页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    Box::new(ViewSwitcher::new(
        |data: &AppState, _env: &Env| data.versions.is_empty(),
        |empty: &bool, _data: &AppState, _env: &Env| -> Box<dyn Widget<AppState>> {
            if *empty {
                empty_state()
            } else {
                version_list_page()
            }
        },
    ))
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
        // 名称逐字节比较：大写 `Alpha` 排在小写 `alpha` 之前。
        let expected = vec!["Alpha", "alpha", "beta"];
        assert_eq!(names(&sort_versions(versions.clone(), 0)), expected);
        // 模式 1 官方未实现时间序（OB-14），结果必须与模式 0 完全相同。
        assert_eq!(names(&sort_versions(versions, 1)), expected);
    }

    #[test]
    fn mode_2_sorts_by_created_desc_then_name() {
        let versions = im::vector![
            version("a", VersionType::Vanilla, 100),
            version("c", VersionType::Vanilla, 300),
            version("b", VersionType::Vanilla, 300),
        ];
        // 创建时间降序；相同时间按名称升序兜底 → b、c，最后 a。
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
            vec!["unknown", "vanilla", "forge", "fabric", "optifine", "quilt", "neoforge"]
        );
        assert!(version_type_rank(VersionType::Fabric) < version_type_rank(VersionType::NeoForge));
    }
}
