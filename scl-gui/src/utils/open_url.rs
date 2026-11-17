//! 打开链接或本地路径的跨平台工具。

#[cfg(not(target_os = "windows"))]
use std::process::Command;

#[cfg(not(target_os = "windows"))]
use anyhow::Context;

/// 根据平台构造打开目标的命令。
#[cfg(target_os = "macos")]
fn command_for(target: &str) -> Command {
    let mut command = Command::new("open");
    command.arg(target);
    command
}

#[cfg(all(unix, not(target_os = "macos")))]
fn command_for(target: &str) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(target);
    command
}

/// 启动子进程并异步回收。
///
/// `xdg-open` 之类的启动器会很快退出；只 `spawn` 不 `wait` 会在启动器进程
/// 生命周期内留下僵尸进程（问题 ⑥）。这里用一个短生命周期线程等待子进程
/// 退出，既不影响界面响应，也不会累积僵尸。
#[cfg(not(target_os = "windows"))]
fn spawn_and_reap(mut command: Command) -> anyhow::Result<u32> {
    let mut child = command.spawn().context("无法打开链接或路径")?;
    let pid = child.id();
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(pid)
}

/// 使用系统默认方式打开一个链接或本地路径。
#[cfg(not(target_os = "windows"))]
pub fn open_url(target: &str) -> anyhow::Result<()> {
    spawn_and_reap(command_for(target))?;
    Ok(())
}

/// 使用 Windows Shell 的默认关联程序打开链接或本地路径。
///
/// 不能再用 `cmd /C start "" <target>`：目标里只要包含 `&`、`|`、`^` 等
/// cmd 元字符就会被当成命令解析（更新日志或模组文件名都可能包含这些字符）。
/// `ShellExecuteW` 直接交给资源管理器/默认浏览器处理，也避免弹出控制台窗口。
#[cfg(target_os = "windows")]
pub fn open_url(target: &str) -> anyhow::Result<()> {
    use windows::{
        core::PCWSTR,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };

    let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW 的返回值大于 32 才表示成功，2/3/5 等是 SE_ERR_*。
    let code = result.0 as isize;
    if code <= 32 {
        anyhow::bail!("ShellExecuteW 无法打开 {}（错误码 {}）", target, code);
    }
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::{
        path::Path,
        time::{Duration, Instant},
    };

    use super::{command_for, spawn_and_reap};

    #[test]
    fn linux_uses_xdg_open() {
        assert_eq!(command_for("https://example.com").get_program(), "xdg-open");
        assert_eq!(command_for("/tmp/some-file").get_program(), "xdg-open");
    }

    #[test]
    fn spawned_child_is_reaped() {
        let pid = spawn_and_reap(std::process::Command::new("/bin/true")).expect("启动子进程");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            // 僵尸进程仍会保留 /proc/<pid>；被 wait 回收后目录消失。
            if !Path::new(&format!("/proc/{pid}")).exists() {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "子进程 {pid} 在 5 秒内未被回收，仍存在 /proc/{pid}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
