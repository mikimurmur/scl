//! 启动设置页面。

use druid::{
    widget::{Axis, CrossAxisAlignment, Flex, Label, TextBox},
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, Lens, LensExt, LifeCycle, LifeCycleCtx,
    PaintCtx, Size, Target, UpdateCtx, Widget, WidgetExt, WidgetPod,
};
use scl_gui_widgets::widgets::{
    label, LargeListItem, OnChange, Radio, RadioGroup, Slider, Store, ToggleSwitch,
};

use crate::{
    app::SET_MEMORY_STRING,
    app_state::AppState,
    config::{Config, VersionIndependentMethod},
    core::DownloadSource,
    theme::icons::JAVA,
    ui::PAGE_SETTINGS_JAVA,
    widgets::{memory_bar::memory_status, MemoryBar},
};

/// 把自动/手动内存设置映射为 [`MemoryBar`] 需要的 [`Option<usize>`]
struct MemoryLens;

impl Lens<AppState, Option<usize>> for MemoryLens {
    fn with<V, F: FnOnce(&Option<usize>) -> V>(&self, data: &AppState, f: F) -> V {
        f(&if data.config.auto_mem_size {
            None
        } else {
            Some(data.config.mem_size)
        })
    }

    fn with_mut<V, F: FnOnce(&mut Option<usize>) -> V>(&self, data: &mut AppState, f: F) -> V {
        let mut value = if data.config.auto_mem_size {
            None
        } else {
            Some(data.config.mem_size)
        };
        let result = f(&mut value);
        match value {
            None => data.config.auto_mem_size = true,
            Some(value) => {
                data.config.auto_mem_size = false;
                data.config.mem_size = value;
            }
        }
        result
    }
}

/// 官方两个内存单选的变体都是 `bool`，直接绑定 `config.auto_mem_size`
fn auto_mem_lens() -> impl Lens<AppState, bool> {
    AppState::config.then(Config::auto_mem_size)
}

/// 下载并发滑块的数据桥接
fn parallel_pre(data: &AppState, value: &mut f64) {
    *value = data.config.download_parallel_amount as f64;
}

fn parallel_post(value: &f64, data: &mut AppState) {
    data.config.download_parallel_amount = *value as usize;
}

/// 创建一个 11px 的次级说明标签
fn caption(text: &'static str) -> Label<AppState> {
    label::new(text)
        .with_text_size(11.)
        .with_text_color(scl_gui_widgets::theme::color::base::MEDIUM)
}

/// 按官方截图把 MB 数量格式化为带单位文本：0 → `0 B`，<1000 → `x.x MB`，
/// 否则 → `x.x GB`。
fn format_memory_text(mb: f64) -> String {
    if mb <= 0. {
        "0 B".to_owned()
    } else if mb >= 1000. {
        format!("{:.1} GB", mb / 1000.)
    } else {
        format!("{:.1} MB", mb)
    }
}

/// 当前占用/游戏内存的动态标签
fn memory_info_row() -> Flex<AppState> {
    let used = label::dynamic(|_data: &AppState, _env: &druid::Env| {
        let status = memory_status();
        let used = status.max.saturating_sub(status.free);
        format!("当前占用：{}", format_memory_text(used as f64))
    })
    .with_text_size(13.);

    let game = label::dynamic(|data: &AppState, _env: &druid::Env| {
        let status = memory_status();
        let game = if data.config.auto_mem_size {
            (status.free as f64 * 0.75).min(4000.)
        } else {
            data.config.mem_size as f64
        };
        format!("游戏内存：{}", format_memory_text(game))
    })
    .with_text_size(13.);

    Flex::row()
        .with_child(used)
        .with_flex_spacer(1.0)
        .with_child(game)
}

/// 创建一个开关设置项
fn toggle(lens: impl Lens<AppState, bool> + 'static) -> impl Widget<AppState> {
    ToggleSwitch::new().lens(lens)
}

/// 截图辅助：在窗口出现后把滚动容器定位到指定偏移
struct MockScroll {
    inner: WidgetPod<AppState, druid::widget::Scroll<AppState, Flex<AppState>>>,
    offset: f64,
    scrolled: bool,
}

