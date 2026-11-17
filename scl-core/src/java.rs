//! Java 的搜索，版本检测
use std::path::{Path, PathBuf};

use crate::{
    prelude::*,
    utils::{locate_path, Arch},
};

/// 一个 Java 运行时类型
#[derive(Debug, Clone)]
pub struct JavaRuntime {
    java_path: String,
    java_version: String,
    java_main_version: u8,
    java_64bit: bool,
    java_arch: Arch,
}

impl JavaRuntime {
    /// 通过一个指向 Java 可执行文件的路径来创建 [`JavaRuntime`]
    ///
    /// 在此会尝试运行这个文件并获取相关的版本信息，确认无误后返回
    pub fn from_java_path(java_path: impl AsRef<std::ffi::OsStr>) -> DynResult<Self> {
        tracing::debug!(
            "正在重新定位 Java 执行文件: {}",
            java_path.as_ref().to_string_lossy()
        );
        let java_path = locate_path(java_path.as_ref());
        tracing::debug!("Java 执行文件已重新定位: {}", java_path.display());
        let output = query_java_version_output(&java_path)?;
        let version = query_java_version(&output);
        let java_main_version = get_java_version(version);
        let java_64bit = query_java_is_64bit(&output);
        let java_arch = crate::utils::get_exec_arch(&java_path)?;
        Ok(Self {
            java_path: java_path.to_string_lossy().to_string(),
            java_64bit,
            java_version: version.to_owned(),
            java_main_version,
            java_arch,
        })
    }

    /// 获取此 Java 运行时的可执行文件路径
    #[inline]
    pub fn path(&self) -> &str {
        &self.java_path
    }

    /// 获取此 Java 运行时的版本号
    #[inline]
    pub fn version(&self) -> &str {
        &self.java_version
    }

    /// 获取此 Java 运行时的运行架构是否是针对 64 位平台的
    #[inline]
    pub fn is_64bit(&self) -> bool {
        self.java_64bit
    }

    /// 获取此 Java 运行时的主 Java 版本号
    #[inline]
    pub fn main_version(&self) -> u8 {
        self.java_main_version
    }

    /// 获取此 Java 运行时的运行架构
    #[inline]
    pub fn arch(&self) -> Arch {
        self.java_arch
    }

    /// 仅用于单元测试：直接指定路径与主版本构造运行时，不执行任何 Java 进程
    #[cfg(test)]
    pub(crate) fn for_test(java_path: &str, java_main_version: u8) -> Self {
        Self {
            java_path: java_path.to_owned(),
            java_version: format!("{java_main_version}.0.0"),
            java_main_version,
            java_64bit: true,
            java_arch: Arch::X64,
        }
    }
}

/// 执行 Java 并使用 -version 获取其版本输出
fn query_java_version_output(java_path: impl AsRef<std::ffi::OsStr>) -> DynResult<String> {
    let c = {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            std::process::Command::new(java_path)
                .arg("-version")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .creation_flags(0x00000200 | 0x08000000)
                .spawn()?
        }
        #[cfg(not(windows))]
        {
            std::process::Command::new(java_path)
                .arg("-version")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?
        }
    };
    let output = c.wait_with_output()?;
    Ok(String::from_utf8(output.stderr)?)
}

/// 根据 Java 的版本输出确认是否为 64 位版本
fn query_java_is_64bit(java_output: &str) -> bool {
    java_output.contains("64-Bit")
}

/// 根据 Java 裁剪的版本号文本确认主版本号
/// - `1.8.x` 将返回 `8`
/// - `1.7.x` 将返回 `7`
/// - `10+` 将返回其主版本数字
/// - 其余返回 `0` 表示未知版本
fn get_java_version(java_version_string: &str) -> u8 {
    fn parser(input: &str) -> nom::IResult<&str, &str> {
        nom::character::complete::digit1(input)
    }
    if let Ok((_, r)) = parser(java_version_string) {
        let r = r.parse().unwrap(); // 应当不会出错的
        if r > 1 {
            r
        } else if java_version_string.contains("1.8") {
            8
        } else if java_version_string.contains("1.7") {
            7
        } else {
            0 // 未知版本
        }
    } else if java_version_string.contains("1.8") {
        8
    } else if java_version_string.contains("1.7") {
        7
    } else {
        0 // 未知版本
    }
}

