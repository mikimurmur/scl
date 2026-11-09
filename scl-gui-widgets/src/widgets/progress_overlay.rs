use std::collections::BTreeMap;

use druid::{
    piet::{TextLayout, *},
    *,
};
use scl_gui_animation::Spring;

/// 用于增加新进度的指令，需要传递一个独一无二的编号作为此进度的唯一编号
pub const NEW_PROGRESS: Selector<usize> = Selector::new("net.stevexmh.scl.progress.new");
/// 用于删除进度的指令，需要传递此进度的唯一编号
pub const REMOVE_PROGRESS: Selector<usize> = Selector::new("net.stevexmh.scl.progress.rm");
/// 用于给进度设置主要信息，需要传递此进度的唯一编号
pub const SET_MESSAGE: Selector<(usize, String)> =
    Selector::new("net.stevexmh.scl.progress.set-msg");
/// 用于给进度设置次要信息，需要传递此进度的唯一编号和其需要展示的信息
pub const SET_SUB_MESSAGE: Selector<(usize, String)> =
    Selector::new("net.stevexmh.scl.progress.set-sub-msg");
/// 用于给进度设置最大进度值，需要传递此进度的唯一编号和其进度最大值
pub const SET_MAX_PROGRESS: Selector<(usize, f64)> =
    Selector::new("net.stevexmh.scl.progress.set-max");
/// 用于给进度增加/减少最大进度值，需要传递此进度的唯一编号和其进度最大值变化值
pub const ADD_MAX_PROGRESS: Selector<(usize, f64)> =
    Selector::new("net.stevexmh.scl.progress.add-max");
/// 用于给进度设置进度值，需要传递此进度的唯一编号和其进度值
pub const SET_PROGRESS: Selector<(usize, f64)> = Selector::new("net.stevexmh.scl.progress.set");
/// 用于给进度增加/减少进度值，需要传递此进度的唯一编号和其进度变化值
pub const ADD_PROGRESS: Selector<(usize, f64)> = Selector::new("net.stevexmh.scl.progress.add");
/// 用于给进度隐藏进度值，需要传递此进度的唯一编号
pub const HIDE_PROGRESS: Selector<usize> = Selector::new("net.stevexmh.scl.progress.hide");

struct ProgressState {
    max_progress: f64,
    progress: f64,
    current_progress: f64,
    indeterminate: bool,
    msg: String,
    msg_layout: Option<PietTextLayout>,
    sub_msg: String,
    sub_msg_layout: Option<PietTextLayout>,
}

/// 当前完成比例；分母为 0 或不定进度时视为 0
fn progress_ratio(item: &ProgressState) -> f64 {
    if item.indeterminate || item.max_progress <= 0. {
        0.
    } else {
        (item.progress / item.max_progress).clamp(0., 1.)
    }
}

/// 进度文本：字节级进度显示 MB，小粒度任务仍显示文件/步骤计数
fn progress_amount_text(current: f64, max: f64) -> String {
    if max >= 1_048_576. {
        format!("{:.1}/{:.1} MB", current / 1_048_576., max / 1_048_576.)
    } else {
        format!("{}/{}", current.round(), max.round())
    }
}

/// 一个进度显示组件，用于搭配 SCL Core 的进度报告功能显示正在处理的项目进度
///
/// 在没有进度时会隐藏，当存在进度时将会占用一点位置来显示进度。
/// 此时如果鼠标悬浮在这个区域则会放大进度组件以查看正在处理的所有进度。
pub struct ProgressOverlay<T> {
    show_progress_on_taskbar: bool,

    progress_expanding: bool,
    progress_height_spring: Spring,
    progress_map: BTreeMap<usize, ProgressState>,

