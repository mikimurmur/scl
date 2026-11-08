//! 账户管理页面。

use druid::{
    piet::{Text, TextLayoutBuilder},
    widget::{CrossAxisAlignment, Flex, List, Scroll},
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle, LifeCycleCtx, PaintCtx,
    RenderContext, Size, Target, Widget, WidgetExt,
};
use im::Vector;
use scl_gui_widgets::theme::color::{base, list, typography};

use crate::{
    app_state::AppState,
    ui::{common, PAGE_LOGIN_METHODS, PUSH_PAGE},
    widgets::account_item::{AccountItem, AccountItemData},
};

/// 根据搜索关键词过滤账户列表
pub struct AccountsLens;

impl Lens<AppState, Vector<AccountItemData>> for AccountsLens {
    fn with<V, F: FnOnce(&Vector<AccountItemData>) -> V>(&self, data: &AppState, f: F) -> V {
        let filter = data.accounts_filter.trim().to_lowercase();
        let selected = data.config.accounts_data.selected;
        let accounts: Vector<AccountItemData> = data
            .config
            .accounts_data
            .accounts
            .iter()
            .enumerate()
            .filter(|(_, account)| {
                filter.is_empty()
                    || account.player_name().to_lowercase().contains(&filter)
                    || account.type_name().to_lowercase().contains(&filter)
            })
            .map(|(index, account)| (index == selected, index, account.clone()))
            .collect();
        f(&accounts)
    }

    fn with_mut<V, F: FnOnce(&mut Vector<AccountItemData>) -> V>(
        &self,
        data: &mut AppState,
        f: F,
    ) -> V {
        let selected = data.config.accounts_data.selected;
        let mut accounts: Vector<AccountItemData> = data
            .config
            .accounts_data
            .accounts
            .iter()
            .enumerate()
            .map(|(index, account)| (index == selected, index, account.clone()))
            .collect();
        f(&mut accounts)
    }
}

/// “创建新账户”列表项
struct CreateAccountItem;

impl Widget<AppState> for CreateAccountItem {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut AppState, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) if ctx.is_active() => {
                ctx.set_active(false);
                ctx.request_paint();
                ctx.submit_command(PUSH_PAGE.with(PAGE_LOGIN_METHODS).to(Target::Global));
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &AppState,
        _env: &Env,
    ) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
    }

    fn update(&mut self, _: &mut druid::UpdateCtx, _old: &AppState, _new: &AppState, _env: &Env) {}

    fn layout(
        &mut self,
        _: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &AppState,
        _env: &Env,
    ) -> Size {
        bc.constrain(Size::new(bc.max().width, 50.))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &AppState, env: &Env) {
        let size = ctx.size();
        // 原版 CreateItem 只在悬停/按下时绘制背景，视频中的灰色创建行正是鼠标悬停状态。
        if ctx.is_hot() {
            let bg = if ctx.is_active() {
                env.get(base::LOW)
            } else {
                env.get(list::LIST_LOW)
            };
            ctx.fill(size.to_rect(), &bg);
        }

        let font_color = env.get(typography::FONT_COLOR);
        let path = druid::kurbo::BezPath::new();
        let mut path = path;
        path.move_to((26., 13.));
        path.line_to((26., 37.));
        path.move_to((14., 25.));
        path.line_to((38., 25.));
        ctx.stroke(&path, &font_color, 4.);

        if let Ok(layout) = ctx
            .text()
            .new_text_layout("创建新账户")
            .font(env.get(typography::BASE).family, 13.)
            .text_color(font_color)
            .build()
        {
            ctx.draw_text(&layout, (50., 17.));
        }
    }
}

/// 创建账户管理页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    // 原版只有搜索框左右各留 8 像素，账户列表和“创建新账户”行铺满整个窗口。
    let search = common::text_box("搜索账户……")
        .lens(AppState::accounts_filter)
        .expand_width()
        .padding(druid::Insets::new(8., 0., 8., 0.));

    let list = List::new(AccountItem::new)
        .lens(AccountsLens)
        .expand_width();

    let scroll = Scroll::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(list)
            .with_child(CreateAccountItem),
    )
    .vertical()
    .expand();

    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(search)
            .with_spacer(8.)
            .with_flex_child(scroll, 1.0)
            .padding(druid::Insets::new(0., 8., 0., 8.))
            .expand(),
    )
}
