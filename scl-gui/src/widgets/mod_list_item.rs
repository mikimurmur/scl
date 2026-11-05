//! 模组搜索结果中的一项。

use druid::{
    piet::{InterpolationMode, Text, TextLayoutBuilder},
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    RenderContext, Size, UpdateCtx, Widget,
};
use scl_gui_widgets::theme::color::{base, list, typography};

use crate::app_state::ModState;

/// 一个模组列表项
#[derive(Default)]
pub struct ModListItem {
    image: Option<druid::piet::PietImage>,
    image_key: Option<String>,
    /// 是否处于禁用态（父级 DisabledIf 传入；MR 搜索期间结果区置灰）
    disabled: bool,
    /// 是否允许 hover 高亮（文件页顶部信息行使用静态模式，不响应 hover）
    hover_enabled: bool,
}

impl ModListItem {
    /// 创建一个新的模组列表项
    pub fn new() -> Self {
        Self {
            image: None,
            image_key: None,
            disabled: false,
            hover_enabled: true,
        }
    }

    /// 创建文件页顶部的静态模组信息行：不响应 hover 高亮（官方同款）
    pub fn new_static() -> Self {
        Self {
            hover_enabled: false,
            ..Self::new()
        }
    }

    fn reload_icon(&mut self, ctx: &mut PaintCtx, data: &ModState) {
        let changed = self.image_key.as_deref() != Some(data.icon.as_str());
        if changed {
            self.image_key = Some(data.icon.clone());
            self.image = if data.icon.is_empty() {
                None
            } else {
                crate::utils::image::decode_image_buf(&data.icon)
                    .ok()
                    .and_then(|(width, height, pixels)| {
                        ctx.make_image(
                            width,
                            height,
                            &pixels,
                            druid::piet::ImageFormat::RgbaSeparate,
                        )
                        .ok()
                    })
            };
        }
    }
}

impl Widget<ModState> for ModListItem {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut ModState, _env: &Env) {
        if let Event::MouseDown(_) = event {
            ctx.set_active(true);
        } else if let Event::MouseUp(_) = event {
            if ctx.is_active() {
                ctx.set_active(false);
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _: &ModState, _: &Env) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        } else if let LifeCycle::DisabledChanged(disabled) = event {
            // 官方 ModListItem 的文字由 RawLabel 绘制：DisabledChanged 时
            // 文字切换为 Druid 禁用色 #A0A09A，图标保持不变。
            self.disabled = *disabled;
            ctx.request_paint();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &ModState, new: &ModState, _: &Env) {
        if !old.same(new) {
            self.image = None;
            self.image_key = None;
            // 图标是后台加载后写回 data 的，必须主动重绘，否则要等下一次鼠标事件才会出现。
            ctx.request_paint();
        }
    }

    fn layout(&mut self, _: &mut LayoutCtx, bc: &BoxConstraints, _: &ModState, _: &Env) -> Size {
        // 原版每个模组项高 48px：左侧 32px 图标 + 两行文字（标题/描述）。
        bc.constrain(Size::new(bc.max().width, 48.))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &ModState, env: &Env) {
        self.reload_icon(ctx, data);
        let rect = ctx.size().to_rect();
        if self.hover_enabled && !self.disabled && ctx.is_hot() {
            ctx.fill(rect, &env.get(list::LIST_LOW));
        }
        // 图标 32×32，左边距 8，上下留 8
        let icon_rect = druid::Rect::new(8., 8., 40., 40.).to_rounded_rect(4.);
        if let Some(image) = &self.image {
            ctx.draw_image(image, icon_rect.rect(), InterpolationMode::Bilinear);
        }
        let text_x = 52.;
        let title = env.get(typography::BODY);
        let desc = env.get(typography::CAPTION_ALT);
        // 禁用态（官方 RawLabel 行为）：文字统一使用 Druid 禁用色 #A0A09A。
        let disabled_color = env.get(druid::theme::DISABLED_TEXT_COLOR);
        let title_color = if self.disabled {
            disabled_color
        } else {
            env.get(base::HIGH)
        };
        let desc_color = if self.disabled {
            disabled_color
        } else {
            env.get(base::MEDIUM)
        };
        // 有中文名时标题显示「[中文名] 原名」
        // （如「[[AS] 苹果皮] AppleSkin」），拉不到译名时仅显示原名。
        let title_text = if data.cname.is_empty() {
            data.name.to_owned()
        } else {
            format!("[{}] {}", data.cname, data.name)
        };
        // 原版是单行裁切而不是换行：先生成不设宽度的文本布局，再裁剪到标题/描述区域。
        let text_rect = druid::Rect::new(text_x, 0., rect.width() - 8., rect.height());
        if let Ok(layout) = ctx
            .text()
            .new_text_layout(title_text)
            .font(title.family.clone(), title.size)
            .text_color(title_color)
            .build()
        {
            ctx.with_save(|ctx| {
                ctx.clip(text_rect);
                ctx.draw_text(&layout, (text_x, 5.));
            });
        }
        if let Ok(layout) = ctx
            .text()
            .new_text_layout(data.description.to_owned())
            .font(desc.family.clone(), desc.size)
            .text_color(desc_color)
            .build()
        {
            ctx.with_save(|ctx| {
                ctx.clip(text_rect);
                ctx.draw_text(&layout, (text_x, 26.));
            });
        }
    }
}
