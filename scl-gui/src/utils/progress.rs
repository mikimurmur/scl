//! 将 [`scl_core::progress`] 的进度报告转换为 Druid 命令发送到界面上。

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex, OnceLock,
};

use druid::{ExtEventSink, Target};
use scl_core::progress::{ReportState, Reporter};
use scl_gui_widgets::widgets::{
    ADD_MAX_PROGRESS, ADD_PROGRESS, HIDE_PROGRESS, NEW_PROGRESS, REMOVE_PROGRESS, SET_MAX_PROGRESS,
    SET_MESSAGE, SET_PROGRESS, SET_SUB_MESSAGE,
};

/// 进度编号生成器
static NEXT_PROGRESS_ID: AtomicUsize = AtomicUsize::new(1);

/// GUI 线程维护的单条系统任务栏进度记录
///
/// 用于 Linux（XApps/Unity 启动器协议）与 macOS（Dock 角标）的任务栏进度显示，
/// Windows 的进度由 `scl-gui-widgets` 的进度遮罩直接驱动任务栏。
#[cfg(unix)]
pub struct InnerProgress {
    current: f64,
    max: Option<f64>,
}

#[cfg(unix)]
impl InnerProgress {
    /// 创建一个新的任务栏进度记录
    pub fn new() -> Self {
        Self {
            current: 0.,
            max: None,
        }
    }

    /// 增加当前进度
    pub fn add(&mut self, value: f64) {
        self.current += value;
    }

    /// 增加最大进度
    pub fn add_max(&mut self, value: f64) {
        self.max = Some(self.max.unwrap_or(0.) + value);
    }

    /// 设置最大进度
    pub fn set_max(&mut self, value: f64) {
        self.max = Some(value);
    }

    /// 任务栏进度不展示消息，保留接口以对齐界面命令
    pub fn set_message(&mut self, _message: String) {}

    /// 任务栏进度没有次级标题，保留接口以对齐界面命令
    pub fn set_sub_message(&mut self, _message: String) {}

    /// 计算该记录的进度比例，`None` 表示不确定进度
    #[cfg(target_os = "macos")]
    pub fn ratio(&self) -> Option<f64> {
        self.max.map(|max| {
            if max <= 0. {
                1.
            } else {
                (self.current / max).clamp(0., 1.)
            }
        })
    }
}

/// 当前注册到系统任务栏的进度记录表
#[cfg(unix)]
fn progress_index() -> &'static Mutex<std::collections::HashMap<usize, InnerProgress>> {
    static PROG_INDEX: OnceLock<Mutex<std::collections::HashMap<usize, InnerProgress>>> =
        OnceLock::new();
    PROG_INDEX.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// 依据进度命令更新任务栏进度表，并把聚合结果写入系统任务栏。
///
/// 返回值表示该命令是否属于进度命令（已被消费）。
#[cfg(unix)]
pub fn update_taskbar_progress(cmd: &druid::Command, window: &druid::WindowHandle) -> bool {
    let mut changed = false;
    if let Some(id) = cmd.get(NEW_PROGRESS) {
        if let Ok(mut map) = progress_index().lock() {
            map.entry(*id).or_insert_with(InnerProgress::new);
        }
        changed = true;
    } else if let Some((id, value)) = cmd.get(ADD_PROGRESS) {
        with_progress(id, |p| p.add(*value));
        changed = true;
    } else if let Some((id, value)) = cmd.get(ADD_MAX_PROGRESS) {
        with_progress(id, |p| p.add_max(*value));
        changed = true;
    } else if let Some((id, value)) = cmd.get(SET_MAX_PROGRESS) {
        with_progress(id, |p| p.set_max(*value));
        changed = true;
    } else if let Some((id, value)) = cmd.get(SET_PROGRESS) {
        with_progress(id, |p| p.current = *value);
        changed = true;
    } else if let Some((id, message)) = cmd.get(SET_MESSAGE) {
        with_progress(id, |p| p.set_message(message.clone()));
        changed = true;
    } else if let Some((id, message)) = cmd.get(SET_SUB_MESSAGE) {
        with_progress(id, |p| p.set_sub_message(message.clone()));
        changed = true;
    } else if let Some(id) = cmd.get(REMOVE_PROGRESS).or_else(|| cmd.get(HIDE_PROGRESS)) {
        if let Ok(mut map) = progress_index().lock() {
            map.remove(id);
        }
        changed = true;
    }
    if changed {
        sync_taskbar(window);
    }
    changed
}

#[cfg(unix)]
fn with_progress(id: &usize, f: impl FnOnce(&mut InnerProgress)) {
    if let Ok(mut map) = progress_index().lock() {
        if let Some(progress) = map.get_mut(id) {
            f(progress);
        }
    }
}

