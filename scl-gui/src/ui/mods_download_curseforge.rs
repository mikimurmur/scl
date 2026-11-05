//! CurseForge 模组搜索页面。

use druid::{
    widget::{Align, Click, ControllerHost, CrossAxisAlignment, Either, Flex, List, Scroll, SizedBox},
    Insets, Target, UnitPoint, Widget, WidgetExt,
};
use im::Vector;
use scl_gui_widgets::{
    widgets::{label, Button, IconButton, ProgressRing},
    WidgetExt as _,
};

use crate::{
    app::{CF_APPLY_SEARCH, CF_LOAD_MORE, CF_SELECT_MOD},
    app_state::{AppState, ModState},
    theme::icons::SEARCH,
    ui::common,
    widgets::{ModListItem, OptionView},
};

/// 「加载更多」按钮：仅在结果列表非空时显示（官方 show_if(!is_empty)）
fn load_more_button() -> impl Widget<Vector<ModState>> {
    Button::new("加载更多")
        .on_click(|ctx, _data, _env| {
            ctx.submit_command(CF_LOAD_MORE.with(()).to(Target::Global));
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
                            ctx.submit_command(CF_SELECT_MOD.with(data.clone()).to(Target::Global));
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
    // 官方此处只调用 label::new("没有结果")，文字颜色/字号沿用主题默认值。
    SizedBox::new(Align::new(
        UnitPoint::CENTER,
        label::new("没有结果"),
    ))
    .expand()
}

/// 请求进行中的分支（结果列表为 None）：内容区居中的转圈
fn loading_view() -> impl Widget<()> {
    SizedBox::new(Align::new(
        UnitPoint::CENTER,
        ProgressRing::new(),
    ))
    .expand()
}

/// 创建 CurseForge 搜索页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let toolbar = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(
            IconButton::new(SEARCH)
                .with_accent(true)
                .on_click(|ctx, _data, _env| {
                    ctx.submit_command(CF_APPLY_SEARCH.with(()).to(Target::Global));
                }),
        )
        .with_spacer(6.)
        .with_flex_child(
            common::text_box("搜索…")
                .controller(InputController(CF_APPLY_SEARCH))
                .lens(AppState::mods_download_curseforge_search_filter)
                .expand_width(),
            1.0,
        )
        .expand_width()
        .padding(Insets::new(8., 10., 8., 8.));

    // 用稳定的 OptionView 而不是 druid Maybe：Maybe 在运行期切换 Some/None 时
    // 会重建 child pod 并把 BuildFocusChain 提前发给未 WidgetAdded 的新 pod，
    // 触发 core.rs 断言（见 option_view.rs 注释）。
    let content = OptionView::new(
        Either::new(|mods: &Vector<ModState>, _| !mods.is_empty(), results_view(), empty_view()),
        loading_view(),
    )
    .lens(AppState::mods_download_curseforge_result_mods);

    Box::new(
        Flex::column()
            .with_child(toolbar)
            .with_flex_child(content, 1.0)
            .disabled_if(|data: &AppState, _: &druid::Env| {
                data.mods_download_curseforge_searching
            }),
    )
}

/// 搜索输入框控制器：按下回车时执行搜索
pub struct InputController(pub druid::Selector<()>);

impl<T: druid::Data, W: Widget<T>> druid::widget::Controller<T, W> for InputController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut T,
        env: &druid::Env,
    ) {
        if let druid::Event::KeyDown(key) = event {
            if key.code == druid::Code::Enter {
                ctx.submit_command(self.0.with(()).to(druid::Target::Global));
                return;
            }
        }
        child.event(ctx, event, data, env);
    }
}
