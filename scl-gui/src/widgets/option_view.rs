//! 稳定的 Option 视图组件。

use druid::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Size, UpdateCtx, Widget, WidgetPod,
};

/// 与 `druid::widget::Maybe` 语义相同，但 child 稳定、无运行期重建。
pub struct OptionView<T> {
    some: WidgetPod<T, Box<dyn Widget<T>>>,
    none: WidgetPod<(), Box<dyn Widget<()>>>,
    is_some: bool,
}

impl<T: Data + Default> OptionView<T> {
    /// 创建视图：`some` 用于 `Some(_)`，`none` 用于 `None`
    pub fn new(some: impl Widget<T> + 'static, none: impl Widget<()> + 'static) -> Self {
        Self {
            some: WidgetPod::new(Box::new(some)),
            none: WidgetPod::new(Box::new(none)),
            is_some: false,
        }
    }
}

impl<T: Data + Default> Widget<Option<T>> for OptionView<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Option<T>, env: &Env) {
        let is_some = data.is_some();
        if is_some != self.is_some {
            self.is_some = is_some;
            ctx.children_changed();
        }
        match data {
            Some(d) => self.some.event(ctx, event, d, env),
            None => self.none.event(ctx, event, &mut (), env),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &Option<T>, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            // 关键：两个 child 都先收到 WidgetAdded（some 在 None 时用默认值初始化），
            // 之后任何 BuildFocusChain 都不会早于 WidgetAdded。
            let default = T::default();
            let some_data = data.as_ref().unwrap_or(&default);
            self.some.lifecycle(ctx, event, some_data, env);
            self.none.lifecycle(ctx, event, &(), env);
            self.is_some = data.is_some();
            return;
        }
        let is_some = data.is_some();
        if is_some != self.is_some {
            self.is_some = is_some;
            ctx.children_changed();
        }
        match data {
            Some(d) => self.some.lifecycle(ctx, event, d, env),
            None => self.none.lifecycle(ctx, event, &(), env),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &Option<T>, data: &Option<T>, env: &Env) {
        let is_some = data.is_some();
        if is_some != self.is_some {
            self.is_some = is_some;
            ctx.children_changed();
        }
        match data {
            Some(d) => self.some.update(ctx, d, env),
            None => self.none.update(ctx, &(), env),
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Option<T>,
        env: &Env,
    ) -> Size {
        match data {
            Some(d) => {
                let size = self.some.layout(ctx, bc, d, env);
                self.some.set_origin(ctx, Point::ORIGIN);
                size
            }
            None => {
                let size = self.none.layout(ctx, bc, &(), env);
                self.none.set_origin(ctx, Point::ORIGIN);
                size
            }
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Option<T>, env: &Env) {
        match data {
            Some(d) => self.some.paint(ctx, d, env),
            None => self.none.paint(ctx, &(), env),
        }
    }
}
