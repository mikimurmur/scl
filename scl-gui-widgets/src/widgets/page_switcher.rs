use std::collections::{HashMap, VecDeque};

use druid::{piet::*, *};

type InnerBuilder<T> = HashMap<&'static str, Box<dyn Fn() -> Box<dyn Widget<T>>>>;
type Inner<T> = HashMap<&'static str, WidgetPod<T, Box<dyn Widget<T>>>>;

const ANIMATION_TIME: u64 = 300_000000;

/// 当分页组件跳转到指定页面时抛出的 [`druid::Notification`]
///
/// 参数是跳转到的当前页面 ID
pub const ON_PAGE: Selector<&'static str> = Selector::new("net.stevexmh.scl.on-page");
/// 使分页组件跳转到指定页面时的 [`druid::Command`]
///
/// 参数是需要跳转到的页面 ID
pub const PUSH_PAGE: Selector<&str> = Selector::new("net.stevexmh.scl.push-page");
/// 当分页组件即将离开指定页面时抛出的 [`druid::Notification`]
///
/// 参数是即将离开的页面 ID
pub const POP_PAGE: Selector<&str> = Selector::new("net.stevexmh.scl.pop-page");
/// 使分页组件回到上一个或指定页面时的 [`druid::Command`]
///
/// 字符串为需要返回的页面，如果留空则回到上一个页面，如果非空则一直返回到该页面或回到第一页
pub const QUERY_POP_PAGE: Selector<&str> = Selector::new("net.stevexmh.scl.query-pop-page");
/// 立即终止指定页面之上的所有页面（含尚未完成推入动画的中间页），然后回到该页面
///
/// 仅用于启动失败/崩溃的终态收尾：该序列要求页面栈必须是
/// `["main", "launch_error"]`，若 500ms 弹栈请求落在 `starting` 的推入动画期间，
/// 普通 [`QUERY_POP_PAGE`] 会按官方语义直接丢弃，导致 `starting` 永久留在栈上，
/// 错误页返回时落到启动页。本命令不做延后排队，而是立即撤销未完成的推入动画并
/// 截断页面链，正常（动画已结束）时行为与 [`QUERY_POP_PAGE`] 完全一致。
pub const RESET_PAGE_TO: Selector<&str> = Selector::new("net.stevexmh.scl.reset-page-to");
/// 使分页组件使用滑动动画而非缩放渐变动画的 [`druid::Command`]
///
/// 参数为是否使用滑动动画
pub const SET_SLIDE_PAGE_ANIMATION: Selector<bool> =
    Selector::new("net.stevexmh.scl.set-slide-page-animation");

