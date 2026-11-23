//! 启动器异常捕获：把 panic 记录到日志，并提供错误内容复制到剪切板的能力。

/// Windows 致命错误弹窗正文
#[cfg(target_os = "windows")]
const PANIC_DIALOG_TEXT: &str = "发生了（可能比较严重的）错误：\n\n\n\n\
省略了栈回溯信息……\n\n\
是否复制错误报告到剪切板？将会收集部分设备信息（目前仅操作系统版本）！\n\
（注：直接将错误报告发入群内，机器人会自动收集错误哦！）";

/// 安装 panic 钩子，把异常信息记录到日志
pub fn install() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        tracing::error!("启动器发生异常：{info}\n{backtrace}");

        // 致命错误时弹出 Windows 消息框：
        // Yes 表示把错误报告复制到剪切板；Linux/macOS 只保留日志钩子。
        #[cfg(target_os = "windows")]
        {
            let report = format!(
                "{info}\n\n操作系统：{} {}\n\n省略了栈回溯信息……",
                std::env::consts::OS,
                std::env::consts::ARCH
            );
            if show_windows_message_box(PANIC_DIALOG_TEXT, "严重错误！") {
                copy_to_clipboard(report);
            }
        }
    }));
}

/// 把错误文本复制到剪切板。
pub fn copy_to_clipboard(text: String) {
    match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
        Ok(()) => {}
        Err(err) => tracing::warn!("写入剪切板失败：{err:?}"),
    }
}

/// 显示原版风格的非致命后台任务错误弹窗。
///
/// 迁移到 Iced 后不再依赖 GTK：错误被推入界面状态，由根视图渲染一个
/// 与旧版文案一致的模态对话框。
pub fn show_async_error_dialog(location: impl Into<String>, error: impl Into<String>) {
    let location = location.into();
    let error = error.into();

    // 测试构建中保留最近错误载荷，供后台任务错误路径的回归测试读取。
    #[cfg(test)]
    {
        if let Ok(mut queue) = ASYNC_ERROR_DIALOGS.lock() {
            queue.push((location.clone(), error.clone()));
            let len = queue.len();
            if len > 32 {
                queue.drain(..len - 32);
            }
        }
    }

    crate::app::report_async_error(location, error);
}

/// 返回最近记录的后台任务错误弹窗载荷（仅测试构建可用）。
#[cfg(test)]
pub fn async_error_dialog_queue_snapshot() -> Vec<(String, String)> {
    ASYNC_ERROR_DIALOGS
        .lock()
        .map(|queue| queue.clone())
        .unwrap_or_default()
}

#[cfg(test)]
static ASYNC_ERROR_DIALOGS: once_cell::sync::Lazy<std::sync::Mutex<Vec<(String, String)>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(Vec::new()));

#[cfg(target_os = "windows")]
fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 显示 Windows 原生 Yes/No 消息框，返回用户是否选择“是”。
///
/// 致命错误弹窗使用这个最小 FFI 包装。
#[cfg(target_os = "windows")]
fn show_windows_message_box(text: &str, caption: &str) -> bool {
    use std::ffi::c_void;

    const MB_YESNO: u32 = 0x0000_0004;
    const IDYES: i32 = 6;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(
            h_wnd: *mut c_void,
            lp_text: *const u16,
            lp_caption: *const u16,
            u_type: u32,
        ) -> i32;
    }

    let text = to_wide(text);
    let caption = to_wide(caption);
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_YESNO,
        ) == IDYES
    }
}
