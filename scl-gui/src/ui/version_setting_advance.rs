//! 版本高级设置页面。

use druid::{
    widget::{Click, CrossAxisAlignment, Flex, Scroll, TextBox},
    Lens, Widget, WidgetExt,
};
use scl_gui_widgets::{
    theme::color::{base, typography},
    widgets::{label, OnChange, Radio},
};

use crate::{
    app_state::AppState,
    core::SCLLaunchConfig,
    widgets::{memory_bar::memory_status, MemoryBar},
};

/// 字段标签固定高度，官方 `SizedBox` 高 32。
const LABEL_HEIGHT: f64 = 32.;
/// 行间距，官方 `with_spacer(5.)`。
const ROW_SPACING: f64 = 5.;
/// 内容外边距，官方 `Padding(10.)`。
const CONTENT_PADDING: f64 = 10.;

/// 把自动/手动内存设置映射为 [`MemoryBar`] 需要的 [`Option<usize>`]
struct MemoryLens;

impl Lens<SCLLaunchConfig, Option<usize>> for MemoryLens {
    fn with<V, F: FnOnce(&Option<usize>) -> V>(&self, data: &SCLLaunchConfig, f: F) -> V {
        let value = if data.override_max_mem {
            data.max_mem_text
                .trim()
                .parse::<usize>()
                .ok()
                .or(data.max_mem)
        } else {
            None
        };
        f(&value)
    }

    fn with_mut<V, F: FnOnce(&mut Option<usize>) -> V>(
        &self,
        data: &mut SCLLaunchConfig,
        f: F,
    ) -> V {
        let mut value = if data.override_max_mem {
            data.max_mem_text
                .trim()
                .parse::<usize>()
                .ok()
                .or(data.max_mem)
        } else {
            None
        };
        let result = f(&mut value);
        // 内存条只读展示，不会写入数据；这里只在确实拿到数值时同步，
        // 避免把「手动调节内存 + 尚未输入数值」的选中态重置回遵循全局。
        if let Some(value) = value {
            data.override_max_mem = true;
            data.max_mem = Some(value);
            data.max_mem_text = value.to_string();
        }
        result
    }
}

/// 按官方截图把 MB 数量格式化为带单位文本：0 → `0 B`，<1024 → `x.x MB`，
/// 否则 → `x.x GB`（官方实测 1012 MB 仍显示 MB）。
fn format_memory_text(mb: f64) -> String {
    if mb <= 0. {
        "0 B".to_owned()
    } else if mb >= 1024. {
        format!("{:.1} GB", mb / 1024.)
    } else {
        format!("{:.1} MB", mb)
    }
}

/// 当前占用/游戏内存的动态标签行
fn memory_info_row() -> Flex<SCLLaunchConfig> {
    let used = label::dynamic(|_data: &SCLLaunchConfig, _env: &druid::Env| {
        let status = memory_status();
        let used = status.max.saturating_sub(status.free);
        format!("当前占用：{}", format_memory_text(used as f64))
    })
    .with_text_size(13.)
    .with_text_color(base::MEDIUM);

    let game = label::dynamic(|data: &SCLLaunchConfig, _env: &druid::Env| {
        let status = memory_status();
        let game = if data.override_max_mem {
            data.max_mem
                .or_else(|| data.max_mem_text.trim().parse::<usize>().ok())
                .unwrap_or(0) as f64
        } else {
            (status.free as f64 * 0.75).min(4000.)
        };
        format!("游戏内存：{}", format_memory_text(game))
    })
    .with_text_size(13.)
    .with_text_color(base::MEDIUM);

    Flex::row()
        .with_child(used)
        .with_flex_spacer(1.0)
        .with_child(game)
}

/// 选择手动内存时把输入框文本同步到 `max_mem`，遵循全局时清空。
fn sync_max_mem(data: &mut SCLLaunchConfig) {
    if data.override_max_mem {
        data.max_mem = data.max_mem_text.trim().parse::<usize>().ok();
    } else {
        data.max_mem = None;
    }
}

