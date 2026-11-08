//! 版本模组管理页面。

use druid::{
    widget::{CrossAxisAlignment, Flex, List, Scroll, TextBox},
    Env, Insets, Lens, Target, Widget, WidgetExt,
};
use im::Vector;
use scl_gui_widgets::{
    theme::color::{base, typography},
    widgets::{label, DynImage, OnChange, ToggleSwitch},
};

use crate::{
    app::TOGGLE_MOD,
    app_state::{AppState, LocalMod},
    ui::common,
};

/// 每行图标尺寸，官方为 40×40。
const ICON_SIZE: f64 = 40.;
/// 图标与文字之间的距离。
const ICON_TEXT_SPACING: f64 = 7.;
/// 列表左右内边距，官方为 8。
const LIST_H_PADDING: f64 = 8.;
/// 行间垂直间距（每行底部留 5）。
const ROW_SPACING: f64 = 5.;
/// 列表右侧留白，给覆盖式滚动条留出空间。
const LIST_RIGHT_PADDING: f64 = 10.;
/// 搜索框内边距，官方为 10。
const SEARCH_PADDING: f64 = 10.;

/// 根据搜索关键词过滤当前版本的模组
pub struct SearchModsLens;

impl Lens<AppState, Vector<LocalMod>> for SearchModsLens {
    fn with<V, F: FnOnce(&Vector<LocalMod>) -> V>(&self, data: &AppState, f: F) -> V {
        // 直接复用缓存好的过滤结果，滚轮/绘制帧不再克隆整个模组列表。
        f(&data.version_mods_filtered)
    }

    fn with_mut<V, F: FnOnce(&mut Vector<LocalMod>) -> V>(&self, data: &mut AppState, f: F) -> V {
        // 滚轮/悬停也会走到 with_mut；先记录 enabled 快照，只有真的切换了
        // 开关才做按路径回写，避免每个滚轮事件都跑一遍 O(n²) 查找。
        let before: Vec<bool> = data
            .version_mods_filtered
            .iter()
            .map(|item| item.enabled)
            .collect();
        let result = f(&mut data.version_mods_filtered);
        let changed = data
            .version_mods_filtered
            .iter()
            .zip(before.iter())
            .any(|(item, old)| item.enabled != *old);
        if !changed {
            return result;
        }
        for item in data.version_mods_filtered.iter() {
            if let Some(slot) = data
                .version_mods
                .iter_mut()
                .find(|slot| slot.path == item.path)
            {
                if slot.enabled != item.enabled {
                    slot.enabled = item.enabled;
                }
            }
        }
        result
    }
}

/// 一行模组：图标 + 名称/版本 + 启用开关
fn mod_row() -> impl Widget<LocalMod> {
    let text = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(
            // 官方 Mods 行是单行硬截断（`LineBreaking::Clip`，末尾没有省略号）：
            // 长文件名裁在可见宽度处，不换行、不越过右侧开关。
            label::new(|data: &LocalMod, _: &Env| data.name.clone())
                .with_font(typography::CAPTION)
                .with_text_color(base::HIGH)
                .with_line_break_mode(druid::widget::LineBreaking::Clip),
        )
        .with_child(
            label::new(|data: &LocalMod, _: &Env| data.version.clone())
                .with_font(typography::CAPTION)
                .with_text_color(base::MEDIUM)
                .with_line_break_mode(druid::widget::LineBreaking::Clip),
        );

    let toggle = ToggleSwitch::new()
        .lens(LocalMod::enabled)
        // 官方的启用/禁用等价于把文件重命名为 `*.disabled` 或改回原名，
        // 由后台任务完成（`ui::version_setting_mods` 内联闭包按
        // `ends_with(".disabled")` 分支处理）。
        .controller(OnChange::new(
            |ctx: &mut druid::EventCtx, old: &LocalMod, data: &mut LocalMod, _env: &Env| {
                if old.enabled != data.enabled {
                    ctx.submit_command(TOGGLE_MOD.with(data.path.clone()).to(Target::Global));
                }
            },
        ));

    Flex::row()
        .with_child(
            DynImage::default()
                .lens(LocalMod::icon)
                .fix_size(ICON_SIZE, ICON_SIZE),
        )
        .with_spacer(ICON_TEXT_SPACING)
        .must_fill_main_axis(true)
        .with_flex_child(text.expand_width(), 1.0)
        .with_child(toggle)
        .expand_width()
        .fix_height(ICON_SIZE)
        .padding(Insets::new(0., ROW_SPACING / 2., 0., ROW_SPACING / 2.))
}

/// 创建版本模组管理页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let search = TextBox::new()
        .with_placeholder("搜索模组……")
        .lens(AppState::version_mods_search_filter)
        .controller(OnChange::new(
            |_ctx: &mut druid::EventCtx, _old: &AppState, data: &mut AppState, _env: &Env| {
                data.rebuild_mods_filter();
            },
        ))
        .expand_width()
        .padding(Insets::new(
            SEARCH_PADDING,
            SEARCH_PADDING,
            SEARCH_PADDING,
            SEARCH_PADDING - 2.,
        ));

    let list = List::new(mod_row).lens(SearchModsLens).padding(Insets::new(
        LIST_H_PADDING,
        0.,
        LIST_RIGHT_PADDING,
        0.,
    ));

    let content = common::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(search)
        .with_flex_child(Scroll::new(list).vertical().expand(), 1.0)
        .expand();

    Box::new(content)
}