#[derive(Debug)]
enum AnimationType {
    PushZoom(&'static str),
    PopZoom(&'static str),
    PushSlide(&'static str),
    PopSlide(&'static str),
    // Used when first started
    PushMoveUp(&'static str),
}

/// 一个分页组件，提供类似 WinUI 3 的缩放/滑动切换动画来切换所选页面
///
/// 且看不到的页面会被释放，不会占用过多内存
///
/// 每个页面必须提供一个静态的页面 ID [`str`]，作为唯一标识以进行跳转等操作
///
/// 默认情况下，第一个被注册的页面将会作为首页被显示出来
///
/// 之后通过发送 [`PUSH_PAGE`] 或 [`QUERY_POP_PAGE`] 来跳转/退出指定页面
///
/// 推荐配合 [`crate::widgets::WindowWidget`] 使用，配合其返回按钮的 [`crate::widgets::BACK_PAGE_CLICKED`] 通知来更好地返回页面
pub struct PageSwitcher<T> {
    page_anime_timer: u64,
    active_page: &'static str,
    page_chain: Vec<&'static str>,
    page_anime_queue: VecDeque<AnimationType>,
    inner_builder: InnerBuilder<T>,
    inner: Inner<T>,
    slide_page_animation: bool,
    skip_first_anime_frame: u8,
}

impl<T> PageSwitcher<T> {
    /// 创建一个空白的分页组件
    pub fn new() -> Self {
        PageSwitcher::default()
    }
}

impl<T> Default for PageSwitcher<T> {
    fn default() -> Self {
        Self {
            page_anime_timer: 0,
            active_page: "",
            page_anime_queue: VecDeque::with_capacity(16),
            page_chain: Vec::with_capacity(8),
            inner_builder: HashMap::with_capacity(1),
            inner: HashMap::with_capacity(1),
            slide_page_animation: false,
            skip_first_anime_frame: 0,
        }
    }
}

impl<T: Data> PageSwitcher<T> {
    fn clean_unused_page(&mut self) {
        self.inner.retain(|k, _| self.page_chain.contains(k));
    }

    fn load_page(&mut self, key: &'static str) {
        if !self.inner.contains_key(key) {
            if let Some(page_builder) = self.inner_builder.get(key) {
                let w = WidgetPod::new(page_builder()).boxed();
                self.inner.insert(key, w);
            }
        }
    }

    /// 把弹栈动画加入队列。
    fn enqueue_pop(&mut self, ctx: &mut EventCtx, target: &'static str) {
        if self.slide_page_animation {
            self.page_anime_queue
                .push_back(AnimationType::PopSlide(target));
        } else {
            self.page_anime_queue
                .push_back(AnimationType::PopZoom(target));
        }
        self.skip_first_anime_frame = 1;
        ctx.request_anim_frame();
    }

    /// 增加一个页面，需要一个页面 ID 和一个生成该页面组件的回调函数
    pub fn add_page(
        &mut self,
        key: &'static str,
        page_widget: Box<dyn Fn() -> Box<dyn Widget<T> + 'static>>,
    ) {
        if self.page_chain.is_empty() && self.inner_builder.is_empty() {
            self.page_anime_queue
                .push_back(AnimationType::PushMoveUp(key));
            self.load_page(key);
        }
        #[cfg(debug_assertions)]
        {
            if self.inner_builder.contains_key(key) {
                panic!("Page {key} has already registered");
            }
        }
        self.inner_builder.insert(key, page_widget);
    }

    /// 以 Builder 模式增加一个页面，需要一个页面 ID 和一个生成该页面组件的回调函数
    pub fn with_page(
        mut self,
        key: &'static str,
        page_widget: Box<dyn Fn() -> Box<dyn Widget<T> + 'static>>,
    ) -> Self {
        self.add_page(key, page_widget);
        self
    }
}

impl<T: Data> Widget<T> for PageSwitcher<T> {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut T,
        env: &druid::Env,
    ) {
        if let Event::AnimFrame(i) = event {
            let anime_queue_is_empty = self.page_anime_queue.is_empty();
            if !anime_queue_is_empty {
                if self.skip_first_anime_frame == 0 {
                    if self.page_anime_timer == 0 {
                        self.page_anime_timer += i;
                        let preload_page = match self.page_anime_queue.front() {
                            Some(AnimationType::PushZoom(page))
                            | Some(AnimationType::PushMoveUp(page)) => *page,
                            _ => "",
                        };
                        if !preload_page.is_empty() {
                            self.load_page(preload_page);
                            ctx.children_changed();
                        }
                    } else {
                        self.page_anime_timer += i;
                        if self.page_anime_timer > ANIMATION_TIME {
                            self.page_anime_timer = 0;
                            match self.page_anime_queue.pop_front() {
                                Some(AnimationType::PushZoom(page))
                                | Some(AnimationType::PushSlide(page))
                                | Some(AnimationType::PushMoveUp(page)) => {
                                    self.load_page(page);
                                    ctx.children_changed();
                                    ctx.request_update();
                                    self.active_page = page;
                                    // 页面切换后清掉被缓存页面残留的悬停状态，
                                    // 否则未移动鼠标时旧页面中的行会保持高亮。
                                    if let Some(inner) = self.inner.get_mut(page) {
                                        if inner.is_initialized() {
                                            inner.event(
                                                ctx,
                                                &Event::Internal(InternalEvent::MouseLeave),
                                                data,
                                                env,
                                            );
                                        }
                                    }
                                    ctx.submit_notification_without_warning(
                                        ON_PAGE.with(page).to(Target::Global),
                                    );
                                    self.page_chain.push(page);
                                    tracing::trace!(
                                        "page pushed: {page:?}, chain={:?}",
                                        self.page_chain
                                    );
                                }
                                Some(AnimationType::PopZoom(page))
                                | Some(AnimationType::PopSlide(page))
                                    if !page.is_empty() && self.page_chain.contains(&page) =>
                                {
                                    ctx.submit_notification_without_warning(
                                        POP_PAGE.with(self.active_page),
                                    );
                                    self.active_page = page;
                                    if let Some(inner) = self.inner.get_mut(page) {
                                        if inner.is_initialized() {
                                            inner.event(
                                                ctx,
                                                &Event::Internal(InternalEvent::MouseLeave),
                                                data,
                                                env,
                                            );
                                        }
                                    }
                                    ctx.submit_notification_without_warning(
                                        ON_PAGE.with(page).to(Target::Global),
                                    );
                                    while self.page_chain.last().unwrap() != &page {
                                        self.page_chain.pop();
                                    }
                                    self.clean_unused_page();
                                    ctx.children_changed();
                                    ctx.request_update();
                                    tracing::trace!(
                                        "page popped to {page:?}, chain={:?}",
                                        self.page_chain
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                } else {
                    self.skip_first_anime_frame -= 1;
                }
                ctx.request_paint();
                ctx.request_anim_frame();
            }
        } else if let Event::Command(cmd) = event {
            if let Some(&page) = cmd.get(PUSH_PAGE) {
                tracing::debug!(
                    "PUSH_PAGE: page={page:?}, chain={:?}, active={:?}, queue={}",
                    self.page_chain,
                    self.active_page,
                    self.page_anime_queue.len()
                );
                self.load_page(page);
                ctx.children_changed();
                if self.inner.contains_key(page) {
                    if self.slide_page_animation {
                        self.page_anime_queue
                            .push_back(AnimationType::PushSlide(page));
                    } else {
                        self.page_anime_queue
                            .push_back(AnimationType::PushZoom(page));
                    }
                    ctx.request_update();
                    ctx.request_layout();
                    #[cfg(target_os = "macos")]
                    {
                        self.skip_first_anime_frame = 12;
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        self.skip_first_anime_frame = 1;
                    }
                    ctx.request_anim_frame();
                } else {
                    panic!("Can't find inner page called {page}");
                }
            } else if let Some(to_page) = cmd.get(QUERY_POP_PAGE) {
                tracing::debug!(
                    "QUERY_POP_PAGE: to={to_page:?}, chain={:?}, active={:?}, queue={}",
                    self.page_chain,
                    self.active_page,
                    self.page_anime_queue.len()
                );
                // 解析目标页面为链上的 &'static str，空字符串表示返回上一页。
                let target = if to_page.is_empty() {
                    if self.page_chain.len() > 1 {
                        Some(self.page_chain[self.page_chain.len() - 2])
                    } else {
                        tracing::trace!("WARNING: Back page invoked when the page is only one!");
                        None
                    }
                } else if let Some(page) =
                    self.page_chain.iter().copied().find(|p| p == to_page)
                {
                    Some(page)
                } else {
                    tracing::trace!("WARNING: Can't find page {to_page:?} in page chain");
                    None
                };
                if let Some(target) = target {
                    // 与官方 PageSwitcher 一致：动画进行中收到的弹栈请求直接丢弃，
                    // 不挂起延后执行。此前本地把请求存进 pending_pop、等队列排空再执行，
                    // 导致「启动页推入动画期间收到 pop(main)」被延后到 launch_error
                    // 页 push 之后才执行，把刚显示的错误页一并弹掉（错误页闪一下回主页）。
                    // 由此带来的「启动成功后停在启动页」与官方一致（OB-15，勿"修"）：官方
                    // 的成功 pop 在窗口等待之后才发出，无窗口环境同样不弹栈。
                    if self.page_anime_queue.is_empty() {
                        if target != self.active_page && self.page_chain.len() > 1 {
                            self.enqueue_pop(ctx, target);
                        }
                    } else {
                        tracing::debug!(
                            "animation in progress, drop pop request to {target:?}"
                        );
                    }
                }
            } else if let Some(to_page) = cmd.get(RESET_PAGE_TO) {
                tracing::debug!(
                    "RESET_PAGE_TO: to={to_page:?}, chain={:?}, active={:?}, queue={}",
                    self.page_chain,
                    self.active_page,
                    self.page_anime_queue.len()
                );
                let target = self.page_chain.iter().copied().find(|p| p == to_page);
                if let Some(target) = target {
                    if self.page_anime_queue.is_empty()
                        && target != self.active_page
                        && self.page_chain.len() > 1
                    {
                        // 动画已结束：与普通弹栈完全一致，保留弹出动画。
                        self.enqueue_pop(ctx, target);
                    } else {
                        // 动画进行中（典型为 starting 推入尚未完成）：立即撤销
                        // 所有未完成动画并截断页面链，不延后、不排队。
                        self.page_anime_queue.clear();
                        self.page_anime_timer = 0;
                        self.skip_first_anime_frame = 0;
                        let changed = !self.active_page.is_empty() && self.active_page != target;
                        if changed {
                            ctx.submit_notification_without_warning(
                                POP_PAGE.with(self.active_page),
                            );
                        }
                        self.active_page = target;
                        while self.page_chain.last() != Some(&target) {
                            self.page_chain.pop();
                        }
                        self.clean_unused_page();
                        if changed {
                            if let Some(inner) = self.inner.get_mut(target) {
                                if inner.is_initialized() {
                                    inner.event(
                                        ctx,
                                        &Event::Internal(InternalEvent::MouseLeave),
                                        data,
                                        env,
                                    );
                                }
                            }
                            ctx.submit_notification_without_warning(
                                ON_PAGE.with(target).to(Target::Global),
                            );
                        }
                        ctx.children_changed();
                        ctx.request_update();
                        ctx.request_paint();
                        tracing::debug!(
                            "RESET_PAGE_TO: chain truncated to {target:?}, chain={:?}",
                            self.page_chain
                        );
                    }
                }
            } else if let Some(&t) = cmd.get(SET_SLIDE_PAGE_ANIMATION) {
                if self.slide_page_animation != t {
                    self.slide_page_animation = t;
                    ctx.request_paint();
                }
            }
        }
        if let Some(last_page) = self.page_chain.last() {
            if last_page == &self.active_page {
                if let Some(inner) = self.inner.get_mut(self.active_page) {
                    if inner.is_initialized() && self.page_anime_queue.is_empty() {
                        inner.event(ctx, event, data, env);
                    }
                }
            }
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &T,
        env: &druid::Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            ctx.request_anim_frame();
        }
        for inner in self.inner.values_mut() {
            inner.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut druid::UpdateCtx, _old_data: &T, data: &T, env: &druid::Env) {
        for inner in self.inner.values_mut() {
            inner.update(ctx, data, env);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &T,
        env: &druid::Env,
    ) -> druid::Size {
        bc.debug_check("PageSwitcher");
        for inner in self.inner.values_mut() {
            inner.layout(ctx, bc, data, env);
            inner.set_origin(ctx, druid::Point::ZERO);
        }
        bc.max()
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &T, env: &druid::Env) {
        #[cfg(not(feature = "druid-ext"))]
        let page_mask = env.get(druid::theme::WINDOW_BACKGROUND_COLOR);
        let anime_timer = self.page_anime_timer.min(ANIMATION_TIME);
        let size = ctx.size();
        let transparent_rect = ctx.region().bounding_box();
        let transparent_size = transparent_rect.size();
        const SCALE_LEVEL: f64 = 0.15;
        match self.page_anime_queue.front() {
            Some(AnimationType::PushZoom(page)) => {
                let x = (anime_timer as f64) / ANIMATION_TIME as f64;
                let s = scl_gui_animation::tween::ease_out_expo(x);
                if s < 0.5 {
                    let s = s * 2.;
                    // 要退出的页面
                    if let Some(inner) = self.inner.get_mut(self.active_page) {
                        ctx.transform(
                            Affine::scale(1. + s * SCALE_LEVEL)
                                * Affine::translate((
                                    size.width * s * -SCALE_LEVEL / 2.,
                                    size.height * s * -SCALE_LEVEL / 2.,
                                )),
                        );
                        #[cfg(feature = "druid-ext")]
                        ctx.set_global_alpha(1. - s);
                        ctx.save().unwrap();
                        inner.paint(ctx, data, env);
                        #[cfg(not(feature = "druid-ext"))]
                        {
                            ctx.fill(
                                transparent_rect.to_owned(),
                                &PaintBrush::Color(page_mask.with_alpha(s)),
                            );
                        }
                        ctx.restore().unwrap();
                    }
                } else {
                    let s = (s - 0.5) * 2.;
                    // 要进入的页面
                    if let Some(inner) = self.inner.get_mut(page) {
                        ctx.transform(
                            Affine::scale((1. - SCALE_LEVEL) + s * SCALE_LEVEL)
                                * Affine::translate((
                                    size.width * (1. - s) * SCALE_LEVEL / 2.,
                                    size.height * (1. - s) * SCALE_LEVEL / 2.,
                                )),
                        );
                        #[cfg(feature = "druid-ext")]
                        ctx.set_global_alpha(s);
                        ctx.save().unwrap();
                        inner.paint(ctx, data, env);
                        #[cfg(not(feature = "druid-ext"))]
                        ctx.fill(
                            transparent_rect.to_owned(),
                            &PaintBrush::Color(page_mask.with_alpha(1. - s)),
                        );
                        ctx.restore().unwrap();
                    }
                }
            }
            Some(AnimationType::PopZoom(page)) => {
                let x = (anime_timer as f64) / ANIMATION_TIME as f64;
                let s = scl_gui_animation::tween::ease_out_expo(x);
                if s < 0.5 {
                    let s = s * 2.;
                    // 要退出的页面
                    if let Some(inner) = self.inner.get_mut(self.active_page) {
                        ctx.transform(
                            Affine::scale(1. - s * SCALE_LEVEL)
                                * Affine::translate((
                                    size.width * s * SCALE_LEVEL / 2.,
                                    size.height * s * SCALE_LEVEL / 2.,
                                )),
                        );
                        #[cfg(feature = "druid-ext")]
                        ctx.set_global_alpha(1. - s);
                        ctx.save().unwrap();
                        inner.paint(ctx, data, env);
                        #[cfg(not(feature = "druid-ext"))]
                        ctx.fill(
                            transparent_rect.to_owned(),
                            &PaintBrush::Color(page_mask.with_alpha(s)),
                        );
                        ctx.restore().unwrap();
                    }
                } else {
                    let s = (s - 0.5) * 2.;
                    // 要进入的页面
                    if let Some(inner) = self.inner.get_mut(page) {
                        ctx.transform(
                            Affine::scale(1. + (1. - s) * SCALE_LEVEL)
                                * Affine::translate((
                                    transparent_size.width * (1. - s) * -SCALE_LEVEL / 2.,
                                    transparent_size.height * (1. - s) * -SCALE_LEVEL / 2.,
                                )),
                        );
                        #[cfg(feature = "druid-ext")]
                        ctx.set_global_alpha(s);
                        ctx.save().unwrap();
                        inner.paint(ctx, data, env);
                        #[cfg(not(feature = "druid-ext"))]
                        ctx.fill(
                            transparent_rect.to_owned(),
                            &PaintBrush::Color(page_mask.with_alpha(1. - s)),
                        );
                        ctx.restore().unwrap();
                    }
                }
            }
            Some(AnimationType::PushSlide(page)) => {
                let x = (anime_timer as f64) / ANIMATION_TIME as f64;
                let s = scl_gui_animation::tween::ease_out_expo(x);
                if s < 0.5 {
                    let s = s * -2.;
                    // 要退出的页面
                    if let Some(inner) = self.inner.get_mut(self.active_page) {
                        ctx.transform(Affine::translate((size.width * s, 0.)));
                        // ctx.apply_global_transparency(1. - s);
                        inner.paint(ctx, data, env);
                        // ctx.fill(
                        //     transparent_rect.to_owned(),
                        //     &PaintBrush::Color(page_mask.with_alpha(s)),
                        // );
                    }
                } else {
                    let s = (s - 0.5) * 2.;
                    // 要进入的页面
                    if let Some(inner) = self.inner.get_mut(page) {
                        ctx.transform(Affine::translate((size.width * (1. - s), 0.)));
                        // ctx.apply_global_transparency(s);
                        inner.paint(ctx, data, env);
                        // ctx.fill(
                        //     transparent_rect.to_owned(),
                        //     &PaintBrush::Color(page_mask.with_alpha(1. - s)),
                        // );
                    }
                }
            }
            Some(AnimationType::PopSlide(page)) => {
                let x = (anime_timer as f64) / ANIMATION_TIME as f64;
                let s = scl_gui_animation::tween::ease_out_expo(x);
                if s < 0.5 {
                    let s = s * 2.;
                    // 要退出的页面
                    if let Some(inner) = self.inner.get_mut(self.active_page) {
                        ctx.transform(Affine::translate((size.width * s, 0.)));
                        inner.paint(ctx, data, env);
                    }
                } else {
                    let s = (s - 0.5) * 2.;
                    // 要进入的页面
                    if let Some(inner) = self.inner.get_mut(page) {
                        ctx.transform(Affine::translate((size.width * (s - 1.), 0.)));
                        inner.paint(ctx, data, env);
                    }
                }
            }
            Some(AnimationType::PushMoveUp(page)) => {
                if let Some(inner) = self.inner.get_mut(page) {
                    let x = ((anime_timer) as f64) / ANIMATION_TIME as f64;
                    let s = 1. - scl_gui_animation::tween::ease_out_expo(x);
                    ctx.transform(Affine::translate((0., s * size.height.min(100.))));
                    #[cfg(feature = "druid-ext")]
                    ctx.set_global_alpha(1. - s);
                    ctx.save().unwrap();
                    inner.paint(ctx, data, env);
                    #[cfg(not(feature = "druid-ext"))]
                    ctx.fill(
                        transparent_rect.to_owned(),
                        &PaintBrush::Color(page_mask.with_alpha(s)),
                    );
                    ctx.restore().unwrap();
                }
            }
            None => {
                if let Some(inner) = self.inner.get_mut(self.active_page) {
                    inner.paint(ctx, data, env);
                }
            }
        }
    }
}
