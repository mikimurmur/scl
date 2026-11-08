//! 一个开始游戏按钮，修改自 [`druid::widget::Button`]
//!
//! 版本名可能非常长：这里按按钮可用宽度自动换行（最多 2 行），
//! 超出行数时在末尾加省略号，并在悬停时用提示框显示完整版本名。

use druid::{
    debug_state::DebugState,
    piet::{PietText, PietTextLayout, Text, TextLayout, TextLayoutBuilder},
    widget::{prelude::*, Click, ControllerHost, Label, LabelText},
    Affine, Color, Data, Env, FontDescriptor, Insets, KeyOrValue, Point, RenderContext, Selector,
    Size, Target, Vec2,
};

use super::label;
use crate::theme::color as theme;

// the minimum padding added to a button.
// NOTE: these values are chosen to match the existing look of TextBox; these
// should be reevaluated at some point.
const LABEL_INSETS: Insets = Insets::uniform_xy(8., 2.);
/// 版本名最多显示 2 行；再多就用省略号，悬停提示显示全名。
const MAX_SUB_LABEL_LINES: usize = 2;

/// 悬停被截断的版本名时通知根组件显示完整名称提示。
pub const VERSION_NAME_TOOLTIP: Selector<Option<String>> =
    Selector::new("net.stevexmh.scl.version-name-tooltip");

/// A button with a text label.
pub struct StartGameButton<T> {
    label: Label<T>,
    sub_label: WrapLabel<T>,
    label_size: Size,
    sub_label_size: Size,
    accent: bool,
}

impl<T: Data> StartGameButton<T> {
    /// Create a new button with a text label.
    ///
    /// Use the [`.on_click`] method to provide a closure to be called when the
    /// button is clicked.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::widget::Button;
    ///
    /// let button = Button::new("Increment").on_click(|_ctx, data: &mut u32, _env| {
    ///     *data += 1;
    /// });
    /// ```
    ///
    /// [`.on_click`]: #method.on_click
    pub fn new(text: impl Into<LabelText<T>>) -> StartGameButton<T> {
        StartGameButton {
            label: label::new("启动游戏").with_font(theme::typography::BODY),
            sub_label: WrapLabel::new(text.into()),
            label_size: Size::ZERO,
            sub_label_size: Size::ZERO,
            accent: false,
        }
    }

    /// Construct a new dynamic button.
    ///
    /// The contents of this button are generated from the data using a closure.
    ///
    /// This is provided as a convenience; a closure can also be passed to [`new`],
    /// but due to limitations of the implementation of that method, the types in
    /// the closure need to be annotated, which is not true for this method.
    ///
    /// # Examples
    ///
    /// The following are equivalent:
    ///
    /// ```
    /// use druid::Env;
    /// use druid::widget::Button;
    /// let button1: Button<u32> = Button::new(|data: &u32, _: &Env| format!("total is {}", data));
    /// let button2: Button<u32> = Button::dynamic(|data, _| format!("total is {}", data));
    /// ```
    ///
    /// [`new`]: #method.new
    pub fn dynamic(text: impl Fn(&T, &Env) -> String + 'static) -> Self {
        let text: LabelText<T> = text.into();
        StartGameButton::new(text)
    }

    /// Provide a closure to be called when this button is clicked.
    pub fn on_click(
        self,
        f: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> ControllerHost<Self, Click<T>> {
        ControllerHost::new(self, Click::new(f))
    }

    /// Use accent color as button color
    pub fn set_accent(&mut self, value: bool) {
        self.accent = value;
        self.label.set_text_color(if self.accent {
            theme::chrome::WHITE_HIGH
        } else {
            theme::typography::FONT_COLOR
        });
        self.sub_label.set_text_color(if self.accent {
            theme::border::EDGE_HIGHTLIGHT
        } else {
            theme::typography::FONT_COLOR
        });
    }

    /// Builder style to use accent color as button color
    pub fn with_accent(mut self, value: bool) -> Self {
        self.set_accent(value);
        self
    }
}

impl<T: Data> Widget<T> for StartGameButton<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) if ctx.is_active() => {
                ctx.set_active(false);
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::HotChanged(hot) = event {
            // 提示框画在根组件上，避免被按钮自身的绘制区域裁掉。
            let tooltip = if *hot && self.sub_label.truncated() {
                Some(self.sub_label.full_text().to_string())
            } else {
                None
            };
            ctx.submit_command(VERSION_NAME_TOOLTIP.with(tooltip).to(Target::Global));
            ctx.request_paint();
        }
        self.label.lifecycle(ctx, event, data, env);
        self.sub_label.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.label.update(ctx, old_data, data, env);
        self.sub_label.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Button");
        let padding = Size::new(LABEL_INSETS.x_value(), LABEL_INSETS.y_value());
        let inner_width = (bc.max().width - padding.width).max(10.);
        let inner_height = (bc.max().height - padding.height).max(10.);
        let text_bc = BoxConstraints::new(Size::ZERO, Size::new(inner_width, inner_height));
        self.label_size = self.label.layout(ctx, &text_bc, data, env);