    inner: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T> ProgressOverlay<T> {
    /// 以一个将会显示进度的组件为参数创建此组件
    pub fn new(inner: impl Widget<T> + 'static) -> Self {
        Self {
            show_progress_on_taskbar: false,
            progress_expanding: false,
            progress_height_spring: Spring::new(0.),
            progress_map: BTreeMap::new(),
            inner: WidgetPod::new(Box::new(inner)),
        }
    }

    /// 是否将接收的进度通过系统接口显示在任务栏的应用程序图标上
    ///
    /// 目前支持 Linux 和 Windows 环境，其它平台下调用该函数将没有任何作用。
    pub fn show_progress_on_taskbar(mut self) -> Self {
        self.show_progress_on_taskbar = true;
        self
    }

    fn should_display(&self) -> bool {
        !self.progress_map.is_empty()
    }

    fn should_update_progress(&self) -> bool {
        if self.should_display() {
            for p in self.progress_map.values() {
                if !p.indeterminate && progress_ratio(p) > p.current_progress + 0.0001 {
                    return true;
                }
            }
        }
        false
    }

    /// Windows/macOS 任务栏进度；调用点位于同名 cfg 块内，
    /// Linux 构建不编译该方法（官方 Linux 版也不接系统任务栏）。
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn update_taskbar_progress(&self, _handle: RawWindowHandle) {
        #[cfg(target_os = "windows")]
        {
            use taskbar_interface::*;
            if let Ok(mut indicator) = TaskbarInterface::new(
                _handle,
                raw_window_handle_5::RawDisplayHandle::Windows(
                    raw_window_handle_5::WindowsDisplayHandle::empty(),
                ),
            ) {
                if self.progress_map.is_empty() {
                    let _ = indicator.set_progress_state(ProgressIndicatorState::NoProgress);
                } else if self.progress_map.iter().all(|x| x.1.indeterminate) {
                    let _ = indicator.set_progress_state(ProgressIndicatorState::Indeterminate);
                } else {
                    // let _ = indicator.set_progress_state(ProgressIndicatorState::Normal);
                    let max_progress: f64 = self
                        .progress_map
                        .iter()
                        .filter(|x| !x.1.indeterminate)
                        .map(|x| x.1.max_progress)
                        .sum();
                    let cur_progress: f64 = self
                        .progress_map
                        .iter()
                        .filter(|x| !x.1.indeterminate)
                        .map(|x| x.1.progress.max(0.0))
                        .sum();
                    let _ = indicator.set_progress((cur_progress / max_progress).clamp(0.0, 1.0));
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            use taskbar_interface::*;
            if let Ok(mut indicator) = TaskbarInterface::new(
                _handle,
                raw_window_handle_5::RawDisplayHandle::AppKit(
                    raw_window_handle_5::AppKitDisplayHandle::empty(),
                ),
            ) {
                if self.progress_map.is_empty() {
                    let _ = indicator.set_progress_state(ProgressIndicatorState::NoProgress);
                } else if self.progress_map.iter().all(|x| x.1.indeterminate) {
                    let _ = indicator.set_progress_state(ProgressIndicatorState::Indeterminate);
                } else {
                    // let _ = indicator.set_progress_state(ProgressIndicatorState::Normal);
                    let max_progress: f64 = self
                        .progress_map
                        .iter()
                        .filter(|x| !x.1.indeterminate)
                        .map(|x| x.1.max_progress)
                        .sum();
                    let cur_progress: f64 = self
                        .progress_map
                        .iter()
                        .filter(|x| !x.1.indeterminate)
                        .map(|x| x.1.progress.max(0.0))
                        .sum();
                    let _ = indicator.set_progress((cur_progress / max_progress).clamp(0.0, 1.0));
                }
            }
        }
    }

    fn paint_progress(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        ctx.with_save(|ctx| {
            // Envs
            let bg = env
                .get(druid::theme::WINDOW_BACKGROUND_COLOR)
                .with_alpha(1.);
            let base_low = env.get(crate::theme::color::base::LOW);
            let base_high = env.get(crate::theme::color::base::HIGH);
            let accent_sec = env.get(crate::theme::color::main::SECONDARY);
            let base_font = env.get(crate::theme::color::typography::BASE).family;

            let linear_brush = PaintBrush::Linear(LinearGradient::new(
                UnitPoint::LEFT,
                UnitPoint::RIGHT,
                (bg.to_owned().with_alpha(0.), bg.to_owned()),
            ));

            let width = ctx.size().width;
            let height = ctx.size().height;
            let mut current_height = height - self.progress_height_spring.position();
            let rect = Rect::new(0., current_height, width, height);
            ctx.fill(rect, &bg);
            let rect = Rect::new(0., current_height, width, current_height + 1.);
            ctx.fill(rect, &base_low);

            for (i, (_, item)) in self.progress_map.iter_mut().enumerate() {
                if item.progress.round() == item.max_progress.round()
                    && item.msg.is_empty()
                    && item.sub_msg.is_empty()
                {
                    continue;
                }
                // 只允许向上追赶；分母或阶段变化不得让已显示的进度回退。
                if !item.indeterminate {
                    let target = progress_ratio(item);
                    if target > item.current_progress {
                        item.current_progress += (target - item.current_progress) / 7.;
                        if target - item.current_progress < 0.0005 {
                            item.current_progress = target;
                        }
                    }
                }
                let text = ctx.text();
                if item.msg_layout.is_none() {
                    item.msg_layout = text
                        .new_text_layout(item.msg.to_owned())
                        .alignment(TextAlignment::Start)
                        .text_color(base_high.to_owned())
                        .font(base_font.to_owned(), 12.)
                        .build()
                        .ok();
                }
                let layout_height = item
                    .msg_layout
                    .as_ref()
                    .map(|x| x.size().height)
                    .unwrap_or(0.);
                if item.sub_msg_layout.is_none() {
                    item.sub_msg_layout = text
                        .new_text_layout(if item.sub_msg.is_empty() {
                            if progress_ratio(item) < 0.9995 {
                                progress_amount_text(item.progress, item.max_progress)
                            } else {
                                "".into()
                            }
                        } else {
                            item.sub_msg.to_owned()
                        })
                        .alignment(TextAlignment::Start)
                        .text_color(base_high.to_owned())
                        .font(base_font.to_owned(), 12.)
                        .build()
                        .ok();
                }
                let right_layout_width = item
                    .sub_msg_layout
                    .as_ref()
                    .map(|x| x.size().width)
                    .unwrap_or(0.);
                ctx.with_save(|ctx| {
                    ctx.clip(Rect::new(
                        12.,
                        current_height + 12.,
                        width - right_layout_width - 20.,
                        current_height + 12. + layout_height,
                    ));
                    if let Some(layout) = &item.msg_layout {
                        ctx.draw_text(layout, (12., current_height + 12.));
                    }
                });
                ctx.fill(
                    Rect::new(
                        width - right_layout_width - 20.,
                        current_height + 12.,
                        width - right_layout_width - 30.,
                        current_height + 12. + layout_height,
                    ),
                    &linear_brush,
                );
                if let Some(layout) = &item.sub_msg_layout {
                    ctx.draw_text(
                        layout,
                        (width - 20. - right_layout_width, current_height + 12.),
                    );
                }
                if !item.indeterminate {
                    let progress_bar_rect =
                        Rect::from_origin_size((10., current_height + 35.), (width - 20., 4.));
                    ctx.fill(
                        progress_bar_rect.to_rounded_rect(progress_bar_rect.height() / 2.),
                        &base_low,
                    );
                    let progress_bar_current_rect = Rect::from_origin_size(
                        progress_bar_rect.origin(),
                        (
                            progress_bar_rect.width() * item.current_progress.clamp(0., 1.),
                            progress_bar_rect.height(),
                        ),
                    );
                    ctx.fill(
                        progress_bar_current_rect.to_rounded_rect(progress_bar_rect.height() / 2.),
                        &accent_sec,
                    );
                }
                if i > 0 && current_height > height - 100. {
                    ctx.fill(
                        Rect::from_origin_size((0., current_height), (width, 50.)),
                        &bg.with_alpha((current_height - (height - 100.)) * 0.4 / 20.),
                    );
                    break;
                }
                current_height += 30.;
                if item.progress > 0. && item.progress <= 1. {
                    current_height += 20.;
                }
            }
        })
    }
}

impl<T: Data> Widget<T> for ProgressOverlay<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseMove(m) => {
                let size = ctx.size();
                let width = size.width;
                let height = size.height;
                let m_pos_x = m.pos.x;
                let m_pos_y = m.pos.y;
                let old_progress_expanding = self.progress_expanding;
                self.progress_expanding = self.should_display()
                    && m_pos_y > height - 50.
                    && m_pos_y < height
                    && m_pos_x > 0.
                    && m_pos_x < width;
                if old_progress_expanding != self.progress_expanding {
                    self.progress_height_spring
                        .set_target(if self.progress_expanding {
                            height
                        } else if self.should_display() {
                            50.
                        } else {
                            0.
                        });
                    ctx.request_anim_frame();
                }
            }
            Event::AnimFrame(_) => {
                let spring_moving = !self.progress_height_spring.arrived();
                if spring_moving {
                    // 只有遮罩展开/收起的高度变化才需要重新布局；
                    // 进度数值变化只需重绘，不能每帧 request_layout。
                    ctx.request_layout();
                }
                if spring_moving || self.should_update_progress() {
                    // Linux 不接入系统任务栏进度；
                    // GTK 后端没有 RawWindowHandle，取句柄会在每一帧打印
                    // “HasRawWindowHandle trait not implemented for gtk.”。
                    #[cfg(any(target_os = "windows", target_os = "macos"))]
                    if self.show_progress_on_taskbar {
                        self.update_taskbar_progress(ctx.window().raw_window_handle());
                    }
                    ctx.request_paint();
                    ctx.request_anim_frame();
                }
            }
            Event::Command(cmd) => {
                if let Some(&p) = cmd.get(NEW_PROGRESS) {
                    self.progress_height_spring
                        .set_target(if self.progress_map.len() > 1 {
                            if self.progress_expanding {
                                ctx.size().height
                            } else {
                                50.
                            }
                        } else {
                            50.
                        });
                    self.progress_map.insert(
                        p,
                        ProgressState {
                            max_progress: 0.,
                            progress: 0.,
                            current_progress: 0.,
                            indeterminate: true,
                            msg: String::new(),
                            msg_layout: None,
                            sub_msg: String::new(),
                            sub_msg_layout: None,
                        },
                    );
                    ctx.request_anim_frame();
                } else if let Some(&p) = cmd.get(REMOVE_PROGRESS) {
                    self.progress_map.remove(&p);
                    if self.progress_map.is_empty() {
                        self.progress_height_spring.set_target(0.);
                    }
                    #[cfg(target_os = "windows")]
                    if self.progress_map.is_empty() && self.show_progress_on_taskbar {
                        use taskbar_interface::*;
                        if let Ok(mut indicator) = TaskbarInterface::new(
                            ctx.window().raw_window_handle(),
                            raw_window_handle_5::RawDisplayHandle::Windows(
                                raw_window_handle_5::WindowsDisplayHandle::empty(),
                            ),
                        ) {
                            let _ = indicator.needs_attention(true);
                        }
                    }
                    #[cfg(target_os = "macos")]
                    if self.progress_map.is_empty() && self.show_progress_on_taskbar {
                        use taskbar_interface::*;
                        if let Ok(mut indicator) = TaskbarInterface::new(
                            ctx.window().raw_window_handle(),
                            raw_window_handle_5::RawDisplayHandle::AppKit(
                                raw_window_handle_5::AppKitDisplayHandle::empty(),
                            ),
                        ) {
                            let _ = indicator.needs_attention(true);
                        }
                    }
                    ctx.request_anim_frame();
                } else if let Some(p) = cmd.get(SET_PROGRESS) {
                    if let Some(v) = self.progress_map.get_mut(&p.0) {
                        v.progress = p.1;
                        if v.indeterminate {
                            v.indeterminate = false;
                            // 从不定进度切换到确定进度时从当前比例起步，
                            // 不能把绝对进度值当作已完成比例（否则会瞬间满格）。
                            v.current_progress = progress_ratio(v);
                        }
                        if v.sub_msg.is_empty() {
                            v.sub_msg_layout = None;
                        }
                        ctx.request_anim_frame();
                    }
                } else if let Some(p) = cmd.get(ADD_PROGRESS) {
                    if let Some(v) = self.progress_map.get_mut(&p.0) {
                        v.progress += p.1;
                        v.indeterminate = false;
                        if v.sub_msg.is_empty() {
                            v.sub_msg_layout = None;
                        }
                        ctx.request_anim_frame();
                    }
                } else if let Some(p) = cmd.get(SET_MAX_PROGRESS) {
                    if let Some(v) = self.progress_map.get_mut(&p.0) {
                        v.max_progress = p.1;
                        v.indeterminate = false;
                        if v.sub_msg.is_empty() {
                            v.sub_msg_layout = None;
                        }
                        ctx.request_anim_frame();
                    }
                } else if let Some(p) = cmd.get(ADD_MAX_PROGRESS) {
                    if let Some(v) = self.progress_map.get_mut(&p.0) {
                        v.max_progress += p.1;
                        v.indeterminate = false;
                        if v.sub_msg.is_empty() {
                            v.sub_msg_layout = None;
                        }
                        ctx.request_anim_frame();
                    }
                } else if let Some(p) = cmd.get(HIDE_PROGRESS) {
                    if let Some(v) = self.progress_map.get_mut(p) {
                        v.indeterminate = true;
                        ctx.request_anim_frame();
                    }
                } else if let Some(p) = cmd.get(SET_MESSAGE) {
                    if let Some(v) = self.progress_map.get_mut(&p.0) {
                        v.msg = p.1.to_owned();
                        v.msg_layout = None;
                        ctx.request_anim_frame();
                    }
                } else if let Some(p) = cmd.get(SET_SUB_MESSAGE) {
                    if let Some(v) = self.progress_map.get_mut(&p.0) {
                        v.sub_msg = p.1.to_owned();
                        v.sub_msg_layout = None;
                        ctx.request_anim_frame();
                    }
                }
            }
            _ => {}
        }
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let progress_area_height = self
            .progress_height_spring
            .position_rounded()
            .clamp(0., 50.);
        let mut size = if progress_area_height == 0. {
            self.inner.layout(ctx, bc, data, env)
        } else {
            self.inner
                .layout(ctx, &bc.shrink((0., progress_area_height)), data, env)
        };
        self.inner.set_origin(ctx, druid::Point::ZERO);
        size.height += progress_area_height;
        bc.constrain(size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        ctx.with_save(|ctx| self.inner.paint(ctx, data, env));
        self.paint_progress(ctx, data, env);
    }
}

#[cfg(test)]
mod tests {
    use super::{progress_amount_text, progress_ratio, ProgressState};

    fn state(progress: f64, max: f64, indeterminate: bool) -> ProgressState {
        ProgressState {
            max_progress: max,
            progress,
            current_progress: 0.,
            indeterminate,
            msg: String::new(),
            msg_layout: None,
            sub_msg: String::new(),
            sub_msg_layout: None,
        }
    }

    /// 分母为 0、不定进度、超出范围时都必须返回可绘制的合法比例。
    #[test]
    fn ratio_is_always_within_unit_range() {
        assert_eq!(progress_ratio(&state(0., 0., false)), 0.);
        assert_eq!(progress_ratio(&state(50., 100., false)), 0.5);
        assert_eq!(progress_ratio(&state(50., 100., true)), 0.);
        assert_eq!(progress_ratio(&state(150., 100., false)), 1.);
        assert_eq!(progress_ratio(&state(-1., 100., false)), 0.);
    }

    /// 字节级进度以 MB 显示，小粒度任务仍显示计数，避免出现一百万分之一的文本。
    #[test]
    fn amount_text_uses_bytes_only_for_byte_scale_tasks() {
        assert_eq!(progress_amount_text(1_048_576., 2_097_152.), "1.0/2.0 MB");
        assert_eq!(progress_amount_text(3., 5.), "3/5");
    }
}
