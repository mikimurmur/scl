//! Windows 父控制台连接。
//!
//! 发布版是 GUI 子系统程序，从终端启动时默认没有 stdio 控制台。
//! 这里按官方 Windows 导入表使用 `FreeConsole` / `AttachConsole` /
//! `CreateFileW("CONOUT$")` / `SetStdHandle` 把 stdout、stderr 接回
//! 父控制台，并读取控制台模式与输出代码页供 tracing 初始化使用。

use std::{
    ffi::{c_void, OsStr},
    os::windows::ffi::OsStrExt,
};

type Handle = isize;

const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
const STD_ERROR_HANDLE: u32 = -12i32 as u32;
const GENERIC_WRITE: u32 = 0x4000_0000;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const OPEN_EXISTING: u32 = 3;
const INVALID_HANDLE_VALUE: Handle = -1;

/// 控制台支持 ANSI 转义序列的模式位
pub const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

#[link(name = "kernel32")]
extern "system" {
    fn FreeConsole() -> i32;
    fn AttachConsole(dw_process_id: u32) -> i32;
    fn GetStdHandle(n_std_handle: u32) -> Handle;
    fn SetStdHandle(n_std_handle: u32, h_handle: Handle) -> i32;
    fn CreateFileW(
        lp_file_name: *const u16,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *mut c_void,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: Handle,
    ) -> Handle;
    fn GetConsoleMode(h_console_handle: Handle, lp_mode: *mut u32) -> i32;
    fn GetConsoleOutputCP() -> u32;
}

/// 父控制台连接结果
pub struct ConsoleInfo {
    /// `GetConsoleMode` 读取到的控制台模式
    pub mode: u32,
    /// `GetConsoleOutputCP` 读取到的输出代码页
    pub output_cp: u32,
}

/// 把当前进程的 stdout/stderr 接回父控制台
///
/// 返回 `None` 表示当前不是从终端启动，或父进程没有控制台。
pub fn attach_parent_console() -> Option<ConsoleInfo> {
    unsafe {
        FreeConsole();
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            return None;
        }
        let conout = OsStr::new("CONOUT$")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let handle = CreateFileW(
            conout.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            0,
        );
        if handle != INVALID_HANDLE_VALUE {
            SetStdHandle(STD_OUTPUT_HANDLE, handle);
            SetStdHandle(STD_ERROR_HANDLE, handle);
        }

        let mut mode = 0u32;
        if GetConsoleMode(GetStdHandle(STD_ERROR_HANDLE), &mut mode) == 0 {
            mode = 0;
        }
        Some(ConsoleInfo {
            mode,
            output_cp: GetConsoleOutputCP(),
        })
    }
}
