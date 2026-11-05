//! Java 运行时列表中的一项。
//!
//! 官方 `JavaItem` 由两个 `Label<JavaRuntimeInfo>` 子组件（名称 / 路径）加
//! 一个 SVG 图标组成；`layout` 按两个 Label 的实际高度计算行高
//! `max(50, 13 + name_h + path_h)`，其中 13 是上下总 padding，不是行间距。
//! 50/100 是 Label 的宽度收缩量（PATH 默认项 50，检测项 100）。

use druid::{
    kurbo::BezPath,
    widget::{Label, LineBreaking},
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, MouseButton,
    PaintCtx, Point, RenderContext, Size, Target, UpdateCtx, Widget, WidgetPod,
};
use scl_gui_widgets::theme::{
    color::{self, base, typography},
    icons::IconKeyPair,
};

use crate::{
    app::{
        java::{is_default_path, runtime_display_name},
        DELETE_JAVA, SELECT_JAVA,
    },
    config::JavaRuntimeInfo,
    theme::icons::{JAVA_16_FULLED, JAVA_8_FULLED},
};

/// 路径没有空格，Piet 的 WordWrap 不会在 `/` 或 `\` 处断行。
/// 这里只在显示字符串里按分隔符插入换行，配置中的真实路径不变。
fn display_path(path: &str) -> String {
    // 检测项官方 Label 宽度收缩 100，13px 字体下约 28 字符一行。
    const MAX_LINE_CHARS: usize = 28;
    let mut output = String::with_capacity(path.len() + path.len() / MAX_LINE_CHARS);
    let mut line_len = 0;
    for ch in path.chars() {
        output.push(ch);
        line_len += 1;
        if (ch == '/' || ch == '\\') && line_len >= MAX_LINE_CHARS {
            output.push('\n');
            line_len = 0;
        }
    }
    output
}

/// Java 运行时列表项
pub struct JavaItem {
    icon_key: IconKeyPair,
    icon_path: BezPath,
    name: WidgetPod<JavaRuntimeInfo, Label<JavaRuntimeInfo>>,
    path: WidgetPod<JavaRuntimeInfo, Label<JavaRuntimeInfo>>,
}

impl Default for JavaItem {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaItem {
    /// 创建一个新的 Java 运行时列表项
    pub fn new() -> Self {
        let name = Label::dynamic(|data: &JavaRuntimeInfo, _: &Env| runtime_display_name(data))
            .with_font(typography::BASE)
            .with_text_size(14.)
            .with_text_color(color::base::HIGH)
            .with_line_break_mode(LineBreaking::WordWrap);
        let path = Label::dynamic(|data: &JavaRuntimeInfo, _: &Env| display_path(&data.path))
            .with_font(typography::CAPTION_ALT)
            .with_text_color(color::base::MEDIUM)
            .with_line_break_mode(LineBreaking::WordWrap);
        Self {
            icon_key: JAVA_8_FULLED,
            icon_path: BezPath::new(),
            name: WidgetPod::new(name),
            path: WidgetPod::new(path),
        }
    }

    fn icon_key_for(data: &JavaRuntimeInfo) -> IconKeyPair {
        if data.main_version >= 16 {
            JAVA_16_FULLED
        } else {
            JAVA_8_FULLED
        }
    }

    /// 根据 Java 运行时信息重新载入图标
    fn reload_icons(&mut self, data: &JavaRuntimeInfo, env: &Env) {
        self.icon_key = Self::icon_key_for(data);
        let path = env.get(&self.icon_key.0);
        self.icon_path = BezPath::from_svg(&path).unwrap_or_default();
    }
}

impl Widget<JavaRuntimeInfo> for JavaItem {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut JavaRuntimeInfo, env: &Env) {
        match event {
            Event::MouseDown(mouse) => {
                if mouse.button == MouseButton::Right {
                    // 官方对非 PATH 默认项发送整条 JavaRuntimeInfo 的 delete-java；
                    // PATH 默认项（java/javaw）不允许删除。
                    if !is_default_path(&data.path) {
                        ctx.submit_command(druid::Command::new(
                            DELETE_JAVA,
                            data.clone(),
                            Target::Global,
                        ));
                        ctx.set_handled();
                    }
                } else {
                    ctx.set_active(true);
                }
            }
            Event::MouseUp(mouse) if mouse.button == MouseButton::Left && ctx.is_active() => {
                ctx.set_active(false);
                ctx.request_paint();
                ctx.submit_command(druid::Command::new(
                    SELECT_JAVA,
                    data.clone(),
                    Target::Global,
                ));
                ctx.set_handled();
            }
            _ => {}
        }
        self.name.event(ctx, event, data, env);
        self.path.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &JavaRuntimeInfo,
        env: &Env,
    ) {
        match event {
            LifeCycle::WidgetAdded => self.reload_icons(data, env),
            LifeCycle::HotChanged(_) => ctx.request_paint(),
            _ => {}
        }
        self.name.lifecycle(ctx, event, data, env);
        self.path.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old: &JavaRuntimeInfo,
        new: &JavaRuntimeInfo,
        env: &Env,
    ) {
        if old.main_version != new.main_version {
            self.reload_icons(new, env);
        } else if ctx.env_key_changed(&self.icon_key.0) {
            self.icon_path = BezPath::from_svg(&env.get(&self.icon_key.0)).unwrap_or_default();
        }
        self.name.update(ctx, new, env);
        self.path.update(ctx, new, env);
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &JavaRuntimeInfo,
        env: &Env,
    ) -> Size {
        let left = if is_default_path(&data.path) {
            50.
        } else {
            100.
        };
        let text_bc = bc
            .shrink_max_width_to((bc.max().width - left).max(10.))
            .loosen();
        let name_size = self.name.layout(ctx, &text_bc, data, env);
        let path_size = self.path.layout(ctx, &text_bc, data, env);
        let text_height = name_size.height + path_size.height;
        let height = (text_height + 13.).max(50.);
        let start_y = (height - text_height).max(0.) / 2.;
        self.name.set_origin(ctx, Point::new(50., start_y));
        self.path
            .set_origin(ctx, Point::new(50., start_y + name_size.height));
        bc.constrain(Size::new(bc.max().width, height))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &JavaRuntimeInfo, env: &Env) {
        let size = ctx.size();
        let height = size.height;
        // 原版没有色块底：8/16 徽标本身就是黑/白色的 filled SVG。
        // 默认项图标约 20x16，左内边距 14；图标在行内垂直居中。
        let icon_top = (height - 16.) / 2.;
        ctx.with_save(|ctx| {
            ctx.transform(druid::kurbo::Affine::translate((14., icon_top)));
            ctx.fill_even_odd(&self.icon_path, &env.get(base::HIGH));
        });
        self.name.paint(ctx, data, env);
        self.path.paint(ctx, data, env);
    }
}
