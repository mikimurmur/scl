//! 启动器更新检查。

use anyhow::Context;
use serde::Deserialize;

const UPDATE_URLS: &[&str] = &[
    "https://gitcode.net/sn/scl-update/-/raw/main/latest",
    "https://raw.githubusercontent.com/Steve-xmh/scl-update/main/latest",
];

/// 更新记录 URL 前缀：`{前缀}{版本号}`。
const UPDATE_LOGS_URL: &str = "https://gitcode.net/sn/scl-update/-/raw/main/logs/";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
}

fn normalize_version(version: &str) -> String {
    version
        .trim()
        .trim_start_matches('v')
        .trim_start_matches("SCL-")
        .to_owned()
}

/// 判断响应体是否像一个版本号。
///
/// 更新源被 WAF/CDN 拦截时会返回整页 HTML 或错误文案（官方不校验，
/// OB-17 的恶性表现，已按用户指令转为修复，见 DEV-12），这里按
/// 「纯版本号」形态校验：非空、长度合理、仅含 semver 常见字符。
fn is_version_like(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'))
}

/// 获取最新版本号
pub fn latest_version() -> anyhow::Result<String> {
    let mut last_error = None;
    for url in UPDATE_URLS {
        match scl_core::http::retry_get(url) {
            Ok(response) => {
                // 不校验状态码会把 WAF/CDN 拦截页当版本号（OB-17 表现），先验状态码
                if !response.is_success() {
                    tracing::error!(
                        "无法获取最新版本号：{url} 返回了非成功的状态码 {}",
                        response.status()
                    );
                    last_error = Some(anyhow::anyhow!(
                        "{url} 返回了非成功的状态码 {}",
                        response.status()
                    ));
                    continue;
                }
                let Ok(text) = response.into_string() else {
                    let err = anyhow::anyhow!("{url} 响应体无法按文本解码");
                    tracing::error!("无法获取最新版本号：{err}");
                    last_error = Some(err);
                    continue;
                };
                let trimmed = text.trim();
                // 恢复版额外兼容 GitHub Release JSON（官方 1.2.0 只认纯文本）；
                // 解析成功时仍返回干净的版本号，避免正常路径回归。
                if trimmed.starts_with('{') {
                    if let Ok(release) = serde_json::from_str::<GithubRelease>(trimmed) {
                        return Ok(normalize_version(&release.tag_name));
                    }
                }
                // 响应体必须形如版本号：拦截页/错误页 HTML 一律视为失败并换下一个源
                if !is_version_like(trimmed) {
                    tracing::error!(
                        "无法获取最新版本号：{url} 返回了非版本号内容（可能被网络拦截），已忽略"
                    );
                    last_error = Some(anyhow::anyhow!(
                        "{url} 返回了非版本号内容（可能被网络拦截）"
                    ));
                    continue;
                }
                return Ok(trimmed.to_owned());
            }
            Err(err) => {
                tracing::error!("无法获取最新版本号：{url}");
                last_error = Some(err);
            }
        }
    }
    Err(last_error
        .map(|x| anyhow::anyhow!("{x:?}"))
        .unwrap_or_else(|| anyhow::anyhow!("没有可用的更新检查链接")))
}

/// 拉取指定版本的更新记录；失败时返回空字符串并记录官方文案的错误日志。
pub fn fetch_update_logs(version: &str) -> String {
    let url = format!("{UPDATE_LOGS_URL}{version}");
    match scl_core::http::retry_get(&url) {
        Ok(response) => {
            if !response.is_success() {
                tracing::error!(
                    "无法获取版本更新记录：请求的链接返回了非成功的状态码：{}",
                    response.status()
                );
                return String::new();
            }
            match response.into_string() {
                Ok(logs) => logs,
                Err(err) => {
                    tracing::error!("无法获取版本更新记录：{err}");
                    String::new()
                }
            }
        }
        Err(err) => {
            tracing::error!("无法获取版本更新记录：{err}");
            String::new()
        }
    }
}

/// 当前平台的更新文件在更新仓库 `downloads/` 下的名称
pub fn platform_download_target() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows-x86_64.exe"
    }
    #[cfg(target_os = "macos")]
    {
        "universal-darwin"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "linux-gnu-x86_64"
    }
}

/// 更新用的临时目录名，位于当前执行程序旁边
pub const UPDATE_TEMP_DIR: &str = "scl-latest";

