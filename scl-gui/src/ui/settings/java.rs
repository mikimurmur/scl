//! Java 运行时管理页面。

use druid::{
    commands,
    kurbo::BezPath,
    piet::{PaintBrush, Text, TextLayout, TextLayoutBuilder},
    widget::{
        Click, ControllerHost, CrossAxisAlignment, Flex, List, Scroll, SizedBox, ViewSwitcher,
    },
    BoxConstraints, Env, Event, FileDialogOptions, Insets, LayoutCtx, LensExt, LifeCycle,
    LifeCycleCtx, PaintCtx, RenderContext, Size, Widget, WidgetExt,
};
use scl_gui_widgets::{
    theme::{
        color::{base, list, main, typography},
        icons::IconKeyPair,
    },
    widgets::label,
};

use crate::{
    app::{java, MANUALLY_ADD_JAVA},
    app_state::AppState,
    config::{Config, JavaListData},
    theme::icons::ADD,
    widgets::java_item::JavaItem,
};

/// 官方的「添加 / 搜索」行：+ 图标 + 左对齐文字，行高 50
struct ActionRow {
    icon_key: IconKeyPair,
    icon_path: BezPath,
    text: &'static str,
}

impl ActionRow {
    fn new(icon_key: IconKeyPair, text: &'static str) -> Self {
        Self {
            icon_key,
            icon_path: BezPath::new(),
            text,
        }
    }

    fn reload_icon(&mut self, env: &Env) {
        self.icon_path = BezPath::from_svg(&env.get(&self.icon_key.0)).unwrap_or_default();
    }
}

impl Widget<AppState> for ActionRow {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &Event,
        _data: &mut AppState,
        _env: &Env,
    ) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) if ctx.is_active() => {
                ctx.set_active(false);
                ctx.request_paint();
            }
            Event::MouseMove(_) => {
                ctx.set_cursor(&druid::Cursor::Pointer);
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &AppState,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            self.reload_icon(env);
        } else if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
    }

    fn update(&mut self, ctx: &mut druid::UpdateCtx, _old: &AppState, _data: &AppState, env: &Env) {
        if ctx.env_key_changed(&self.icon_key.0) {
            self.reload_icon(env);
        }
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &AppState,
        _env: &Env,
    ) -> Size {
        // 原版截图：默认 Java 行与按钮行均为 50px 高。
        bc.constrain(Size::new(bc.max().width, 50.))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &AppState, env: &Env) {
        let size = ctx.size();

        // 与运行时列表项一致：hover 显示列表底色，按下加深。
        if ctx.is_hot() {
            let is_active = ctx.is_active();
            ctx.fill(
                size.to_rect(),
                &PaintBrush::Color(if is_active {
                    env.get(base::LOW)
                } else {
                    env.get(list::LIST_LOW)
                }),
            );
        }

        // 原版：+ 图标约 24x24（16x16 视图盒放大 1.5），左内边距 14，文字从 50 开始。
        ctx.with_save(|ctx| {
            ctx.transform(druid::kurbo::Affine::translate((14., 13.)));
            ctx.transform(druid::kurbo::Affine::scale(24. / 16.));
            ctx.fill(&self.icon_path, &env.get(base::HIGH));
        });

        let font = env.get(typography::BODY);
        if let Ok(layout) = ctx
            .text()
            .new_text_layout(self.text)
            .font(font.family, font.size)
            .text_color(env.get(base::HIGH))
            .build()
        {
            ctx.draw_text(&layout, (50., (size.height - layout.size().height) / 2.));
        }
    }
}

fn action_row(
    icon_key: IconKeyPair,
    text: &'static str,
    on_click: impl Fn(&mut druid::EventCtx, &mut AppState, &Env) + 'static,
) -> ControllerHost<ActionRow, Click<AppState>> {
    ControllerHost::new(ActionRow::new(icon_key, text), Click::new(on_click))
}

/// 创建 Java 运行时管理页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let list = List::new(JavaItem::new)
        .lens(
            AppState::config
                .then(Config::javas)
                .then(JavaListData::javapaths),
        )
        .expand_width();

    // 官方把 open panel 的 accept command 设为
    let add_row = action_row(
        ADD,
        "添加新的 Java 运行时",
        |ctx: &mut druid::EventCtx, _data: &mut AppState, _env| {
            ctx.submit_command(
                commands::SHOW_OPEN_PANEL.with(
                    FileDialogOptions::new()
                        .title("请选择名称为 java 的运行时")
                        .accept_command(MANUALLY_ADD_JAVA),
                ),
            );
        },
    );

    let search_row = action_row(
        ADD,
        "尝试搜索可用 Java 运行时",
        |ctx: &mut druid::EventCtx, data: &mut AppState, _env| {
            java::search_java(ctx, data);
        },
    );

    // 问题 ⑤：扫描候选解析失败时给出可见反馈，而不是只写日志。
    let scan_message = ViewSwitcher::new(
        |data: &AppState, _env: &Env| data.java_scan_message.clone(),
        |message: &Option<String>, _data: &AppState, _env: &Env| -> Box<dyn Widget<AppState>> {
            match message {
                Some(text) => Box::new(
                    label::new(text.clone())
                        .with_text_color(main::PRIMARY)
                        .padding(Insets::new(14., 8., 14., 8.)),
                ),
                None => Box::new(SizedBox::empty()),
            }
        },
    );

    // 注意：外层会再套一次 Scroll，因此这里不能再使用 flex child，
    // 否则 Flex 在无限高度约束下会告警并把 List 撑到无限高。
    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(list)
        .with_child(add_row)
        .with_child(search_row)
        .with_child(scan_message);

    // 只在外层 column 上滚动一次；不要再嵌套 Scroll::new(list)。
    Box::new(Scroll::new(content).vertical().expand_width().boxed())
}
