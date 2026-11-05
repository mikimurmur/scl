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
                copy_to_clipboard_windows(&report);
            }
        }
    }));
}

/// 把错误文本复制到剪切板。
///
/// 错误对话框会提示“这个错误不会使程序崩溃，但是可能会重要，
/// 是否将此错误复制到剪切板？”，用户确认后调用本函数。
/// Windows 使用与官方一致的原生 Win32 剪切板，其余平台复用 druid。
pub fn copy_to_clipboard(text: String) {
    #[cfg(target_os = "windows")]
    {
        copy_to_clipboard_windows(&text);
    }
    #[cfg(not(target_os = "windows"))]
    {
        druid::Application::global().clipboard().put_string(&text);
    }
}

/// 显示原版风格的非致命后台任务错误弹窗。
#[cfg(target_os = "linux")]
pub fn show_background_error_dialog(location: impl Into<String>, error: impl Into<String>) {
    background_error_dialogs()
        .lock()
        .unwrap()
        .push_back((location.into(), error.into()));
    schedule_next_background_error_dialog();
}

#[cfg(target_os = "linux")]
fn background_error_dialogs() -> &'static std::sync::Mutex<std::collections::VecDeque<(String, String)>>
{
    static BACKGROUND_ERROR_DIALOGS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::VecDeque<(String, String)>>,
    > = std::sync::OnceLock::new();
    BACKGROUND_ERROR_DIALOGS.get_or_init(|| std::sync::Mutex::new(std::collections::VecDeque::new()))
}

/// 测试专用：当前错误弹窗队列的快照（位置, 错误）。
#[cfg(all(test, target_os = "linux"))]
pub(crate) fn background_error_dialog_queue_snapshot() -> Vec<(String, String)> {
    background_error_dialogs()
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .collect()
}

#[cfg(target_os = "linux")]
static BACKGROUND_ERROR_DIALOG_OPEN: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "linux")]
thread_local! {
    /// 保证 GTK 对话框在显示期间不会被 Rust 包装对象提前释放。
    static CURRENT_BACKGROUND_ERROR_DIALOG: std::cell::RefCell<Option<gtk::MessageDialog>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(target_os = "linux")]
fn schedule_next_background_error_dialog() {
    use std::sync::atomic::Ordering;

    if BACKGROUND_ERROR_DIALOG_OPEN.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = gtk::glib::idle_add_once(show_next_background_error_dialog);
}

#[cfg(target_os = "linux")]
fn show_next_background_error_dialog() {
    use std::sync::atomic::Ordering;

    use gtk::prelude::*;

    let Some((location, error)) = background_error_dialogs().lock().unwrap().pop_front() else {
        BACKGROUND_ERROR_DIALOG_OPEN.store(false, Ordering::SeqCst);
        return;
    };

    let body = format!(
        "执行位于 {location} 的后台任务时发生错误：\n{error}\n\n\
         这个错误不会使程序崩溃，但是可能会重要，是否将此错误复制到剪切板？"
    );
    let dialog = gtk::MessageDialog::new(
        None::<&gtk::Window>,
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Info,
        gtk::ButtonsType::YesNo,
        &body,
    );
    dialog.set_title("执行后台任务发生错误");
    dialog.set_modal(true);

    // 找到 Druid 主窗口并设为父窗口，确保模态行为与原版一致。
    let parent = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk::Window>().ok())
        .find(|window| {
            window
                .title()
                .map(|title| title.as_str() == "Sharp Craft Launcher")
                .unwrap_or(false)
        });
    if let Some(parent) = parent {
        dialog.set_transient_for(Some(&parent));
    }

    {
        let report = error;
        let handled = std::rc::Rc::new(std::cell::Cell::new(false));
        dialog.connect_response({
            let handled = handled.clone();
            move |dialog, response| {
                if handled.replace(true) {
                    return;
                }
                tracing::debug!("后台任务错误弹窗响应: {response:?}");
                CURRENT_BACKGROUND_ERROR_DIALOG.with(|current| {
                    *current.borrow_mut() = None;
                });
                dialog.close();
                if response == gtk::ResponseType::Yes {
                    tracing::debug!("复制后台任务错误到剪切板：{report}");
                    copy_to_clipboard(report.clone());
                }
                BACKGROUND_ERROR_DIALOG_OPEN.store(false, Ordering::SeqCst);
                schedule_next_background_error_dialog();
            }
        });
    }
    dialog.show_all();
    // No/Yes 按钮不显示蓝色焦点框；GTK 默认会把焦点放到第一个按钮，
    // 这里清空焦点。
    dialog.set_focus(None::<&gtk::Widget>);
    CURRENT_BACKGROUND_ERROR_DIALOG.with(|current| {
        *current.borrow_mut() = Some(dialog);
    });
}