/// 下载最新版本的 SCL 并交给更新器覆盖当前程序。
///
/// 成功后本函数不会返回，更新器会接管覆盖流程并重启新版本启动器。
pub fn self_update() -> anyhow::Result<()> {
    use anyhow::Context;

    tracing::info!("正在下载最新版本的 SCL");
    let target = platform_download_target();
    let urls = [
        format!("https://gitcode.net/sn/scl-update/-/raw/main/downloads/{target}"),
        format!("https://raw.githubusercontent.com/Steve-xmh/scl-update/main/downloads/{target}"),
    ];
    let mut last_error = None;
    let data = 'download: {
        for url in urls {
            match scl_core::http::retry_get_bytes(&url) {
                Ok(data) => break 'download data,
                Err(err) => {
                    tracing::error!(
                        "无法更新 SCL：请求的链接返回了非成功的状态码或请求失败：{err:?}"
                    );
                    last_error = Some(err);
                }
            }
        }
        return Err(last_error
            .map(|x| anyhow::anyhow!("{x:?}"))
            .unwrap_or_else(|| anyhow::anyhow!("没有可用的更新下载链接")));
    };

    let exe = std::env::current_exe().context("无法获取当前可执行文件路径")?;
    let temp_dir = exe.with_file_name(UPDATE_TEMP_DIR);
    fs_err::create_dir_all(&temp_dir)
        .with_context(|| format!("创建更新临时目录 {} 失败", temp_dir.display()))?;
    let new_file = temp_dir.join(
        exe.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "scl-new".to_owned()),
    );
    // 上一个更新器可能还在退出过程中，先清理目标文件再原子写入
    let _ = fs_err::remove_file(&new_file);
    crate::utils::prelude::write_file_safe(&new_file, &data)
        .with_context(|| format!("写入更新文件 {} 失败", new_file.display()))?;

    #[cfg(unix)]
    {
        let meta = fs_err::metadata(&exe).context("获取当前执行程序文件元数据失败")?;
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode() & 0o777;
        fs_err::set_permissions(&new_file, std::fs::Permissions::from_mode(mode))
            .context("复制可执行权限到更新临时文件失败")?;
    }

    // 把自身复制成更新器，由它来覆盖正在运行的程序文件
    let updater_name = if cfg!(target_os = "windows") {
        "scl-updater.exe"
    } else {
        "scl-updater"
    };
    let updater = temp_dir.join(updater_name);
    copy_and_update(&exe, &updater, &new_file)?;
    Ok(())
}

/// 把当前启动器复制为更新器，并交给它执行两阶段覆盖。
///
/// 对应官方 `utils::update::update::copy_and_update` 的职责：
/// 使用 `copy_file_safe` 原子复制当前可执行文件，然后启动
/// `scl-updater --update <新文件> <目标文件>`。
fn copy_and_update(
    exe: &std::path::Path,
    updater: &std::path::Path,
    new_file: &std::path::Path,
) -> anyhow::Result<()> {
    // Windows 上 rename 不能覆盖已存在的更新器文件
    #[cfg(target_os = "windows")]
    let _ = fs_err::remove_file(updater);
    crate::utils::prelude::copy_file_safe(exe, updater).context("无法启动更新器")?;
    std::process::Command::new(updater)
        .arg("--update")
        .arg(new_file)
        .arg(exe)
        .spawn()
        .context("无法启动更新器")?;
    Ok(())
}

/// 以更新器身份运行：等待目标程序退出后完成覆盖并重启新版本。
///
/// 用法：`scl-updater --update <新文件路径> <目标文件路径>`
pub fn run_updater(args: &[String]) -> anyhow::Result<()> {
    let new_file = args.first().context("没有提供更新文件路径")?;
    let target = args.get(1).context("没有提供目标文件路径")?;
    let new_file = std::path::Path::new(new_file);
    let target = std::path::Path::new(target);

    // 正在运行的程序文件在 Windows 上无法被覆盖，重试直到成功
    let old_file = {
        let mut name = target
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        name.push_str(".scl-old");
        target.with_file_name(name)
    };
    loop {
        let _ = fs_err::remove_file(&old_file);
        match fs_err::rename(target, &old_file).and_then(|()| fs_err::rename(new_file, target)) {
            Ok(()) => break,
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        }
    }

    // 新版本启动时负责清理临时文件
    std::process::Command::new(target)
        .arg("--post-update")
        .arg(&old_file)
        .spawn()?;
    Ok(())
}

/// 更新完成后清理旧版本文件与更新临时目录
pub fn run_post_updater(args: &[String]) -> anyhow::Result<()> {
    let old_file = args.first().context("没有提供目标文件路径")?;
    let old_file = std::path::Path::new(old_file);
    let _ = fs_err::remove_file(old_file);
    if let Some(dir) = old_file.parent() {
        if dir
            .file_name()
            .map(|name| name == UPDATE_TEMP_DIR)
            .unwrap_or(false)
        {
            let _ = fs_err::remove_dir_all(dir);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_version_like, normalize_version};

    #[test]
    fn accepts_plain_version_strings() {
        assert!(is_version_like("1.2.0"));
        assert!(is_version_like("26.2"));
        assert!(is_version_like("1.21.8"));
        assert!(is_version_like("v1.2.3"));
        assert!(is_version_like("SCL-1.2.0"));
        assert!(is_version_like("1.2.3+build.5"));
    }

    #[test]
    fn rejects_html_and_message_payloads() {
        // 更新源被 WAF/CDN 拦截时返回的 HTML 不能再被当成版本号渲染（问题 ②）。
        assert!(!is_version_like(
            "<!DOCTYPE html><html><body>400 Bad Request</body></html>"
        ));
        assert!(!is_version_like("error: request blocked"));
        assert!(!is_version_like(""));
        assert!(!is_version_like(&"9".repeat(65)));
    }

    #[test]
    fn normalizes_v_and_scl_prefixes() {
        assert_eq!(normalize_version(" v1.2.3\n"), "1.2.3");
        assert_eq!(normalize_version("SCL-1.2.0"), "1.2.0");
        assert_eq!(normalize_version("26.2"), "26.2");
    }
}
