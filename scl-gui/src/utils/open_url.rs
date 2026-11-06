//! 打开链接或本地路径的跨平台工具。

use std::process::Command;

use anyhow::Context;

/// 根据平台构造打开目标的命令。
#[cfg(target_os = "windows")]
fn command_for(target: &str) -> Command {
    let mut command = Command::new("cmd");
    // `start` 的第一个引号参数是窗口标题，必须保留空串占位。
    command.arg("/C").arg("start").arg("").arg(target);
    command
}

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
fn spawn_and_reap(mut command: Command) -> anyhow::Result<u32> {
    let mut child = command.spawn().context("无法打开链接或路径")?;
    let pid = child.id();
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(pid)
}

/// 使用系统默认方式打开一个链接或本地路径。
pub fn open_url(target: &str) -> anyhow::Result<()> {
    spawn_and_reap(command_for(target))?;
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