/// 显示原版 Windows 的非致命后台任务错误弹窗。
///
/// Windows 弹窗使用 `MB_YESNO`，无图标、非置顶；
/// 返回 `IDYES` 时把错误文本写入系统剪切板。
/// 剪切板直接调用 `GlobalAlloc`/`OpenClipboard`/`SetClipboardData`，
/// 避免在后台线程调用需要主线程断言的 `Application::global()`。
#[cfg(target_os = "windows")]
pub fn show_background_error_dialog(location: impl Into<String>, error: impl Into<String>) {
    let location = location.into();
    let error = error.into();
    let body = format!(
        "执行位于 {location} 的后台任务时发生错误：\n{error}\n\n\
         这个错误不会使程序崩溃，但是可能会重要，是否将此错误复制到剪切板？"
    );
    if show_windows_message_box(&body, "执行后台任务发生错误") {
        copy_to_clipboard(error);
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn show_background_error_dialog(location: impl Into<String>, error: impl Into<String>) {
    tracing::warn!(
        "后台任务发生错误（{}）：{}",
        location.into(),
        error.into()
    );
}

#[cfg(target_os = "windows")]
fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 显示 Windows 原生 Yes/No 消息框，返回用户是否选择“是”。
///
/// 致命错误弹窗与后台任务错误弹窗共用这一个最小 FFI 包装。
#[cfg(target_os = "windows")]
fn show_windows_message_box(text: &str, caption: &str) -> bool {
    use std::ffi::c_void;

    const MB_YESNO: u32 = 0x0000_0004;
    const IDYES: i32 = 6;

    #[link(name = "user32")]
    extern "system" {
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

/// 使用官方 Windows 原生剪贴板 API 写入 UTF-16 文本
#[cfg(target_os = "windows")]
fn copy_to_clipboard_windows(text: &str) {
    type HGlobal = isize;
    const CF_UNICODETEXT: u32 = 13;
    const GMEM_MOVEABLE: u32 = 0x0000_0002;

    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(h_wnd_new_owner: isize) -> i32;
        fn EmptyClipboard() -> i32;
        fn CloseClipboard() -> i32;
        fn SetClipboardData(u_format: u32, h_mem: HGlobal) -> HGlobal;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalAlloc(u_flags: u32, dw_bytes: usize) -> HGlobal;
        fn GlobalLock(h_mem: HGlobal) -> *mut std::ffi::c_void;
        fn GlobalUnlock(h_mem: HGlobal) -> i32;
        fn GlobalFree(h_mem: HGlobal) -> HGlobal;
    }

    unsafe {
        if OpenClipboard(0) == 0 {
            tracing::warn!("打开剪切板失败，无法复制错误报告");
            return;
        }
        let _ = EmptyClipboard();
        let wide = to_wide(text);
        let size = wide.len() * std::mem::size_of::<u16>();
        let handle = GlobalAlloc(GMEM_MOVEABLE, size);
        if handle != 0 {
            let ptr = GlobalLock(handle) as *mut u16;
            if !ptr.is_null() {
                std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
                let _ = GlobalUnlock(handle);
                if SetClipboardData(CF_UNICODETEXT, handle) == 0 {
                    let _ = GlobalFree(handle);
                }
            } else {
                let _ = GlobalFree(handle);
            }
        }
        let _ = CloseClipboard();
    }
}
