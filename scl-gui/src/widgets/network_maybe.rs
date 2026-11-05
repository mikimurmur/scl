//! 一个在数据加载中或加载失败时显示提示的包装组件。

use druid::{
    piet::{Text, TextLayout, TextLayoutBuilder},
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    RenderContext, Size, UpdateCtx, Widget, WidgetPod,
};

use crate::app_state::NetworkData;

/// 网络数据包装组件
pub struct NetworkMaybe<T> {
    inner: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T: Data> NetworkMaybe<T> {
    /// 创建一个新的包装组件
    pub fn new(inner: impl Widget<T> + 'static) -> Self {
        Self {
            inner: WidgetPod::new(Box::new(inner)),
        }
    }
}

impl<T: Data + Default> Widget<NetworkData<T>> for NetworkMaybe<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut NetworkData<T>, env: &Env) {
        if let Some(value) = data.data.as_mut() {
            self.inner.event(ctx, event, value, env);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &NetworkData<T>,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            // WidgetPod 必须先收到 WidgetAdded 才能处理后续事件。数据尚未加载时
            // 使用一个占位数据完成初始化，等真实数据到达后再通过 update 传入。
            let placeholder;
            let value = match data.data.as_ref() {
                Some(value) => value,
                None => {
                    placeholder = T::default();
                    &placeholder
                }
            };
            self.inner.lifecycle(ctx, event, value, env);
            return;
        }
        if let Some(value) = data.data.as_ref() {
            self.inner.lifecycle(ctx, event, value, env);
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &NetworkData<T>,
        data: &NetworkData<T>,
        env: &Env,
    ) {
        match (old_data.data.as_ref(), data.data.as_ref()) {
            (Some(_), Some(new)) => self.inner.update(ctx, new, env),
            (None, Some(new)) => {
                ctx.children_changed();
                self.inner.update(ctx, new, env);
            }
            (Some(_), None) => ctx.children_changed(),
            (None, None) => {}
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &NetworkData<T>,
        env: &Env,
    ) -> Size {
        if let Some(value) = data.data.as_ref() {
            let size = self.inner.layout(ctx, bc, value, env);
            self.inner.set_origin(ctx, (0., 0.).into());
            size
        } else {
            bc.constrain(Size::new(bc.max().width, 120.))
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &NetworkData<T>, env: &Env) {
        if let Some(value) = data.data.as_ref() {
            self.inner.paint(ctx, value, env);
            return;
        }
        let text = if let Some(error) = &data.error {
            format!("加载失败：{error}")
        } else {
            "正在加载……".to_owned()
        };
        let font = env.get(scl_gui_widgets::theme::color::typography::BODY);
        let color = env.get(scl_gui_widgets::theme::color::base::MEDIUM);
        if let Ok(layout) = ctx
            .text()
            .new_text_layout(text)
            .font(font.family, font.size)
            .text_color(color)
            .build()
        {
            let size = layout.size();
            let origin = (
                (ctx.size().width - size.width) / 2.,
                (ctx.size().height - size.height) / 2.,
            );
            ctx.draw_text(&layout, origin);
        }
    }
}