impl MockScroll {
    fn new(inner: druid::widget::Scroll<AppState, Flex<AppState>>, offset: f64) -> Self {
        Self {
            inner: WidgetPod::new(inner),
            offset,
            scrolled: false,
        }
    }
}

impl Widget<AppState> for MockScroll {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        if let Event::AnimFrame(_) = event {
            if !self.scrolled {
                self.scrolled = true;
                self.inner
                    .widget_mut()
                    .scroll_to_on_axis(ctx, Axis::Vertical, self.offset);
                ctx.request_paint();
            }
        }
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &AppState, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            ctx.request_anim_frame();
        }
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old: &AppState, data: &AppState, env: &Env) {
        self.inner.update(ctx, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &AppState,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppState, env: &Env) {
        self.inner.paint(ctx, data, env);
    }

    fn id(&self) -> Option<druid::WidgetId> {
        Some(self.inner.id())
    }
}

/// 创建启动设置页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let java_item = LargeListItem::new(JAVA, "Java 运行时管理", "管理你电脑内的 Java 运行时环境")
        .on_click(|ctx, _data: &mut AppState, _env| {
            ctx.submit_command(
                crate::ui::PUSH_PAGE
                    .with(PAGE_SETTINGS_JAVA)
                    .to(Target::Global),
            );
        });

    let memory_input = TextBox::new()
        .with_placeholder("内存大小")
        .expand_width()
        .lens(AppState::setting_launch_java_memory_size)
        .controller(OnChange::new(
            |ctx: &mut druid::EventCtx, old: &AppState, data: &mut AppState, _env: &druid::Env| {
                if data.setting_launch_java_memory_size != old.setting_launch_java_memory_size {
                    ctx.submit_command(
                        SET_MEMORY_STRING
                            .with(data.setting_launch_java_memory_size.clone())
                            .to(Target::Global),
                    );
                }
            },
        ))
        .disabled_if(|data: &AppState, _env| data.config.auto_mem_size);

    let manual_row = Flex::row()
        .with_child(Radio::new("手动调节内存", false).lens(auto_mem_lens()))
        .with_spacer(12.)
        .with_flex_child(memory_input, 1.0)
        .with_spacer(4.)
        .with_child(label::new("MB"));

    let version_independent = RadioGroup::column(vec![
        ("不使用版本隔离", VersionIndependentMethod::NoIndependent),
        (
            "隔离非正式版",
            VersionIndependentMethod::IndependentForNonRelease,
        ),
        (
            "隔离模组加载器版",
            VersionIndependentMethod::IndependentForModLoader,
        ),
        (
            "隔离非正式版和模组加载器版",
            VersionIndependentMethod::IndependentForModLoaderAndNonRelease,
        ),
        ("隔离全部版本", VersionIndependentMethod::IndependentAll),
    ])
    .lens(AppState::config.then(Config::version_independent_method));

    let download_source = RadioGroup::column(vec![
        ("默认（官方）下载源", DownloadSource::Default),
        ("BMCLAPI 下载源", DownloadSource::BmclApi),
    ])
    .lens(AppState::config.then(Config::download_source));

    let concurrency = label::dynamic(|data: &AppState, _env: &druid::Env| {
        if data.config.download_parallel_amount == 0 {
            "下载并发量：不限（高资源消耗！）".to_owned()
        } else {
            format!("下载并发量：{} 个", data.config.download_parallel_amount)
        }
    })
    .with_text_size(13.);

    let concurrency_slider = Store::new_default(
        Slider::new().with_range(0., 256.),
        parallel_pre,
        parallel_post,
    );

    let advanced_field = |label_text: &'static str, input: Box<dyn Widget<AppState>>| {
        Flex::row()
            .with_child(druid::widget::SizedBox::new(label::new(label_text)).width(110.))
            .with_spacer(8.)
            .with_flex_child(input, 1.0)
    };

    let advanced = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(advanced_field(
            "JVM 参数",
            Box::new(
                TextBox::new()
                    .lens(AppState::config.then(Config::jvm_args))
                    .expand_width(),
            ),
        ))
        .with_spacer(8.)
        .with_child(advanced_field(
            "游戏参数",
            Box::new(
                TextBox::new()
                    .lens(AppState::config.then(Config::game_args))
                    .expand_width(),
            ),
        ))
        .with_spacer(8.)
        .with_child(advanced_field(
            "包装执行程序路径",
            Box::new(
                TextBox::new()
                    .lens(AppState::config.then(Config::wrapper_path))
                    .expand_width(),
            ),
        ))
        .with_spacer(8.)
        .with_child(advanced_field(
            "包装执行程序参数",
            Box::new(
                TextBox::new()
                    .lens(AppState::config.then(Config::wrapper_args))
                    .expand_width(),
            ),
        ));

    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_spacer(8.)
        .with_child(label::new("游戏运行内存").with_text_size(17.))
        .with_child(memory_info_row())
        .with_spacer(8.)
        .with_child(MemoryBar::new().lens(MemoryLens))
        .with_spacer(18.)
        .with_child(
            Flex::row().with_child(Radio::new("自动调节内存", true).lens(auto_mem_lens())),
        )
        .with_child(manual_row)
        .with_spacer(8.)
        .with_child(label::new("启动游戏时等待窗口出现后关闭启动器"))
        .with_child(caption(
            "如果你的游戏启动出现启动器窗口没有关闭的问题可以取消勾选此项，如果关闭将无法使用检查文件完整性的功能",
        ))
        .with_child(toggle(
            AppState::config.then(Config::wait_until_launched),
        ))
        .with_child(label::new("启动后不关闭启动器"))
        .with_child(
            toggle(
                AppState::config
                    .then(Config::dont_close_window_when_launched),
            )
            .disabled_if(|data: &AppState, _env| !data.config.wait_until_launched),
        )
        .with_child(label::new("启动后隐藏启动器"))
        .with_child(
            toggle(
                AppState::config.then(Config::hide_window_when_launched),
            )
            .disabled_if(|data: &AppState, _env| {
                !data.config.dont_close_window_when_launched
            }),
        )
        .with_child(label::new("自动尝试检查文件完整性"))
        .with_child(caption(
            "当启动游戏失败时自动尝试检查文件完整性并尝试重启",
        ))
        // 说明文字写明“如果关闭将无法使用检查文件完整性的功能”，
        // 因此不等待窗口出现时该项禁用。
        .with_child(
            toggle(
                AppState::config
                    .then(Config::auto_check_when_first_start_failed),
            )
            .disabled_if(|data: &AppState, _env| !data.config.wait_until_launched),
        )
        .with_spacer(8.)
        .with_child(label::new("版本隔离方式"))
        .with_child(caption(
            "如需针对指定版本单独设置，请在版本页面里选择指定版本后点击高级设置进行设置",
        ))
        .with_child(version_independent)
        .with_spacer(8.)
        .with_child(label::new("优先下载源"))
        .with_child(caption(
            "如果优先下载源下载失败了，则会依次尝试其他下载源",
        ))
        .with_child(download_source)
        .with_spacer(8.)
        .with_child(concurrency)
        .with_child(concurrency_slider)
        .with_child(caption(
            "虽然 SCL 可以调节并发量甚至不限并发量，但是 64 已经足够绝大部分电脑设备跑满带宽，所以如无必要无需调节并发数。",
        ))
        .with_spacer(8.)
        .with_child(label::new("高级设置"))
        .with_child(caption(
            "以下 JVM 参数和游戏参数均会直接附加在已有参数后面\n而版本独立设置将会覆盖下面的设置",
        ))
        .with_spacer(8.)
        .with_child(advanced);

    let page = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(java_item)
        .with_child(content.padding(12.))
        .scroll()
        .vertical();

    if let Some(offset) = std::env::var("SCL_MOCK_SCROLL")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
    {
        Box::new(MockScroll::new(page, offset))
    } else {
        Box::new(page.boxed())
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use druid::{
        BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point,
        Selector, Size, UpdateCtx, Widget, WidgetId, WidgetPod,
    };
    use scl_gui_widgets::widgets::Store;

    use super::{parallel_post, parallel_pre};
    use crate::app_state::AppState;

    const SET_SLIDER_VALUE: Selector<f64> = Selector::new("test.set-slider-value");
    const SET_CONFIG_VALUE: Selector<usize> = Selector::new("test.set-config-value");

    /// 记录子组件收到的滑块值，并可被命令改成新值以模拟拖动。
    struct Probe {
        seen: Rc<Cell<f64>>,
    }

    impl Widget<f64> for Probe {
        fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64, _env: &Env) {
            if let Event::Command(cmd) = event {
                if let Some(value) = cmd.get(SET_SLIDER_VALUE) {
                    *data = *value;
                    self.seen.set(*data);
                    ctx.set_handled();
                }
            }
        }

        fn lifecycle(
            &mut self,
            _ctx: &mut LifeCycleCtx,
            event: &LifeCycle,
            data: &f64,
            _env: &Env,
        ) {
            if let LifeCycle::WidgetAdded = event {
                self.seen.set(*data);
            }
        }

        fn update(&mut self, _ctx: &mut UpdateCtx, _old: &f64, data: &f64, _env: &Env) {
            self.seen.set(*data);
        }

        fn layout(
            &mut self,
            _ctx: &mut LayoutCtx,
            _bc: &BoxConstraints,
            _data: &f64,
            _env: &Env,
        ) -> Size {
            Size::ZERO
        }

        fn paint(&mut self, _ctx: &mut PaintCtx, _data: &f64, _env: &Env) {}
    }

    /// 在测试中模拟「配置被外部修改」，用于验证 Store 的 update 同步。
    struct ParentSetter {
        inner: WidgetPod<AppState, Store<AppState, f64>>,
    }

    impl Widget<AppState> for ParentSetter {
        fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
            if let Event::Command(cmd) = event {
                if let Some(value) = cmd.get(SET_CONFIG_VALUE) {
                    data.config.download_parallel_amount = *value;
                    ctx.set_handled();
                }
            }
            self.inner.event(ctx, event, data, env);
        }

        fn lifecycle(
            &mut self,
            ctx: &mut LifeCycleCtx,
            event: &LifeCycle,
            data: &AppState,
            env: &Env,
        ) {
            self.inner.lifecycle(ctx, event, data, env);
        }

        fn update(&mut self, ctx: &mut UpdateCtx, _old: &AppState, data: &AppState, env: &Env) {
            self.inner.update(ctx, data, env);
        }

        fn layout(
            &mut self,
            ctx: &mut LayoutCtx,
            bc: &BoxConstraints,
            data: &AppState,
            env: &Env,
        ) -> Size {
            let size = self.inner.layout(ctx, bc, data, env);
            self.inner.set_origin(ctx, Point::ZERO);
            size
        }

        fn paint(&mut self, ctx: &mut PaintCtx, data: &AppState, env: &Env) {
            self.inner.paint(ctx, data, env);
        }

        fn id(&self) -> Option<WidgetId> {
            Some(self.inner.id())
        }
    }

    #[test]
    fn concurrency_slider_round_trips_with_config() {
        let mut state = AppState::default();
        state.config.download_parallel_amount = 63;
        let seen = Rc::new(Cell::new(-1.0));
        let store = Store::new_default(Probe { seen: seen.clone() }, parallel_pre, parallel_post);
        let root = ParentSetter {
            inner: WidgetPod::new(store),
        };

        druid::tests::harness::Harness::create_simple(state, root, |harness| {
            harness.send_initial_events();
            // 配置值 → 滑块值：首次构建就必须同步，不能等用户拖一次。
            assert_eq!(seen.get(), 63.0, "配置里的 63 必须同步到滑块子组件");

            // 配置在页面打开后被外部修改时，update 也要把新值同步给滑块。
            harness.event(Event::Command(SET_CONFIG_VALUE.with(200usize)));
            assert_eq!(seen.get(), 200.0, "配置变化应同步到滑块子组件");

            // 滑块值 → 配置值：模拟拖动到 100 后应写回配置。
            harness.event(Event::Command(SET_SLIDER_VALUE.with(100.0)));
            assert_eq!(
                harness.data().config.download_parallel_amount,
                100,
                "滑块位置变化应写回配置"
            );
            assert_eq!(seen.get(), 100.0);
        });
    }
}
