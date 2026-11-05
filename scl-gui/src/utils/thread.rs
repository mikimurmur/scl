//! 一些用于在后台线程运行异步任务的工具。

use std::{future::Future, sync::Once};

use once_cell::sync::Lazy;

/// 所有后台任务共用的异步执行器，对应原版的单一后台线程（日志中的 `ThreadId(2)`）。
static EXECUTOR: Lazy<smol::Executor<'static>> = Lazy::new(smol::Executor::new);
static EXECUTOR_STARTED: Once = Once::new();

/// 启动后台线程（重复调用无副作用），并输出与原版一致的线程启动日志。
///
/// 官方线程经 `std::thread::Builder::new()` 直接 spawn（无名，`ThreadId(2)`），
/// 句柄记录在 `THREAD_MANAGER` 内；这里保持无名以对齐 panic 输出与 `/proc` 视图。
pub fn ensure_background_thread() {
    EXECUTOR_STARTED.call_once(|| {
        let _ = std::thread::Builder::new().spawn(|| {
            tracing::info!("后台线程 {:?} 已启动", std::thread::current().id());
            smol::block_on(EXECUTOR.run(std::future::pending::<()>()));
        });
    });
}

/// 在一个新的线程中运行异步任务
///
/// 这个函数会立即返回，任务的运行结果会通过日志输出。如果需要在任务结束后更新界面，
/// 请使用 [`druid::ExtEventSink`] 发送命令或通知。
#[track_caller]
pub fn thread_main<F>(future: F)
where
    F: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    ensure_background_thread();
    let caller = std::panic::Location::caller();
    let location = format!("{}:{}", caller.file(), caller.line());
    EXECUTOR
        .spawn(async move {
            tracing::trace!("正在执行来自 {location} 的任务");
            if let Err(err) = future.await {
                // 官方错误路径只经 g_idle_add_full 弹窗，不产生额外日志。
                crate::panic::show_async_error_dialog(location, format!("{err:#?}"));
            }
        })
        .detach();
}

/// 在一个新的线程中运行异步任务，失败时显示原版风格的非致命错误弹窗。
///
/// 与原版 `thread_main` 一致：闭包捕获调用点的 `file:line`，作为弹窗正文中的
/// “执行位于 … 的后台异步任务”。下载/元数据任务使用本函数；启动期错误仍走
/// `launch_error` 页面，不受影响。
#[track_caller]
pub fn thread_main_report<F>(future: F)
where
    F: Future<Output = Result<(), String>> + Send + 'static,
{
    ensure_background_thread();
    let caller = std::panic::Location::caller();
    let location = format!("{}:{}", caller.file(), caller.line());
    EXECUTOR
        .spawn(async move {
            tracing::trace!("正在执行来自 {location} 的任务");
            if let Err(err) = future.await {
                // 官方错误路径只经 g_idle_add_full 弹窗，不产生额外日志。
                crate::panic::show_async_error_dialog(location, err);
            }
        })
        .detach();
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

    /// 消息同步压入无界队列后返回，后台线程按接收顺序 spawn 到执行器，
    /// 即提交顺序 = 执行顺序（FIFO）。
    #[test]
    fn tasks_run_in_submission_order() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let done = Arc::new(Mutex::new(0usize));
        let total = 32;
        for i in 0..total {
            let order = order.clone();
            let done = done.clone();
            thread_main(async move {
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
        assert_eq!(
            taken,
            (0..total).collect::<Vec<_>>(),
            "任务应按提交顺序执行"
        );
    }

    /// 官方任务运行在独立后台线程上，与 GUI 主线程互不依赖；
    /// 本地同样无需 GTK/Druid 主循环参与即可执行（GUI 更新走 ExtEventSink）。
    #[test]
    fn tasks_start_without_gui_loop() {
        let (tx, rx) = std::sync::mpsc::channel::<std::time::Instant>();
        let submitted = std::time::Instant::now();
        thread_main(async move {
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

    /// 官方失败任务经 `#[track_caller]` 捕获调用点后弹窗；
    /// 本地同样进入错误弹窗队列，且载荷包含 `file:line` 调用点。
    #[cfg(target_os = "linux")]
    #[test]
    fn failed_task_queues_dialog_with_caller_location() {
        let before = crate::panic::async_error_dialog_queue_snapshot();
        thread_main(async move { anyhow::bail!("模拟后台任务失败") });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let after = crate::panic::async_error_dialog_queue_snapshot();
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
