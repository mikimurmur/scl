//! 显示账户头像的组件。

use std::marker::PhantomData;

use druid::{
    piet::{ImageFormat, InterpolationMode, RenderContext},
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size,
    UpdateCtx, Widget,
};

use crate::{config::AccountData, core::AuthMethod};

/// 没有皮肤时显示的默认头像
pub(crate) const DEFAULT_HEAD: [u8; 8 * 8 * 4] = [
    51, 36, 17, 255, 51, 36, 17, 255, 63, 42, 21, 255, 63, 42, 21, 255, 63, 42, 21, 255, 63, 42,
    21, 255, 51, 36, 17, 255, 43, 30, 13, 255, 36, 24, 8, 255, 51, 36, 17, 255, 51, 36, 17, 255,
    63, 42, 21, 255, 63, 42, 21, 255, 51, 36, 17, 255, 63, 42, 21, 255, 51, 36, 17, 255, 43, 30,
    13, 255, 155, 99, 73, 255, 179, 121, 94, 255, 183, 131, 107, 255, 179, 121, 94, 255, 170, 114,
    89, 255, 155, 99, 73, 255, 52, 37, 18, 255, 155, 99, 73, 255, 170, 114, 89, 255, 179, 121, 94,
    255, 179, 121, 94, 255, 170, 114, 89, 255, 170, 114, 89, 255, 170, 114, 89, 255, 155, 99, 73,
    255, 170, 114, 89, 255, 255, 255, 255, 255, 82, 61, 137, 255, 170, 114, 89, 255, 155, 99, 73,
    255, 82, 61, 137, 255, 255, 255, 255, 255, 170, 114, 89, 255, 155, 99, 73, 255, 170, 114, 89,
    255, 170, 114, 89, 255, 106, 64, 48, 255, 106, 64, 48, 255, 170, 114, 89, 255, 170, 114, 89,
    255, 155, 99, 73, 255, 144, 89, 63, 255, 143, 94, 62, 255, 73, 37, 16, 255, 119, 66, 53, 255,
    119, 66, 53, 255, 66, 29, 10, 255, 143, 94, 62, 255, 129, 83, 57, 255, 148, 96, 62, 255, 129,
    83, 57, 255, 66, 29, 10, 255, 73, 37, 16, 255, 66, 29, 10, 255, 73, 37, 16, 255, 129, 83, 57,
    255, 143, 94, 62, 255,
];

/// 一个显示玩家皮肤头部的组件
pub struct HeadImage<T> {
    image: Option<druid::piet::PietImage>,
    image_key: Vec<u8>,
    phantom: PhantomData<T>,
}

impl<T> Default for HeadImage<T> {
    fn default() -> Self {
        Self {
            image: None,
            image_key: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<T> HeadImage<T> {
    /// 创建一个新的头像组件
    pub fn new() -> Self {
        Self::default()
    }

    fn paint_skin(&mut self, ctx: &mut PaintCtx, skin: &[u8], size: Size, env: &Env) {
        // 官方头像控件整体为 85x85，内部皮肤图像是左上角对齐的 80x80，
        // 投影则绘制在 (2.5, 2.5) 到 (82.5, 82.5) 的位置，形成右下偏移的阴影。
        let image_rect = druid::Rect::new(0., 0., size.width * 0.941176, size.height * 0.941176);
        let shadow_rect = druid::Rect::new(
            size.width * 0.029412,
            size.height * 0.029412,
            size.width * 0.970588,
            size.height * 0.970588,
        );
        let skin = if skin.len() == 8 * 8 * 4 {
            skin
        } else {
            &DEFAULT_HEAD
        };
        if self.image.is_none() || self.image_key != skin {
            self.image_key = skin.to_vec();
            self.image = ctx.make_image(8, 8, skin, ImageFormat::RgbaSeparate).ok();
        }
        ctx.blurred_rect(
            shadow_rect,
            8.0,
            &env.get(scl_gui_widgets::theme::color::border::TRANSIENT),
        );
        if let Some(image) = &self.image {
            ctx.draw_image(image, image_rect, InterpolationMode::NearestNeighbor);
            return;
        }
        let color = env
            .get(scl_gui_widgets::theme::color::main::PRIMARY)
            .with_alpha(0.35);
        ctx.fill(image_rect.to_rounded_rect(6.), &color);
    }
}

impl Widget<AuthMethod> for HeadImage<AuthMethod> {
    fn event(&mut self, _: &mut EventCtx, _: &Event, _: &mut AuthMethod, _: &Env) {}

    fn lifecycle(&mut self, _: &mut LifeCycleCtx, _: &LifeCycle, _: &AuthMethod, _: &Env) {}

    fn update(&mut self, _: &mut UpdateCtx, _: &AuthMethod, _: &AuthMethod, _: &Env) {
        self.image = None;
        self.image_key.clear();
    }

    fn layout(&mut self, _: &mut LayoutCtx, bc: &BoxConstraints, _: &AuthMethod, _: &Env) -> Size {
        bc.constrain(Size::new(85., 85.))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AuthMethod, env: &Env) {
        let skin = data.head_skin().to_vec();
        let size = ctx.size();
        self.paint_skin(ctx, &skin, size, env);
    }
}

impl Widget<AccountData> for HeadImage<AccountData> {
    fn event(&mut self, _: &mut EventCtx, _: &Event, _: &mut AccountData, _: &Env) {}

    fn lifecycle(&mut self, _: &mut LifeCycleCtx, _: &LifeCycle, _: &AccountData, _: &Env) {}

    fn update(&mut self, _: &mut UpdateCtx, _: &AccountData, _: &AccountData, _: &Env) {
        self.image = None;
        self.image_key.clear();
    }

    fn layout(&mut self, _: &mut LayoutCtx, bc: &BoxConstraints, _: &AccountData, _: &Env) -> Size {
        bc.constrain(Size::new(85., 85.))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AccountData, env: &Env) {
        let skin = data
            .accounts
            .get(data.selected)
            .map(|x| x.head_skin().to_vec())
            .unwrap_or_default();
        let size = ctx.size();
        self.paint_skin(ctx, &skin, size, env);
    }
}