/// 创建版本高级设置页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let title = label::new("高级设置")
        .with_font(typography::TITLE)
        .fix_height(40.);

    let desc = label::new("以下 JVM 参数和游戏参数均会直接附加在已有参数后面").fix_height(32.);

    // 字段标签列：官方为 6 个固定 32 高的标签，列内间隔 5，交叉轴 Fill。
    let field_labels = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(
            label::new("JVM 参数")
                .align_vertical(druid::UnitPoint::LEFT)
                .fix_height(LABEL_HEIGHT),
        )
        .with_spacer(ROW_SPACING)
        .with_child(
            label::new("游戏参数")
                .align_vertical(druid::UnitPoint::LEFT)
                .fix_height(LABEL_HEIGHT),
        )
        .with_spacer(ROW_SPACING)
        .with_child(
            label::new("Java 路径")
                .align_vertical(druid::UnitPoint::LEFT)
                .fix_height(LABEL_HEIGHT),
        )
        .with_spacer(ROW_SPACING)
        .with_child(
            label::new("窗口标题")
                .align_vertical(druid::UnitPoint::LEFT)
                .fix_height(LABEL_HEIGHT),
        )
        .with_spacer(ROW_SPACING)
        .with_child(
            label::new("包装执行程序路径")
                .align_vertical(druid::UnitPoint::LEFT)
                .fix_height(LABEL_HEIGHT),
        )
        .with_spacer(ROW_SPACING)
        .with_child(
            label::new("包装执行程序参数")
                .align_vertical(druid::UnitPoint::LEFT)
                .fix_height(LABEL_HEIGHT),
        );

    // 输入框列：官方 6 个 expand_width 的 TextBox，列内间隔 5，作为弹性 1.0 子项。
    let field_inputs = Flex::column()
        .with_child(
            TextBox::new()
                .lens(SCLLaunchConfig::jvm_args)
                .expand_width(),
        )
        .with_spacer(ROW_SPACING)
        .with_child(
            TextBox::new()
                .lens(SCLLaunchConfig::game_args)
                .expand_width(),
        )
        .with_spacer(ROW_SPACING)
        .with_child(
            TextBox::new()
                .lens(SCLLaunchConfig::java_path)
                .expand_width(),
        )
        .with_spacer(ROW_SPACING)
        .with_child(
            TextBox::new()
                .lens(SCLLaunchConfig::window_title)
                .expand_width(),
        )
        .with_spacer(ROW_SPACING)
        .with_child(
            TextBox::new()
                .lens(SCLLaunchConfig::wrapper_path)
                .expand_width(),
        )
        .with_spacer(ROW_SPACING)
        .with_child(
            TextBox::new()
                .lens(SCLLaunchConfig::wrapper_args)
                .expand_width(),
        );

    let fields = Flex::row()
        .with_child(field_labels)
        .with_spacer(ROW_SPACING)
        .with_flex_child(field_inputs, 1.0);

    // 内存数值框：官方包在 SizedBox（expand_width）外再套 OnChange 与 DisabledIf。
    let memory_input = TextBox::new()
        .with_placeholder("内存大小")
        .lens(SCLLaunchConfig::max_mem_text)
        .expand_width()
        .controller(OnChange::new(
            |_ctx: &mut druid::EventCtx,
             _old: &SCLLaunchConfig,
             data: &mut SCLLaunchConfig,
             _env: &druid::Env| {
                sync_max_mem(data);
            },
        ))
        .disabled_if(|data: &SCLLaunchConfig, _env: &druid::Env| !data.override_max_mem);

    // 单选行：官方用 Click<SCLLaunchConfig> 控制器在点击后同步 max_mem。
    let follow_global = Radio::new("遵循全局设置", false)
        .lens(SCLLaunchConfig::override_max_mem)
        .controller(Click::new(
            |ctx: &mut druid::EventCtx, data: &mut SCLLaunchConfig, _env: &druid::Env| {
                data.override_max_mem = false;
                data.max_mem = None;
                ctx.request_layout();
            },
        ));

    let manual_memory_row = Flex::row()
        .with_child(
            Radio::new("手动调节内存", true)
                .lens(SCLLaunchConfig::override_max_mem)
                .controller(Click::new(
                    |ctx: &mut druid::EventCtx, data: &mut SCLLaunchConfig, _env: &druid::Env| {
                        data.override_max_mem = true;
                        if let Ok(value) = data.max_mem_text.trim().parse::<usize>() {
                            data.max_mem = Some(value);
                        }
                        ctx.request_layout();
                    },
                )),
        )
        .with_spacer(12.)
        .with_flex_child(memory_input, 1.0)
        .with_spacer(4.)
        .with_child(label::new("MB"));

    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(title)
        .with_child(desc)
        .with_spacer(8.)
        .with_child(fields)
        .with_spacer(8.)
        .with_child(label::new("游戏运行内存").with_text_size(14.))
        .with_spacer(8.)
        .with_child(memory_info_row())
        .with_spacer(8.)
        .with_child(MemoryBar::new().lens(MemoryLens))
        .with_spacer(18.)
        .with_child(Flex::row().with_child(follow_global))
        .with_child(manual_memory_row)
        .with_spacer(8.)
        .padding(CONTENT_PADDING);

    Box::new(
        Scroll::new(content)
            .vertical()
            .expand()
            .lens(AppState::version_setting_config),
    )
}
