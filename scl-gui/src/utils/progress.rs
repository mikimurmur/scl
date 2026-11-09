//! 将 [`scl_core::progress`] 的进度报告转换为 Druid 命令发送到界面上。

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
    time::Duration,
};

use druid::{ExtEventSink, Target};
use scl_core::progress::{ReportState, Reporter};
use scl_gui_widgets::widgets::{
    ADD_MAX_PROGRESS, ADD_PROGRESS, HIDE_PROGRESS, NEW_PROGRESS, REMOVE_PROGRESS, SET_MAX_PROGRESS,
    SET_MESSAGE, SET_PROGRESS, SET_SUB_MESSAGE,
};

/// 进度事件合并窗口
///
/// 下载器按文件完成上报，64 并发时会在极短时间内产生成批事件；直接逐条
/// 提交到 Druid 会让界面在事件风暴里反复重排。这里把同一进度 id 的状态
/// 合并到每 60ms 最多刷新一次，既保持视觉连续，也避免无意义重绘。
const PROGRESS_FLUSH_INTERVAL: Duration = Duration::from_millis(60);

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

/// 合并中的单条进度状态
struct PendingProgress {
    sink: ExtEventSink,
    current: f64,
    max: f64,
    message: Option<String>,
    sub_message: Option<String>,
    current_dirty: bool,
    max_dirty: bool,
    message_dirty: bool,
    sub_message_dirty: bool,
    indeterminate: bool,
    indeterminate_dirty: bool,
}

impl PendingProgress {
    fn new(sink: ExtEventSink) -> Self {
        Self {
            sink,
            current: 0.,
            max: 0.,
            message: None,
            sub_message: None,
            current_dirty: false,
            max_dirty: false,
            message_dirty: false,
            sub_message_dirty: false,
            indeterminate: false,
            indeterminate_dirty: false,
        }
    }
}

fn pending_map() -> &'static Mutex<HashMap<usize, PendingProgress>> {
    static PENDING: OnceLock<Mutex<HashMap<usize, PendingProgress>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 启动后台合并线程（全进程一个即可）
fn ensure_flusher() {
    static FLUSHER: OnceLock<()> = OnceLock::new();
    FLUSHER.get_or_init(|| {
        let _ = std::thread::Builder::new()
            .name("scl-progress-flush".into())
            .spawn(|| loop {
                std::thread::sleep(PROGRESS_FLUSH_INTERVAL);
                flush_pending();
            });
    });
}

/// 把当前合并窗口内的状态一次性提交给界面
fn flush_pending() {
    let batch: Vec<_> = {
        let Ok(mut map) = pending_map().lock() else {
            return;
        };
        map.iter_mut()
            .filter_map(|(id, progress)| {
                let dirty = progress.current_dirty
                    || progress.max_dirty
                    || progress.message_dirty
                    || progress.sub_message_dirty
                    || progress.indeterminate_dirty;
                if !dirty {
                    return None;
                }
                let hide = progress.indeterminate && progress.indeterminate_dirty;
                let max = progress.max_dirty.then_some(progress.max);
                let current = progress.current_dirty.then_some(progress.current);
                let message = if progress.message_dirty {
                    progress.message.take()
                } else {
                    None
                };
                let sub_message = if progress.sub_message_dirty {
                    progress.sub_message.take()
                } else {
                    None
                };
                progress.current_dirty = false;
                progress.max_dirty = false;
                progress.message_dirty = false;
                progress.sub_message_dirty = false;
                progress.indeterminate_dirty = false;
                Some((
                    *id,
                    progress.sink.clone(),
                    hide,
                    max,
                    current,
                    message,
                    sub_message,
                ))
            })
            .collect()
    };

    for (id, sink, hide, max, current, message, sub_message) in batch {
        if hide {
            let _ = sink.submit_command(HIDE_PROGRESS, id, Target::Global);
            continue;
        }
        // 先更新分母再更新完成量，避免界面短暂拿到旧分母。
        if let Some(max) = max {
            let _ = sink.submit_command(SET_MAX_PROGRESS, (id, max), Target::Global);
        }
        if let Some(current) = current {
            let _ = sink.submit_command(SET_PROGRESS, (id, current), Target::Global);
        }
        if let Some(message) = message {
            let _ = sink.submit_command(SET_MESSAGE, (id, message), Target::Global);
        }
        if let Some(sub_message) = sub_message {
            let _ = sink.submit_command(SET_SUB_MESSAGE, (id, sub_message), Target::Global);
        }
    }
}