        // 版本名可以使用剩余高度（最多 2 行），不会再被按钮上下硬裁。
        let sub_height = (inner_height - self.label_size.height).max(0.);
        let sub_bc = BoxConstraints::new(Size::ZERO, Size::new(inner_width, sub_height));
        self.sub_label_size = self.sub_label.layout(ctx, &sub_bc, data, env);

        let text_height = self.label_size.height + self.sub_label_size.height;
        ctx.set_baseline_offset(self.label_size.height);
        bc.constrain(Size::new(
            (self.label_size.width + padding.width).max(bc.min().width),
            (text_height + padding.height).max(32.).max(bc.min().height),
        ))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let is_hot = ctx.is_hot();
        let is_active = ctx.is_active();
        let is_disabled = ctx.is_disabled();
        let size = ctx.size();

        super::common::print_common_button(
            (self.accent, false, is_active, is_hot, is_disabled),
            ctx,
            size,
            env,
        );

        let text_height = self.label_size.height + self.sub_label_size.height;
        let top = ((size.height - text_height) / 2.).max(0.);
        let label_offset = Vec2::new((size.width - self.label_size.width) / 2., top);
        ctx.with_save(|ctx| {
            ctx.transform(Affine::translate(label_offset));
            self.label.paint(ctx, data, env);
        });

        let sub_offset = Vec2::new(
            (size.width - self.sub_label_size.width) / 2.,
            top + self.label_size.height,
        );
        ctx.with_save(|ctx| {
            // 换行后仍可能因字体度量略超宽，裁剪限定在本行文本区域，
            // 但这不会再造成左右两端同时被切（省略号已由 WrapLabel 处理）。
            ctx.clip(druid::Rect::new(
                sub_offset.x,
                sub_offset.y,
                sub_offset.x + self.sub_label_size.width,
                sub_offset.y + self.sub_label_size.height,
            ));
            ctx.transform(Affine::translate(sub_offset));
            self.sub_label.paint(ctx, data, env);
        });
    }

    fn debug_state(&self, _data: &T) -> DebugState {
        DebugState {
            display_name: "StartGameButton".into(),
            main_value: format!(
                "label={:?} sub={:?} lines={} truncated={}",
                self.label_size,
                self.sub_label_size,
                self.sub_label.line_count(),
                self.sub_label.truncated(),
            ),
            ..Default::default()
        }
    }
}

/// 按可用宽度自动换行、超出行数时末尾省略的标签。
struct WrapLabel<T> {
    text: LabelText<T>,
    font: KeyOrValue<FontDescriptor>,
    color: KeyOrValue<Color>,
    cache: Option<WrapCache>,
}

struct WrapCache {
    source: String,
    max_width: f64,
    layout: PietTextLayout,
    truncated: bool,
}

impl<T: Data> WrapLabel<T> {
    fn new(text: LabelText<T>) -> Self {
        Self {
            text,
            font: KeyOrValue::Key(theme::typography::CAPTION),
            color: KeyOrValue::Key(theme::border::EDGE_HIGHTLIGHT),
            cache: None,
        }
    }

    fn set_text_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        self.color = color.into();
        self.cache = None;
    }

    fn line_count(&self) -> usize {
        self.cache
            .as_ref()
            .map(|cache| cache.layout.line_count())
            .unwrap_or(0)
    }

    fn truncated(&self) -> bool {
        self.cache
            .as_ref()
            .map(|cache| cache.truncated)
            .unwrap_or(false)
    }

    fn full_text(&self) -> String {
        self.text.display_text().to_string()
    }

    fn build_layout(
        &self,
        text_engine: &mut PietText,
        text: &str,
        max_width: f64,
        env: &Env,
    ) -> PietTextLayout {
        let font = self.font.resolve(env);
        let color = self.color.resolve(env);
        text_engine
            .new_text_layout(text.to_owned())
            .font(font.family.clone(), font.size)
            .text_color(color.clone())
            .max_width(max_width)
            .build()
            .unwrap_or_else(|_| {
                text_engine
                    .new_text_layout(String::new())
                    .font(font.family, font.size)
                    .text_color(color)
                    .build()
                    .unwrap()
            })
    }

    /// 生成最多 `MAX_SUB_LABEL_LINES` 行的布局；超出时截断并追加省略号。
    fn build_cache(
        &self,
        text_engine: &mut PietText,
        source: &str,
        max_width: f64,
        env: &Env,
    ) -> WrapCache {
        let layout = self.build_layout(text_engine, source, max_width, env);
        // 没有空格的长版本名可能让某一行超宽（文本框不会在 '-' 处断行），
        // 因此除了行数还要检查实际布局宽度。
        let overflows =
            layout.line_count() > MAX_SUB_LABEL_LINES || layout.size().width > max_width + 0.5;
        if !overflows {
            return WrapCache {
                source: source.to_owned(),
                max_width,
                layout,
                truncated: false,
            };
        }

        let mut lines = String::new();
        for index in 0..MAX_SUB_LABEL_LINES {
            if index > 0 {
                lines.push('\n');
            }
            if let Some(line) = layout.line_text(index) {
                lines.push_str(line.trim_end());
            }
        }
        let mut base = lines;
        loop {
            let shown = format!("{base}…");
            let candidate = self.build_layout(text_engine, &shown, max_width, env);
            if candidate.line_count() <= MAX_SUB_LABEL_LINES
                && candidate.size().width <= max_width + 0.5
            {
                return WrapCache {
                    source: source.to_owned(),
                    max_width,
                    layout: candidate,
                    truncated: true,
                };
            }
            // 按字符回退（String::pop 保证 UTF-8 边界），直到能放进 2 行。
            if base.pop().is_none() {
                return WrapCache {
                    source: source.to_owned(),
                    max_width,
                    layout: candidate,
                    truncated: true,
                };
            }
        }
    }
}

