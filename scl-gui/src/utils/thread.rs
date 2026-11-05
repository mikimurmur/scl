//! 一些用于在后台线程运行阻塞任务的工具。
//!
//! 每个后台任务按需 `std::thread::spawn` 一个操作系统线程，任务结束线程自然退出；
//! 不再维护共享的后台线程池。需要大量并发的下载任务由 scl-core 的 64 线程
//! rayon 下载池负责，与这里相互独立。

use std::sync::atomic::{AtomicU64, Ordering};

/// 后台任务序号，仅用于日志区分，不含任何敏感信息。
static NEXT_BACKGROUND_TASK: AtomicU64 = AtomicU64::new(1);

/// 按需创建一个后台线程执行任务。
///
/// 创建线程失败时不能静默吞掉：记录错误并通过原版风格的非致命错误弹窗提示用户。
fn spawn_background_task<F>(location: String, task: F)
where
    F: FnOnce() + Send + 'static,
{
    let task_id = NEXT_BACKGROUND_TASK.fetch_add(1, Ordering::Relaxed);
    let thread_location = location.clone();
    let spawn_result = std::thread::Builder::new()
        // 无名线程：与官方后台线程保持一致，避免线程名干扰日志/诊断。
        .spawn(move || {
            tracing::trace!(
                "后台任务 #{task_id} 在 {:?} 上执行，来源 {thread_location}",
                std::thread::current().id()
            );
            task();
        });
    if let Err(err) = spawn_result {
        tracing::error!("无法为后台任务创建线程（来源 {location}）：{err:?}");
        crate::panic::show_background_error_dialog(
            location,
            format!("无法为后台任务创建线程：{err:?}"),
        );
    }
}

/// 兼容既有调用点：后台线程现在按任务按需创建，不再需要提前启动。
pub fn ensure_background_thread() {}

/// 在一个新的线程中运行阻塞任务
///
/// 这个函数会立即返回，任务的运行结果会通过日志输出。如果需要在任务结束后更新界面，
/// 请使用 [`druid::ExtEventSink`] 发送命令或通知。
#[track_caller]
pub fn thread_main<F>(task: F)
where
    F: FnOnce() -> anyhow::Result<()> + Send + 'static,
{
    let caller = std::panic::Location::caller();
    let location = format!("{}:{}", caller.file(), caller.line());
    let dialog_location = location.clone();
    spawn_background_task(location, move || {
        if let Err(err) = task() {
            // 官方错误路径只经 g_idle_add_full 弹窗，不产生额外日志。
            crate::panic::show_background_error_dialog(dialog_location, format!("{err:#?}"));
        }
    });
}

/// 在一个新的线程中运行阻塞任务，失败时显示原版风格的非致命错误弹窗。
///
/// 与原版 `thread_main` 一致：闭包捕获调用点的 `file:line`，作为弹窗正文中的
/// “执行位于 … 的后台任务”。下载/元数据任务使用本函数；启动期错误仍走
/// `launch_error` 页面，不受影响。
#[track_caller]
pub fn thread_main_report<F>(task: F)
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    let caller = std::panic::Location::caller();
    let location = format!("{}:{}", caller.file(), caller.line());
    let dialog_location = location.clone();
    spawn_background_task(location, move || {
        if let Err(err) = task() {
            // 官方错误路径只经 g_idle_add_full 弹窗，不产生额外日志。
            crate::panic::show_background_error_dialog(dialog_location, err);
        }
    });
}

/// 把一个 `image` 库的图像转换为 Druid 使用的图像缓冲区
pub fn image_buf_from_dynamic(image: image::DynamicImage) -> anyhow::Result<druid::ImageBuf> {
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    Ok(druid::ImageBuf::from_raw(
        image.into_raw(),
        druid::piet::ImageFormat::RgbaSeparate,
        width as usize,
        height as usize,
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// 提交的全部任务都应被执行（每个任务独立线程，不保证执行顺序）。
    #[test]
    fn tasks_all_execute() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let done = Arc::new(Mutex::new(0usize));
        let total = 32;
        for i in 0..total {
            let order = order.clone();
            let done = done.clone();
            thread_main(move || {
                order.lock().unwrap().push(i);
                *done.lock().unwrap() += 1;
                Ok(())
            });
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while *done.lock().unwrap() < total && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let taken = order.lock().unwrap().clone();
        assert_eq!(taken.len(), total, "全部任务应被执行");
        let mut sorted = taken;
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            (0..total).collect::<Vec<_>>(),
            "每个任务应恰好执行一次"
        );
    }

    /// 后台任务运行在独立线程上，与 GUI 主线程互不依赖；
    /// 本地同样无需 GTK/Druid 主循环参与即可执行（GUI 更新走 ExtEventSink）。
    #[test]
    fn tasks_start_without_gui_loop() {
        let (tx, rx) = std::sync::mpsc::channel::<std::time::Instant>();
        let submitted = std::time::Instant::now();
        thread_main(move || {
            let _ = tx.send(std::time::Instant::now());
            Ok(())
        });
        let started = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("任务应在 5 秒内开始执行，且不依赖 GUI 主循环");
        assert!(
            started.duration_since(submitted) < std::time::Duration::from_secs(5),
            "投递延迟应在秒级以内"
        );
    }

    /// 每个任务使用独立线程，长等待任务不会阻塞其它任务。
    #[test]
    fn long_waiting_task_does_not_block_other_tasks() {
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let (other_tx, other_rx) = std::sync::mpsc::channel::<()>();
        thread_main(move || {
            let _ = release_rx.recv_timeout(std::time::Duration::from_secs(5));
            Ok(())
        });
        thread_main(move || {
            let _ = other_tx.send(());
            Ok(())
        });
        other_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("第二个任务不应被长等待任务阻塞");
        let _ = release_tx.send(());
    }

    /// 官方失败任务经 `#[track_caller]` 捕获调用点后弹窗；
    /// 本地同样进入错误弹窗队列，且载荷包含 `file:line` 调用点。
    #[cfg(target_os = "linux")]
    #[test]
    fn failed_task_queues_dialog_with_caller_location() {
        let before = crate::panic::background_error_dialog_queue_snapshot();
        thread_main(|| anyhow::bail!("模拟后台任务失败"));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let after = crate::panic::background_error_dialog_queue_snapshot();
            if after.len() > before.len() {
                let (location, error) = after.last().unwrap();
                assert!(
                    location.ends_with("utils/thread.rs") || location.contains("thread.rs"),
                    "弹窗载荷应包含调用点文件，实际：{location}"
                );
                assert!(
                    location.contains(':'),
                    "调用点应为 file:line 形式：{location}"
                );
                // 消息错误的弹窗文本带引号，使用底层 Debug 形态。
                assert_eq!(error, "\"模拟后台任务失败\"");
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "失败任务应在 5 秒内进入错误弹窗队列"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