fn with_pending(sink: &ExtEventSink, id: usize, f: impl FnOnce(&mut PendingProgress)) {
    ensure_flusher();
    let Ok(mut map) = pending_map().lock() else {
        return;
    };
    let progress = map
        .entry(id)
        .or_insert_with(|| PendingProgress::new(sink.clone()));
    f(progress);
}

fn remove_pending(id: usize) {
    if let Ok(mut map) = pending_map().lock() {
        map.remove(&id);
    }
}

/// 一个将进度上报到 Druid 界面的报告器
///
/// `clone` 共享同一个进度编号，只用于把同一个任务内的多个线程/闭包写入
/// 同一条进度；只有创建者（owner）在 drop 时才负责移除进度条。
pub struct Progress {
    id: usize,
    sink: ExtEventSink,
    /// 祖先进度编号链（顶层在前、直接父级在后）
    parents: Vec<usize>,
    /// 是否已经被移除
    removed: bool,
    /// 是否为这条进度的创建者（仅创建者 drop 时移除）
    owner: bool,
}

impl Clone for Progress {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            sink: self.sink.clone(),
            parents: self.parents.clone(),
            removed: self.removed,
            owner: false,
        }
    }
}

impl Progress {
    /// 创建一个新的进度
    pub fn new(sink: ExtEventSink) -> Self {
        let id = NEXT_PROGRESS_ID.fetch_add(1, Ordering::Relaxed);
        Self::register(sink, id, Vec::new())
    }

    fn register(sink: ExtEventSink, id: usize, parents: Vec<usize>) -> Self {
        let _ = sink.submit_command(NEW_PROGRESS, id, Target::Global);
        with_pending(&sink, id, |_| {});
        Self {
            id,
            sink,
            parents,
            removed: false,
            owner: true,
        }
    }

    /// 更新本进度与所有祖先的合并状态
    fn update_all(&self, f: impl Fn(&mut PendingProgress)) {
        if self.removed {
            return;
        }
        for id in &self.parents {
            with_pending(&self.sink, *id, &f);
        }
        with_pending(&self.sink, self.id, &f);
    }

    /// 只更新本进度的合并状态
    fn update_self(&self, f: impl FnOnce(&mut PendingProgress)) {
        if self.removed {
            return;
        }
        with_pending(&self.sink, self.id, f);
    }

    /// 只移除自身，父进度由创建它的任务负责移除
    fn remove_self(&self) {
        remove_pending(self.id);
        let _ = self
            .sink
            .submit_command(REMOVE_PROGRESS, self.id, Target::Global);
    }

    /// 移除这个进度
    pub fn remove(mut self) {
        self.removed = true;
        self.remove_self();
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
        let mut parents = self.parents.clone();
        parents.push(self.id);
        Self::register(self.sink.clone(), id, parents)
    }

    fn send(&self, state: ReportState) {
        // Add/Max 增量与文案聚合到祖先；Set 绝对值和隐藏/移除只作用于自身，
        // 避免子进度重置分母或把仍在运行的父进度条弹掉。
        match state {
            ReportState::SetMessage(msg) => {
                self.update_all(|p| {
                    p.message = Some(msg.clone());
                    p.message_dirty = true;
                });
            }
            ReportState::SetSubMessage(msg) => {
                self.update_all(|p| {
                    p.sub_message = Some(msg.clone());
                    p.sub_message_dirty = true;
                });
            }
            ReportState::AddMaxProgress(value) => {
                self.update_all(|p| {
                    p.max += value;
                    p.max_dirty = true;
                    p.indeterminate = false;
                    p.indeterminate_dirty = false;
                });
            }
            ReportState::AddProgress(value) => {
                self.update_all(|p| {
                    p.current += value;
                    p.current_dirty = true;
                    p.indeterminate = false;
                    p.indeterminate_dirty = false;
                });
            }
            ReportState::SetMaxProgress(value) => {
                self.update_self(|p| {
                    p.max = value;
                    p.max_dirty = true;
                    p.indeterminate = false;
                    p.indeterminate_dirty = false;
                });
            }
            ReportState::SetProgress(value) => {
                self.update_self(|p| {
                    p.current = value;
                    p.current_dirty = true;
                    p.indeterminate = false;
                    p.indeterminate_dirty = false;
                });
            }
            ReportState::SetIndeterminateProgress | ReportState::HideProgress => {
                self.update_self(|p| {
                    p.indeterminate = true;
                    p.indeterminate_dirty = true;
                });
            }
            ReportState::RemoveProgress => {
                self.remove_self();
            }
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if !self.removed && self.owner {
            self.remove_self();
        }
    }
}
