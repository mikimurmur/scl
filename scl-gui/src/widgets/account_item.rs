//! 账户列表中的一项。

use druid::{
    kurbo::{BezPath, Rect, Shape},
    piet::{ImageFormat, InterpolationMode, Text, TextLayoutBuilder},
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    RenderContext, Size, UpdateCtx, Widget,
};
use scl_gui_widgets::theme::color::{alt, base, list, main, typography};

use crate::{
    app::{DELETE_AUTH, SWITCH_AUTH},
    core::AuthMethod,
    widgets::head_image::DEFAULT_HEAD,
};

/// 账户列表项的数据：(是否选中, 索引, 账户)
pub type AccountItemData = (bool, usize, AuthMethod);

const ROW_WIDTH: f64 = 300.;
const ROW_HEIGHT: f64 = 50.;
const HEAD_SIZE: f64 = 40.;
const HEAD_INSET: f64 = 1.175;
const DELETE_SIZE: f64 = 50.;

/// 账户列表项组件
#[derive(Default)]
pub struct AccountItem {
    image: Option<druid::piet::PietImage>,
    image_key: Vec<u8>,
    hat_image: Option<druid::piet::PietImage>,
    hat_key: Vec<u8>,
    delete_icon: Option<BezPath>,
    delete_hot: bool,
}

impl AccountItem {
    /// 创建一个新的账户列表项
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_head(&mut self, ctx: &mut PaintCtx, account: &AuthMethod) {
        let skin = if account.head_skin().len() == 8 * 8 * 4 {
            account.head_skin()
        } else {
            &DEFAULT_HEAD
        };
        if self.image_key != skin {
            self.image_key = skin.to_vec();
            self.image = ctx.make_image(8, 8, skin, ImageFormat::RgbaSeparate).ok();
        }

        let hat = account.hat_skin();
        if hat.len() == 8 * 8 * 4 {
            if self.hat_key != hat {
                self.hat_key = hat.to_vec();
                self.hat_image = ctx.make_image(8, 8, hat, ImageFormat::RgbaSeparate).ok();
            }
        } else {
            self.hat_key.clear();
            self.hat_image = None;
        }
    }

    fn delete_area(&self, size: Size) -> Rect {
        Rect::new(
            (size.width - DELETE_SIZE).max(0.),
            0.,
            size.width,
            size.height,
        )
    }
}

