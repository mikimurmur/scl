//! 一个用于显示内存占用与游戏分配的内存条。

use std::time::Duration;

use druid::{
    piet::PaintBrush, BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, RenderContext, Size, TimerToken, UpdateCtx, Widget,
};
use scl_core::utils::{get_mem_status, MemoryStatus};
use scl_gui_animation::Spring;
use scl_gui_widgets::theme::color::{base, main};

/// 读取内存状态。设置了 `SCL_MOCK_MEM=总内存,可用内存`（单位 MB）时使用模拟值。
pub fn memory_status() -> MemoryStatus {
    if let Ok(value) = std::env::var("SCL_MOCK_MEM") {
        if let Some((max, free)) = value.split_once(',') {
            if let (Ok(max), Ok(free)) = (max.trim().parse(), free.trim().parse()) {
                return MemoryStatus { max, free };
            }
        }
    }
    get_mem_status()
}

/// 内存选择条
///
/// 数据是被选择的内存大小，单位为 MB。`None` 表示自动分配。
pub struct MemoryBar {
    width: f64,
    total: f64,
    timer: TimerToken,
    used_spring: Spring,
    game_spring: Spring,
}

impl Default for MemoryBar {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBar {
    /// 创建一个新的内存条
    pub fn new() -> Self {
        Self {
            width: 0.,
            total: memory_status().max as f64,
            timer: TimerToken::INVALID,
            used_spring: Spring::new(0.).with_damper(0.5),
            game_spring: Spring::new(0.).with_damper(0.5),
        }
    }

    fn update_targets(&mut self, data: &Option<usize>, width: f64) {
        if width <= 0. {
            return;
        }
        self.width = width;

        let status = memory_status();
        self.total = status.max as f64;

        let used = status.max.saturating_sub(status.free);
        self.used_spring
            .set_target(used as f64 / self.total * width);

        let game = match data {
            Some(value) => *value as f64,
            None => (status.free as f64 * 0.75).min(4000.),
        };
        self.game_spring.set_target(game / self.total * width);
    }
}

impl Widget<Option<usize>> for MemoryBar {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Option<usize>, _env: &Env) {
        match event {
            Event::Timer(token) if token == &self.timer => {
                self.update_targets(data, ctx.size().width);
                ctx.request_anim_frame();
                ctx.request_paint();
            }
            Event::AnimFrame(_) => {
                ctx.request_paint();
                if !self.used_spring.arrived() || !self.game_spring.arrived() {
                    ctx.request_anim_frame();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _: &Option<usize>, _: &Env) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old: &Option<usize>,
        _data: &Option<usize>,
        _env: &Env,
    ) {
        if self.timer == TimerToken::INVALID {
            self.timer = ctx.request_timer(Duration::from_secs(1));
        }
        ctx.request_anim_frame();
    }

    fn layout(
        &mut self,
        _: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Option<usize>,
        _env: &Env,
    ) -> Size {
        let size = bc.constrain(Size::new(100., 4.));
        self.update_targets(data, size.width);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Option<usize>, env: &Env) {
        let size = ctx.size();
        let width = size.width;
        let height = size.height;
        self.update_targets(data, width);

        let radius = height * 0.5;

        // 底槽
        let background = Size::new(width, height).to_rect().to_rounded_rect(radius);
        ctx.fill(background, &PaintBrush::Color(env.get(base::MEDIUM_LOW)));

        // 已占用内存
        let used = self.used_spring.position().min(width);
        let used_rect = druid::Rect::new(0., 0., used, height).to_rounded_rect(radius);
        ctx.fill(used_rect, &PaintBrush::Color(env.get(main::PRIMARY)));

        // 游戏分配内存
        let game_end = (used + self.game_spring.position()).min(width);
        let game_rect =
            druid::Rect::new(used, 0., game_end.max(used), height).to_rounded_rect(radius);
        let color = env.get(main::PRIMARY);
        let (r, g, b, _) = color.as_rgba8();
        ctx.fill(
            game_rect,
            &PaintBrush::Color(druid::Color::rgba8(r, g, b, 0x66)),
        );
    }
}
