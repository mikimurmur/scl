//! 一些常用的路径，用于跨平台统一
use std::sync::OnceLock;

/// 默认的 Minecraft 主目录
///
/// 如果是 Windows 系统，这将指向当前工作目录的 `.minecraft` 文件夹
///
/// 如果是 Linux 系统，这将尝试指向当前主目录（Home Directory）的 `.minecraft` 文件夹
pub(crate) fn minecraft_path() -> &'static str {
    static MINECRAFT_PATH: OnceLock<String> = OnceLock::new();
    MINECRAFT_PATH
        .get_or_init(|| {
            #[cfg(target_os = "windows")]
            {
                ".minecraft".into()
            }
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            {
                if let Some(dir) = dirs::home_dir() {
                    dir.join(".minecraft").to_str().unwrap().to_string()
                } else {
                    "~/.minecraft".into()
                }
            }
        })
        .as_str()
}

pub(crate) fn minecraft_assets_path() -> String {
    static MINECRAFT_ASSETS_PATH: OnceLock<String> = OnceLock::new();
    MINECRAFT_ASSETS_PATH
        .get_or_init(|| format!("{}/assets", minecraft_path()))
        .clone()
}

pub(crate) fn minecraft_libraries_path() -> String {
    static MINECRAFT_LIBRARIES_PATH: OnceLock<String> = OnceLock::new();
    MINECRAFT_LIBRARIES_PATH
        .get_or_init(|| format!("{}/libraries", minecraft_path()))
        .clone()
}

pub(crate) fn minecraft_versions_path() -> String {
    static MINECRAFT_VERSIONS_PATH: OnceLock<String> = OnceLock::new();
    MINECRAFT_VERSIONS_PATH
        .get_or_init(|| format!("{}/versions", minecraft_path()))
        .clone()
}