impl Widget<AccountItemData> for AccountItem {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AccountItemData, _: &Env) {
        match event {
            Event::MouseMove(event) => {
                let hot = self.delete_area(ctx.size()).contains(event.pos);
                if hot != self.delete_hot {
                    self.delete_hot = hot;
                    ctx.request_paint();
                }
            }
            Event::MouseDown(event) => {
                let hot = self.delete_area(ctx.size()).contains(event.pos);
                if hot != self.delete_hot {
                    self.delete_hot = hot;
                }
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(event) if ctx.is_active() => {
                ctx.set_active(false);
                if self.delete_area(ctx.size()).contains(event.pos) {
                    ctx.submit_command(druid::Command::new(
                        DELETE_AUTH,
                        data.1,
                        druid::Target::Global,
                    ));
                } else {
                    ctx.submit_command(druid::Command::new(
                        SWITCH_AUTH,
                        data.1,
                        druid::Target::Global,
                    ));
                }
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _: &AccountItemData,
        _: &Env,
    ) {
        if let LifeCycle::HotChanged(hot) = event {
            if !hot && self.delete_hot {
                self.delete_hot = false;
            }
            ctx.request_paint();
        }
    }

    fn update(&mut self, _: &mut UpdateCtx, old: &AccountItemData, new: &AccountItemData, _: &Env) {
        if old.2 != new.2 {
            self.image = None;
            self.image_key.clear();
            self.hat_image = None;
            self.hat_key.clear();
        }
    }

    fn layout(
        &mut self,
        _: &mut LayoutCtx,
        bc: &BoxConstraints,
        _: &AccountItemData,
        _: &Env,
    ) -> Size {
        // 原版 AccountItem 的 layout 尾部直接 constrain 到 300x50。
        bc.constrain(Size::new(ROW_WIDTH, ROW_HEIGHT))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AccountItemData, env: &Env) {
        let (selected, _, account) = data;
        self.ensure_head(ctx, account);
        let size = ctx.size();

        // 单选框：基准矩形 (15, 15)-(35, 35)。选中时向内缩 3，
        // 填充 alt::HIGH（浅色主题下为白色）并描 6 像素宽的强调色圆环；
        // 未选中时向内缩 0.5，填充 base::LOW 并描 1 像素 base::MEDIUM。
        let radio_center = (25., 25.);
        let circle = if *selected {
            let rect = Rect::new(15., 15., 35., 35.).inset(-3.);
            druid::kurbo::Circle::new(radio_center, rect.width() / 2.)
        } else {
            let rect = Rect::new(15., 15., 35., 35.).inset(-0.5);
            druid::kurbo::Circle::new(radio_center, rect.width() / 2.)
        };
        if *selected {
            ctx.fill(circle, &env.get(alt::HIGH));
            ctx.stroke(circle, &env.get(main::PRIMARY), 6.);
        } else {
            ctx.fill(circle, &env.get(base::LOW));
            ctx.stroke(circle, &env.get(base::MEDIUM), 1.);
        }

        // 头像：40x40，左上角 (51, 5)，向内缩 1.175 后使用最近邻缩放绘制；
        // 帽子贴图与未缩放的 40x40 矩形对齐。
        let head_rect = Rect::new(51., 5., 51. + HEAD_SIZE, 5. + HEAD_SIZE);
        let head_rect = head_rect.inset(-HEAD_INSET);
        if let Some(image) = &self.image {
            ctx.draw_image(image, head_rect, InterpolationMode::NearestNeighbor);
        } else {
            ctx.fill(head_rect, &env.get(main::PRIMARY).with_alpha(0.35));
        }
        if let Some(hat) = &self.hat_image {
            ctx.draw_image(
                hat,
                Rect::new(51., 5., 51. + HEAD_SIZE, 5. + HEAD_SIZE),
                InterpolationMode::NearestNeighbor,
            );
        }

        // 名称与账户类型
        let font_color = env.get(typography::FONT_COLOR);
        let name_font = env.get(typography::BASE);
        if let Ok(layout) = ctx
            .text()
            .new_text_layout(account.display_name())
            .font(name_font.family, 13.)
            .text_color(font_color)
            .build()
        {
            ctx.draw_text(&layout, (107., 10.5));
        }
        let type_font = env.get(typography::CAPTION);
        if let Ok(layout) = ctx
            .text()
            .new_text_layout(account.type_name())
            .font(type_font.family, 11.)
            .text_color(font_color)
            .build()
        {
            ctx.draw_text(&layout, (107., 25.5));
        }

        // 右侧 50x50 的平面图标按钮，图标为 16x16 的删除图标。
        let delete_area = self.delete_area(size);
        if self.delete_hot {
            let bg = if ctx.is_active() {
                env.get(base::LOW)
            } else {
                env.get(list::LIST_LOW)
            };
            ctx.fill(delete_area.to_rounded_rect(4.), &bg);
        }
        if self.delete_icon.is_none() {
            let svg = env.get(&crate::theme::icons::DELETE.0);
            self.delete_icon = BezPath::from_svg(&svg).ok();
        }
        if let Some(path) = &self.delete_icon {
            let icon_size = path.bounding_box().size();
            let offset =
                delete_area.origin().to_vec2() + ((delete_area.size() - icon_size) / 2.).to_vec2();
            ctx.with_save(|ctx| {
                ctx.transform(druid::Affine::translate(offset));
                ctx.fill_even_odd(path, &env.get(base::HIGH));
            });
        }
    }
}