impl<T: Data> Widget<T> for WrapLabel<T> {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if matches!(event, LifeCycle::WidgetAdded) {
            self.text.resolve(data, env);
            self.cache = None;
        }
        if let LifeCycle::DisabledChanged(_) = event {
            ctx.request_paint();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        if self.text.resolve(data, env) {
            self.cache = None;
            ctx.request_layout();
        } else if ctx.env_changed() {
            self.cache = None;
            ctx.request_layout();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, env: &Env) -> Size {
        let max_width = bc.max().width;
        let source = self.text.display_text().to_string();
        let stale = match &self.cache {
            Some(cache) => cache.source != source || cache.max_width != max_width,
            None => true,
        };
        if stale {
            self.cache = Some(self.build_cache(ctx.text(), &source, max_width, env));
        }
        let size = self
            .cache
            .as_ref()
            .map(|cache| cache.layout.size())
            .unwrap_or(Size::ZERO);
        bc.constrain(size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        if let Some(cache) = &self.cache {
            ctx.draw_text(&cache.layout, Point::ZERO);
        } else {
            // 正常情况下 layout 会先执行；兜底避免空白。
            let width = ctx.size().width;
            let source = self.text.display_text();
            let layout = self.build_layout(ctx.text(), &source, width, env);
            ctx.draw_text(&layout, Point::ZERO);
        }
    }
}

#[cfg(test)]
mod tests {
    use druid::debug_state::DebugState;
    use druid::tests::harness::Harness;
    use druid::widget::{EnvScope, Flex, SizedBox};
    use druid::{Data, Env};

    use super::StartGameButton;
    use crate::theme::color::{set_color_to_env, Theme};

    #[derive(Clone, Data)]
    struct TestData {
        version: String,
    }

    fn find_start_button(state: &DebugState) -> Option<&DebugState> {
        if state.display_name == "StartGameButton" {
            return Some(state);
        }
        state.children.iter().find_map(find_start_button)
    }

    fn layout_with_version(version: &str) -> DebugState {
        let button = StartGameButton::dynamic(|data: &TestData, _: &Env| data.version.clone());
        // 与主界面一致：按钮实际可用区域为 220x62，并注入深色主题环境。
        // 外面套 Flex 以模拟 Flex 行给出的宽松约束（Harness 根约束是紧的 400x400）。
        let content = SizedBox::new(button).width(220.).height(62.);
        let content = Flex::column().with_child(content);
        let root = EnvScope::new(
            |env: &mut Env, _data: &TestData| set_color_to_env(env, Theme::Dark),
            content,
        );
        let data = TestData {
            version: version.to_owned(),
        };
        let mut captured = None;
        Harness::create_simple(data, root, |harness| {
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_root_debug_state();
            captured = find_start_button(&state).cloned();
        });
        captured.expect("应能拿到 StartGameButton 的布局状态")
    }

    #[test]
    fn common_long_version_name_is_not_truncated() {
        // 26.1.2-neoforge-26.1.2.126 这类实际版本名必须完整显示，不能左右被裁。
        let state = layout_with_version("26.1.2-neoforge-26.1.2.126");
        assert!(
            state.main_value.contains("truncated=false"),
            "实际长版本名不应被省略: {}",
            state.main_value
        );
    }

    #[test]
    fn oversized_version_name_wraps_to_two_lines_with_ellipsis() {
        let state = layout_with_version(
            "26.1.2-neoforge-26.1.2.126-Fabric-Quilt-Forge-HyperLongVersionName-WithExtraSuffix",
        );
        assert!(
            state.main_value.contains("truncated=true"),
            "超长版本名应使用省略号: {}",
            state.main_value
        );
        assert!(
            state.main_value.contains("lines=2"),
            "超长版本名应限制在 2 行: {}",
            state.main_value
        );
    }
}