/// 把所有进度记录聚合后写入系统任务栏（仅 macOS Dock 角标）
#[cfg(unix)]
fn sync_taskbar(window: &druid::WindowHandle) {
    #[cfg(target_os = "macos")]
    {
        use druid::HasRawWindowHandle;
        use taskbar_interface::{ProgressIndicatorState, TaskbarInterface};

        let Ok(mut map) = progress_index().lock() else {
            return;
        };
        let raw = window.raw_window_handle();
        let display = raw_window_handle_5::RawDisplayHandle::AppKit(
            raw_window_handle_5::AppKitDisplayHandle::empty(),
        );
        let Ok(mut indicator) = TaskbarInterface::new(raw, display) else {
            return;
        };
        if map.is_empty() {
            let _ = indicator.set_progress_state(ProgressIndicatorState::NoProgress);
            return;
        }
        let ratios: Vec<f64> = map.values().filter_map(|x| x.ratio()).collect();
        if ratios.is_empty() {
            let _ = indicator.set_progress_state(ProgressIndicatorState::Indeterminate);
        } else {
            let average = ratios.iter().sum::<f64>() / ratios.len() as f64;
            let _ = indicator.set_progress_state(ProgressIndicatorState::Normal);
            let _ = indicator.set_progress(average.clamp(0., 1.));
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
    }
}

/// 一个将进度上报到 Druid 界面的报告器
#[derive(Clone)]
pub struct Progress {
    id: usize,
    sink: ExtEventSink,
    /// 祖先进度编号链（顶层在前、直接父级在后）
    parents: Vec<usize>,
    /// 是否已经被移除
    removed: bool,
}

impl Progress {
    /// 创建一个新的进度
    pub fn new(sink: ExtEventSink) -> Self {
        let id = NEXT_PROGRESS_ID.fetch_add(1, Ordering::Relaxed);
        let _ = sink.submit_command(NEW_PROGRESS, id, Target::Global);
        Self {
            id,
            sink,
            parents: Vec::new(),
            removed: false,
        }
    }

    /// 向本进度与所有祖先进度发送同一条命令
    ///
    /// 祖先链复刻官方 `sub()`：子任务上报的 Set/Add 进度会同步写入父进度，
    /// 父进度条不再静止不动。
    fn send_to_all<T: Send + Clone + 'static>(
        &self,
        selector: druid::Selector<T>,
        payload: impl Fn(usize) -> T,
    ) {
        if self.removed {
            return;
        }
        for id in &self.parents {
            let _ = self
                .sink
                .submit_command(selector, payload(*id), Target::Global);
        }
        let _ = self
            .sink
            .submit_command(selector, payload(self.id), Target::Global);
    }

    /// 只向本进度发送命令
    fn send_self<T: Send + 'static>(&self, selector: druid::Selector<T>, payload: T) {
        if self.removed {
            return;
        }
        let _ = self.sink.submit_command(selector, payload, Target::Global);
    }

    /// 移除这个进度
    pub fn remove(mut self) {
        self.removed = true;
        // 只移除自身：父进度由创建它的任务负责移除，子进度提前结束不能
        // 连带弹掉仍在运行的父任务进度条。
        let _ = self
            .sink
            .submit_command(REMOVE_PROGRESS, self.id, Target::Global);
    }
}

impl Reporter for Progress {
    fn fork(&self) -> Self {
        // fork 是独立进度，不参与父进度合计（官方默认 fork = to_owned 语义外的
        // 独立新进度；只有 sub 才携带祖先链）。
        Self::new(self.sink.clone())
    }

    fn sub(&self) -> Self {
        let id = NEXT_PROGRESS_ID.fetch_add(1, Ordering::Relaxed);
        let _ = self.sink.submit_command(NEW_PROGRESS, id, Target::Global);
        let mut parents = self.parents.clone();
        parents.push(self.id);
        Self {
            id,
            sink: self.sink.clone(),
            parents,
            removed: false,
        }
    }

    fn send(&self, state: ReportState) {
        // 进度/最大进度/文案同步聚合到祖先；隐藏与移除只作用于自身，
        // 否则一个子任务结束会把仍在运行的父进度条一起弹掉。
        match state {
            ReportState::SetMessage(msg) => {
                self.send_to_all(SET_MESSAGE, |id| (id, msg.clone()));
            }
            ReportState::SetSubMessage(msg) => {
                self.send_to_all(SET_SUB_MESSAGE, |id| (id, msg.clone()));
            }
            ReportState::SetMaxProgress(value) => {
                self.send_to_all(SET_MAX_PROGRESS, |id| (id, value));
            }
            ReportState::AddMaxProgress(value) => {
                self.send_to_all(ADD_MAX_PROGRESS, |id| (id, value));
            }
            ReportState::SetProgress(value) => {
                self.send_to_all(SET_PROGRESS, |id| (id, value));
            }
            ReportState::AddProgress(value) => {
                self.send_to_all(ADD_PROGRESS, |id| (id, value));
            }
            ReportState::SetIndeterminateProgress | ReportState::HideProgress => {
                self.send_self(HIDE_PROGRESS, self.id);
            }
            ReportState::RemoveProgress => {
                self.send_self(REMOVE_PROGRESS, self.id);
            }
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if !self.removed {
            let _ = self
                .sink
                .submit_command(REMOVE_PROGRESS, self.id, Target::Global);
        }
    }
}
