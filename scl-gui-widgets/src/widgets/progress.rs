use druid::{
    kurbo::BezPath,
    piet::{PaintBrush, StrokeStyle},
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    RenderContext, Size, UpdateCtx, Widget,
};

use crate::theme::color as theme;

/// 一个仿 WinUI 3 的进度指示圈，仅支持无界进度动画
pub struct ProgressRing {
    indeterminate: bool,
    indeterminate_time: f64,
}

impl ProgressRing {
    /// 创建一个进度指示圈
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ProgressRing {
    fn default() -> Self {
        Self {
            indeterminate: true,
            indeterminate_time: 0.,
        }
    }
}

impl<T: Data> Widget<T> for ProgressRing {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, _env: &Env) {
        if let Event::AnimFrame(t) = event {
            self.indeterminate_time += *t as f64 / 1_000_000_000.;
            if self.indeterminate {
                ctx.request_paint();
                ctx.request_anim_frame();
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &T, _env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            if self.indeterminate {
                ctx.request_anim_frame();
            }
        }
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, _env: &Env) -> Size {
        bc.debug_check("ProgressRing");
        bc.constrain((60., 60.))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        let front = env.get(theme::main::SECONDARY);

        let t = self.indeterminate_time;

        let size = ctx.size().width.min(ctx.size().height).max(0.);
        let scale = size / 60.;
        let width = 8. * scale; // 线条粗细
        let radius = (size - width) / 2.; // 半径
        let center = size / 2.;
        let speed = std::f64::consts::PI * 3.;
        let target = t * -speed;

        // 曲线的长度
        let l = ((t * std::f64::consts::PI) % std::f64::consts::TAU - std::f64::consts::PI).abs(); // ((t * -std::f64::consts::PI).sin() + 1.) / 2.; // ((t * -std::f64::consts::PI).sin() + 1.) / 2.;
        let thera = l; // * std::f64::consts::PI;

        // 计算切线长度
        let rad = thera.cos().acos();
        let tangent_scale = 4. / 3. * (1. - (rad / 2.).cos()) / (rad / 2.).sin();
        let tangent_len = tangent_scale * radius;

        // 计算各个点的正弦余弦
        let start = target - thera / 2.;
        let end = target + thera / 2.;
        let (sx, sy) = start.sin_cos();
        let (p1ox, p1oy) = (start + std::f64::consts::FRAC_PI_2).sin_cos();
        let (ex, ey) = end.sin_cos();
        let (p2ox, p2oy) = (end - std::f64::consts::FRAC_PI_2).sin_cos();

        // 点的坐标（以控件中心为圆心）
        let start_point = (center + sx * radius, center + sy * radius);
        let end_point = (center + ex * radius, center + ey * radius);
        let control_1_point = (
            center + sx * radius + p1ox * tangent_len,
            center + sy * radius + p1oy * tangent_len,
        );
        let control_2_point = (
            center + ex * radius + p2ox * tangent_len,
            center + ey * radius + p2oy * tangent_len,
        );

        let mut path = BezPath::new();
        path.move_to(start_point);
        path.curve_to(control_1_point, control_2_point, end_point);

        // 绘制
        let region = ctx.region().bounding_box();

        ctx.clip(region);
        // 官方路径圆心在 (radius, radius)，绘制时平移 (4,4)，最终圆心
        // 落在控件中心 (30,30)；这里直接把圆心算在控件中心，无需平移。
        ctx.stroke_styled(
            path,
            &PaintBrush::Color(front),
            width,
            &StrokeStyle {
                line_cap: druid::piet::LineCap::Round,
                ..Default::default()
            },
        );
    }

    fn id(&self) -> Option<druid::WidgetId> {
        None
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