/// 搜索可能存在 Java 的地方找到可用的 Java 运行时
/// 返回的结果列表项均为 java.exe/javaw.exe 或 java 执行文件的目录
pub fn search_for_java() -> Vec<String> {
    // 从安装目录中搜索 Java
    fn check_bin_java_directory(path: impl AsRef<Path>, result: &mut Vec<String>) {
        if let Ok(d) = fs_err::read_dir(path.as_ref()) {
            for d in d.flatten() {
                let mut path = d.path();
                #[cfg(target_os = "macos")]
                path.push("Contents/Home");
                path.push("bin");
                #[cfg(target_os = "windows")]
                {
                    path.push("java.exe");
                    if path.is_file() {
                        tracing::debug!("Added {}", path.display());
                        result.push(d.path().to_string_lossy().to_string());
                    }
                    {
                        path.pop();
                        path.push("javaw.exe");
                        if path.is_file() {
                            tracing::debug!("Added {}", path.display());
                            result.push(path.to_string_lossy().to_string());
                        }
                    }
                }
                #[cfg(not(windows))]
                {
                    path.push("java");
                    if path.is_file() {
                        result.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    fn check_java(path: impl Into<PathBuf>, result: &mut Vec<String>) {
        let mut path: PathBuf = path.into();
        #[cfg(windows)]
        {
            path.push("java.exe");
            if path.is_file() {
                result.push(path.to_string_lossy().to_string());
            }
            path.pop();
            path.push("javaw.exe");
            if path.is_file() {
                result.push(path.to_string_lossy().to_string());
            }
        }
        #[cfg(not(windows))]
        {
            path.push("java");
            if path.is_file() {
                result.push(path.to_string_lossy().to_string());
            }
        }
    }

    /// 版本管理器 / 构建工具 / IDE 常见 JDK 目录。
    ///
    /// `check_bin_java_directory` 会把每个子目录当作 JDK 根目录继续拼接
    /// `bin/java`，因此这里传入的都是包含多个 JDK 的父目录：
    /// - sdkman：`~/.sdkman/candidates/java`（含各版本目录与 `current` 符号链接）
    /// - Gradle 工具链：`~/.gradle/jdks`
    /// - JetBrains IDE：`~/.jdks`
    fn check_home_java_directories(result: &mut Vec<String>) {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let sdkman = std::env::var_os("SDKMAN_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".sdkman"));
        check_bin_java_directory(sdkman.join("candidates").join("java"), result);
        check_bin_java_directory(home.join(".gradle").join("jdks"), result);
        check_bin_java_directory(home.join(".jdks"), result);
        // JAVA_HOME 直接指向单个 JDK，可执行文件在 bin 目录下。
        if let Some(java_home) = std::env::var_os("JAVA_HOME") {
            check_java(PathBuf::from(java_home).join("bin"), result);
        }
    }
    #[cfg(windows)]
    {
        let mut result = Vec::with_capacity(16);
        use winreg::{enums::*, RegKey, HKEY};

        /// 读取 JavaSoft / Eclipse Adoptium 风格键下所有版本的 `JavaHome`。
        fn collect_java_homes(key: &RegKey, flags: u32, result: &mut Vec<String>) {
            for version in key.enum_keys().flatten() {
                let Ok(version_key) = key.open_subkey_with_flags(&version, KEY_READ | flags) else {
                    continue;
                };
                let Ok(home) = version_key.get_value::<String, _>("JavaHome") else {
                    continue;
                };
                // 个别安装器会把 JavaHome 写成带引号或尾随分隔符的字符串。
                let home = home.trim_matches(['"', '\0']).trim_end_matches(['\\', '/']);
                if !home.is_empty() {
                    result.push(home.to_owned());
                }
            }
        }

        /// 同时搜索 64 位与 32 位注册表视图。
        ///
        /// 64 位启动器进程默认只能看到 64 位视图，而老版本 Minecraft 经常
        /// 依赖 32 位 Java；不显式打开 KEY_WOW64_32KEY 会漏掉这些运行时。
        fn search_java_registry(root: HKEY, flags: u32, result: &mut Vec<String>) {
            let root = RegKey::predef(root);
            for base in [
                "SOFTWARE\\JavaSoft",
                "SOFTWARE\\Eclipse Adoptium",
                "SOFTWARE\\Eclipse Foundation",
            ] {
                let Ok(base_key) = root.open_subkey_with_flags(base, KEY_READ | flags) else {
                    continue;
                };
                for product in [
                    "Java Runtime Environment",
                    "Java Development Kit",
                    "JRE",
                    "JDK",
                ] {
                    let Ok(product_key) =
                        base_key.open_subkey_with_flags(product, KEY_READ | flags)
                    else {
                        continue;
                    };
                    collect_java_homes(&product_key, flags, result);
                }
            }
        }

        for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            for flags in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
                search_java_registry(root, flags, &mut result);
            }
        }

        fn list_java_directory(root: &Path, result: &mut Vec<String>) {
            for vendor in [
                "Java",
                "BellSoft",
                "AdoptOpenJDK",
                "Zulu",
                "Microsoft",
                "Eclipse Foundation",
                "Semeru",
            ] {
                check_bin_java_directory(root.join(vendor), result);
            }
        }

        // 常见 JDK 安装目录，同时扫描官方启动器随游戏下载的运行时。
        for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramFiles(ARM)"] {
            let Some(program_files) = std::env::var_os(var) else {
                continue;
            };
            let program_files = PathBuf::from(program_files);
            list_java_directory(&program_files, &mut result);
            let runtime_root = program_files.join("Minecraft Launcher").join("runtime");
            for dir in minecraft_launcher_runtime_dirs(&runtime_root) {
                check_java(&dir, &mut result);
            }
        }

        // Microsoft Store 版 Minecraft Launcher 的运行时位于包私有目录。
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let runtime_root = PathBuf::from(local_app_data)
                .join("Packages")
                .join("Microsoft.4297127D64EC6_8wekyb3d8bbwe")
                .join("LocalCache")
                .join("Local")
                .join("runtime");
            for dir in minecraft_launcher_runtime_dirs(&runtime_root) {
                check_java(&dir, &mut result);
            }
        }

        // 从环境变量中查询 java.exe。Windows 的 PATH 可能包含带引号的
        // 路径，交给 split_paths 处理，不能直接按 ';' 切分。
        if let Some(paths) = std::env::var_os("PATH") {
            for path in std::env::split_paths(&paths) {
                check_java(path, &mut result);
            }
        }
        // 从 JABBA 中查询 java.exe
        if let Ok(path) = std::env::var("JABBA_HOME") {
            let path = PathBuf::from(path).join("jdk");
            check_bin_java_directory(path.as_path(), &mut result);
        }
        check_home_java_directories(&mut result);
        let mut result: Vec<_> = result
            .into_iter()
            .map(|mut a| {
                if !a.ends_with("java.exe")
                    && !a.ends_with("javaw.exe")
                    && !a.ends_with("java")
                    && !a.ends_with("javaw")
                {
                    if !a.ends_with('\\') && !a.ends_with('/') {
                        a.push('\\');
                    }
                    a.push_str("bin\\java.exe");
                }
                a
            })
            .collect();
        // Windows 路径不区分大小写，按小写键排序并去重。
        result.sort_by_key(|path| path.to_ascii_lowercase());
        result.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        result
    }
    #[cfg(target_os = "linux")]
    {
        let mut result = Vec::with_capacity(16);

        // 一些常见目录（每个子目录是一个 JDK 根目录）
        check_bin_java_directory("/usr/java", &mut result);
        check_bin_java_directory("/usr/lib/jvm", &mut result);
        check_bin_java_directory("/usr/lib32/jvm", &mut result);

        // 从环境变量中查询 java；POSIX 的 PATH 用 ':' 分隔，
        // 必须交给 split_paths 处理，不能按 Windows 的 ';' 切分。
        if let Ok(paths) = std::env::var("PATH") {
            for path in std::env::split_paths(&paths) {
                check_java(path, &mut result);
            }
        }
        // 从 JABBA 中查询 java.exe
        if let Ok(path) = std::env::var("JABBA_HOME") {
            let path = PathBuf::from(path).join("jdk");
            check_bin_java_directory(path.as_path(), &mut result);
        }
        // sdkman / Gradle / JetBrains 等常见的用户级 JDK 目录
        check_home_java_directories(&mut result);
        // 主目录里的 .minecraft/runtime 文件夹
        if let Some(mut home_path) = dirs::home_dir() {
            home_path.push(".minecraft/runtime/jre-legacy/linux/jre-legacy/bin");
            check_java(&home_path, &mut result);
            home_path.push("../../../../java-runtime-alpha/linux/java-runtime-alpha/bin");
            check_java(&home_path, &mut result);
        }

        result
    }
    #[cfg(target_os = "macos")]
    {
        let mut result = Vec::with_capacity(16);

        check_bin_java_directory("/Library/Java/JavaVirtualMachines", &mut result);
        check_bin_java_directory("/System/Library/Java/JavaVirtualMachines", &mut result);
        check_java(
            "/Library/Internet Plug-Ins/JavaAppletPlugin.plugin/Contents/Home/bin/java",
            &mut result,
        );
        check_java("/Applications/Xcode.app/Contents/Applications/Application Loader.app/Contents/MacOS/itms/java/bin/java", &mut result);
        check_home_java_directories(&mut result);

        result
    }
}

/// 官方启动器内置 Java 运行时的目录布局。
///
/// 布局为 `runtime/<组件>/<windows-arch>/<组件>/bin`，`runtime_root` 是
/// `runtime` 目录本身。旧实现连续 `pop` 拼路径时会把 `java-runtime-alpha`
/// 拼到 `runtime/jre-legacy` 下面，导致随官方启动器下载的现代 Java 永远
/// 扫描不到。
#[cfg_attr(not(windows), allow(dead_code))]
fn minecraft_launcher_runtime_dirs(runtime_root: &Path) -> Vec<PathBuf> {
    const COMPONENTS: &[&str] = &[
        "jre-legacy",
        "java-runtime-alpha",
        "java-runtime-beta",
        "java-runtime-gamma",
        "java-runtime-delta",
        "java-runtime-epsilon",
    ];
    const ARCHES: &[&str] = &["windows-x64", "windows-x86", "windows-arm64"];

    let mut dirs = Vec::with_capacity(COMPONENTS.len() * ARCHES.len());
    for component in COMPONENTS {
        for arch in ARCHES {
            dirs.push(
                runtime_root
                    .join(component)
                    .join(arch)
                    .join(component)
                    .join("bin"),
            );
        }
    }
    dirs
}

/// 根据 Java 的输出裁剪出版本号文本
fn query_java_version(java_output: &str) -> &str {
    fn parser(input: &str) -> nom::IResult<&str, &str> {
        nom::bytes::complete::take_until("\"")(input)
    }
    let pat = "version \"";
    if let Some(p) = java_output.find(pat) {
        if let Ok((_, r)) = parser(&java_output[p + pat.len()..]) {
            r
        } else {
            ""
        }
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::minecraft_launcher_runtime_dirs;

    #[test]
    fn launcher_runtime_layout_keeps_component_at_each_level() {
        let dirs = minecraft_launcher_runtime_dirs(Path::new(r"C:\Minecraft Launcher\runtime"));

        assert!(dirs
            .iter()
            .any(|p| p.ends_with("jre-legacy/windows-x64/jre-legacy/bin")));
        assert!(dirs
            .iter()
            .any(|p| p.ends_with("java-runtime-gamma/windows-x64/java-runtime-gamma/bin")));
        assert!(dirs
            .iter()
            .any(|p| p.ends_with("java-runtime-epsilon/windows-arm64/java-runtime-epsilon/bin")));

        // 旧实现会把现代运行时拼到 runtime/jre-legacy 下面。
        assert!(!dirs.iter().any(|p| {
            let text = p.to_string_lossy();
            text.contains("jre-legacy") && text.contains("java-runtime-alpha")
        }));
    }
}
