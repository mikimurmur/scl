//! Modrinth 模组搜索页面。

use std::sync::atomic::{AtomicBool, Ordering};

use druid::{
    widget::{Align, Click, ControllerHost, CrossAxisAlignment, Either, Flex, List, Scroll, SizedBox},
    Insets, Target, UnitPoint, Widget, WidgetExt,
};
use im::Vector;
use scl_gui_widgets::{
    widgets::{label, Button, ProgressRing},
    WidgetExt as _,
};

use crate::{
    app::{MR_APPLY_SEARCH, MR_LOAD_MORE, MR_SELECT_MOD},
    app_state::{AppState, ModState},
    ui::common,
    widgets::{ModListItem, OptionView},
};

/// 进入页面清空搜索框时抑制一次「输入变动自动搜索」，
/// 避免与 on_app_push_page 显式发起的那次搜索重复请求。
pub static SKIP_NEXT_INPUT_SEARCH: AtomicBool = AtomicBool::new(false);

/// 「加载更多」按钮：仅在结果列表非空时显示（官方 show_if(!is_empty)）
fn load_more_button() -> impl Widget<Vector<ModState>> {
    Button::new("加载更多")
        .on_click(|ctx, _data, _env| {
            ctx.submit_command(MR_LOAD_MORE.with(()).to(Target::Global));
        })
        .show_if(|mods: &Vector<ModState>, _: &druid::Env| !mods.is_empty())
        .expand_width()
        .padding(Insets::new(10., 4., 10., 10.))
}

/// 有结果时的分支：可滚动的列表 + 加载更多
fn results_view() -> impl Widget<Vector<ModState>> {
    Scroll::new(
        Flex::column()
            .with_child(
                List::new(|| {
                    ControllerHost::new(
                        ModListItem::new(),
                        Click::new(|ctx, data: &mut ModState, _env| {
                            ctx.submit_command(MR_SELECT_MOD.with(data.clone()).to(Target::Global));
                        }),
                    )
                })
                .expand_width(),
            )
            .with_child(load_more_button()),
    )
    .vertical()
    .expand()
}

/// 搜索完成但结果为空时的分支：内容区居中的「没有结果」
fn empty_view() -> impl Widget<Vector<ModState>> {
    SizedBox::new(Align::new(UnitPoint::CENTER, label::new("没有结果"))).expand()
}

/// 请求进行中的分支（结果列表为 None）：内容区居中的转圈
fn loading_view() -> impl Widget<()> {
    SizedBox::new(Align::new(UnitPoint::CENTER, ProgressRing::new())).expand()
}

/// 创建 Modrinth 搜索页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let toolbar = Flex::row()
        .with_flex_child(
            common::text_box("搜索…")
                .lens(AppState::mods_download_modrinth_search_filter)
                .controller(SearchOnChange)
                .expand_width(),
            1.0,
        )
        .expand_width()
        .padding(Insets::new(8., 10., 8., 8.));

    // 稳定 OptionView（替代 druid Maybe，避免 None↔Some 切换触发
    // BuildFocusChain-before-WidgetAdded 断言，详见 widgets/option_view.rs）。
    let content = OptionView::new(
        Either::new(|mods: &Vector<ModState>, _| !mods.is_empty(), results_view(), empty_view()),
        loading_view(),
    )
    .lens(AppState::mods_download_modrinth_result_mods)
    .disabled_if(|data: &AppState, _: &druid::Env| data.mods_download_modrinth_searching);

    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(toolbar)
            .with_flex_child(content, 1.0),
    )
}

/// 搜索输入框控制器：输入内容变动即发起搜索（官方 MR 行为，无防抖延时）
pub struct SearchOnChange;

impl<W: Widget<AppState>> druid::widget::Controller<AppState, W> for SearchOnChange {
    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut druid::UpdateCtx,
        old_data: &AppState,
        data: &AppState,
        env: &druid::Env,
    ) {
        let changed =
            old_data.mods_download_modrinth_search_filter != data.mods_download_modrinth_search_filter;
        if changed {
            // 页面打开时程序化清空输入框：若置了抑制位且新值为空，则不再重复发请求。
            let suppressed = SKIP_NEXT_INPUT_SEARCH.swap(false, Ordering::SeqCst);
            if !(suppressed && data.mods_download_modrinth_search_filter.is_empty()) {
                ctx.submit_command(MR_APPLY_SEARCH.with(()).to(Target::Global));
            }
        }
        child.update(ctx, old_data, data, env);
    }
}
