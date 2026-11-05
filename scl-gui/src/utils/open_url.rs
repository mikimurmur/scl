//! 打开链接或本地路径的跨平台工具。

use anyhow::Context;

/// 使用系统默认方式打开一个链接或本地路径。
pub fn open_url(target: &str) -> anyhow::Result<()> {
    if target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
    {
        webbrowser::open(target).context("无法打开浏览器")?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(target)
            .spawn()
            .context("无法打开路径")?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(target)
            .spawn()
            .context("无法打开路径")?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(target)
            .spawn()
            .context("无法打开路径")?;
    }

    Ok(())
}
