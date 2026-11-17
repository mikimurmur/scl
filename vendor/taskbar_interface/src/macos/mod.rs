use std::sync::Arc;

use icrate::{
    objc2::{rc::Id, ClassType},
    AppKit::{
        NSApplication, NSDockTile, NSImageView, NSInformationalRequest, NSProgressIndicator,
        NSProgressIndicatorStyleBar,
    },
    Foundation::{NSPoint, NSRect, NSSize},
};
use once_cell::sync::Lazy;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::ProgressIndicatorState;

static GLOBAL_TASKBAR_INDICATOR_INNER: Lazy<Arc<TaskbarIndicatorInner>> =
    Lazy::new(|| Arc::new(TaskbarIndicatorInner::new()));

struct TaskbarIndicatorInner {
    progress_indicator_view: Id<NSProgressIndicator>,
    dock_tile: Id<NSDockTile>,
}

unsafe impl Send for TaskbarIndicatorInner {}
unsafe impl Sync for TaskbarIndicatorInner {}

impl TaskbarIndicatorInner {
    fn new() -> Self {
        unsafe {
            let app = NSApplication::sharedApplication();
            let dock_tile = app.dockTile();
            let prog = NSProgressIndicator::initWithFrame(
                NSProgressIndicator::alloc(),
                NSRect::new(
                    NSPoint::new(0., 0.),
                    NSSize::new(dock_tile.size().width, 10.),
                ),
            );
            prog.setStyle(NSProgressIndicatorStyleBar);
            prog.setBezeled(true);
            prog.setHidden(false);
            prog.setIndeterminate(true);
            prog.setUsesThreadedAnimation(true);
            prog.setDisplayedWhenStopped(true);
            prog.setMaxValue(100.);
            prog.setMinValue(0.);
            if let Some(content_view) = dock_tile.contentView() {
                content_view.addSubview(&prog);
            } else {
                let app_img_view = NSImageView::init(NSImageView::alloc());
                if let Some(app_img) = app.applicationIconImage() {
                    app_img_view.setImage(Some(&app_img));
                }
                app_img_view.addSubview(&prog);
                dock_tile.setContentView(Some(&app_img_view));
            }

            Self {
                progress_indicator_view: prog,
                dock_tile,
            }
        }
    }
}

pub struct TaskbarIndicator {
    inner: Arc<TaskbarIndicatorInner>,
}

unsafe impl Send for TaskbarIndicator {}
unsafe impl Sync for TaskbarIndicator {}

impl TaskbarIndicator {
    pub fn new(
        _window: RawWindowHandle,
        _display: RawDisplayHandle,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            inner: GLOBAL_TASKBAR_INDICATOR_INNER.clone(),
        })
    }

    fn update_progress(&mut self) {
        unsafe {
            self.inner.dock_tile.display();
        }
    }

    pub fn set_progress(&mut self, progress: f64) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.inner
                .progress_indicator_view
                .setDoubleValue(progress * 100.);
            self.inner.progress_indicator_view.setIndeterminate(false);
        }
        self.update_progress();
        Ok(())
    }

    pub fn set_progress_state(
        &mut self,
        state: ProgressIndicatorState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            match state {
                ProgressIndicatorState::NoProgress => {
                    self.inner.progress_indicator_view.setHidden(true);
                    self.inner
                        .progress_indicator_view
                        .stopAnimation(Some(&self.inner.progress_indicator_view));
                }
                ProgressIndicatorState::Indeterminate => {
                    self.inner.progress_indicator_view.setHidden(false);
                    self.inner.progress_indicator_view.setIndeterminate(true);
                    self.inner
                        .progress_indicator_view
                        .startAnimation(Some(&self.inner.progress_indicator_view));
                }
                ProgressIndicatorState::Normal => {
                    self.inner.progress_indicator_view.setHidden(false);
                    self.inner.progress_indicator_view.setIndeterminate(false);
                    self.inner
                        .progress_indicator_view
                        .stopAnimation(Some(&self.inner.progress_indicator_view));
                }
                ProgressIndicatorState::Paused => {
                    self.inner.progress_indicator_view.setHidden(false);
                    self.inner.progress_indicator_view.setIndeterminate(false);
                    self.inner
                        .progress_indicator_view
                        .stopAnimation(Some(&self.inner.progress_indicator_view));
                }
                ProgressIndicatorState::Error => {
                    self.inner.progress_indicator_view.setHidden(false);
                    self.inner.progress_indicator_view.setIndeterminate(false);
                    self.inner
                        .progress_indicator_view
                        .stopAnimation(Some(&self.inner.progress_indicator_view));
                }
            }
        }
        self.update_progress();
        Ok(())
    }

    pub fn needs_attention(
        &mut self,
        needs_attention: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if needs_attention {
            unsafe {
                NSApplication::sharedApplication().requestUserAttention(NSInformationalRequest);
            }
        }
        Ok(())
    }
}
