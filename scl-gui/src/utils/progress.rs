//! 将 [`scl_core::progress`] 的进度报告转换为 Iced 界面消息。

use std::sync::atomic::{AtomicUsize, Ordering};

use scl_core::progress::{ReportState, Reporter};

use crate::app::{EventSink, Message, ProgressMessage};

/// 进度编号生成器
static NEXT_PROGRESS_ID: AtomicUsize = AtomicUsize::new(1);

/// 一个将进度上报到界面的报告器
#[derive(Clone)]
pub struct Progress {
    id: usize,
    sink: EventSink,
    /// 祖先进度编号链（顶层在前、直接父级在后）
    parents: Vec<usize>,
    /// 是否已经被移除
    removed: bool,
}

impl Progress {
    /// 创建一个新的进度
    pub fn new(sink: EventSink) -> Self {
        let id = NEXT_PROGRESS_ID.fetch_add(1, Ordering::Relaxed);
        sink.send(Message::Progress(ProgressMessage::New(id)));
        Self {
            id,
            sink,
            parents: Vec::new(),
            removed: false,
        }
    }

    /// 向本进度与所有祖先进度发送同一条消息。
    ///
    /// 祖先链复刻官方 `sub()`：子任务上报的 Set/Add 进度会同步写入父进度，
    /// 父进度条不再静止不动。
    fn send_to_all(&self, payload: impl Fn(usize) -> ProgressMessage) {
        if self.removed {
            return;
        }
        for id in &self.parents {
            self.sink.send(Message::Progress(payload(*id)));
        }
        self.sink.send(Message::Progress(payload(self.id)));
    }

    /// 只向本进度发送消息
    fn send_self(&self, payload: ProgressMessage) {
        if self.removed {
            return;
        }
        self.sink.send(Message::Progress(payload));
    }

    /// 移除这个进度
    pub fn remove(mut self) {
        self.removed = true;
        // 只移除自身：父进度由创建它的任务负责移除，子进度提前结束不能
        // 连带弹掉仍在运行的父任务进度条。
        self.sink
            .send(Message::Progress(ProgressMessage::Remove(self.id)));
    }
}

impl Reporter for Progress {
    fn fork(&self) -> Self {
        // fork 是独立进度，不参与父进度合计（官方默认 fork 语义）。
        Self::new(self.sink.clone())
    }

    fn sub(&self) -> Self {
        let id = NEXT_PROGRESS_ID.fetch_add(1, Ordering::Relaxed);
        self.sink.send(Message::Progress(ProgressMessage::New(id)));
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
                self.send_to_all(|id| ProgressMessage::SetMessage(id, msg.clone()));
            }
            ReportState::SetSubMessage(msg) => {
                self.send_to_all(|id| ProgressMessage::SetSubMessage(id, msg.clone()));
            }
            ReportState::SetMaxProgress(value) => {
                self.send_to_all(|id| ProgressMessage::SetMax(id, value));
            }
            ReportState::AddMaxProgress(value) => {
                self.send_to_all(|id| ProgressMessage::AddMax(id, value));
            }
            ReportState::SetProgress(value) => {
                self.send_to_all(|id| ProgressMessage::Set(id, value));
            }
            ReportState::AddProgress(value) => {
                self.send_to_all(|id| ProgressMessage::Add(id, value));
            }
            ReportState::SetIndeterminateProgress | ReportState::HideProgress => {
                self.send_self(ProgressMessage::Hide(self.id));
            }
            ReportState::RemoveProgress => {
                self.send_self(ProgressMessage::Remove(self.id));
            }
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if !self.removed {
            self.sink
                .send(Message::Progress(ProgressMessage::Remove(self.id)));
        }
    }
}
