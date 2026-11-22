//! 原生库目录布局、启动前检查与解压。
//!
//! SCL 默认把某个平台的原生库解压到
//! `versions/<版本>/natives-<os>-<arch>/`，例如
//! `natives-linux-x86_64`。旧版本 SCL 使用
//! `versions/<版本>/natives/natives-<platform>/`，这里会在新目录不存在时
//! 复用旧目录，避免升级后重复解压。

use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::Context;
use tracing::{info, warn};

use crate::{
    prelude::*,
    utils::Arch,
    version::structs::{Allowed, DownloadItem, Library, VersionMeta},
};

/// 26.3+ 原生库的 JVM 搜索子目录名（对应 `java.library.path=${natives_directory}/java`）
pub(crate) const NATIVE_SUBDIR: &str = "java";

/// 原生库压缩包中需要解压的文件扩展名
const NATIVE_EXTS: &[&str] = &["dll", "so", "dylib", "jnilib"];

/// 启动前原生库检查结果
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeCheckReport {
    /// 检查的原生库压缩包数量
    pub archives: usize,
    /// 实际执行了解压的压缩包数量
    pub extracted: usize,
    /// 缺失的原生库压缩包路径
    pub missing: Vec<PathBuf>,
}

/// 当前平台的原生库目录名，例如 `natives-linux-x86_64`
pub(crate) fn current_platform_dir_name(arch: Arch) -> String {
    format!("natives-{}-{}", os_name(), arch_name(arch))
}

/// 当前平台在 Mojang 元数据里使用的系统名
pub(crate) fn os_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
}

/// SCL 原生库目录使用的架构名
pub(crate) fn arch_name(arch: Arch) -> &'static str {
    match arch {
        Arch::X86 => "x86",
        Arch::X64 => "x86_64",
        Arch::ARM64 => "arm64",
    }
}

fn arch_bits(arch: Arch) -> &'static str {
    match arch {
        Arch::X86 => "32",
        Arch::X64 | Arch::ARM64 => "64",
    }
}

/// 计算默认的原生库目录：`versions/<版本>/natives-<os>-<arch>`
pub(crate) fn default_native_dir(version_dir: &Path, arch: Arch) -> PathBuf {
    version_dir.join(current_platform_dir_name(arch))
}

#[cfg_attr(target_os = "linux", allow(unused_variables))]
fn legacy_platform_name(arch: Arch) -> &'static str {
    #[cfg(target_os = "windows")]
    {
        match arch {
            Arch::X86 => "natives-windows-x86",
            Arch::X64 => "natives-windows",
            Arch::ARM64 => "natives-windows-arm64",
        }
    }
    #[cfg(target_os = "linux")]
    {
        "natives-linux"
    }
    #[cfg(target_os = "macos")]
    {
        match arch {
            Arch::X86 | Arch::X64 => "natives-macos",
            Arch::ARM64 => "natives-macos-arm64",
        }
    }
}

fn legacy_native_dir(version_dir: &Path, arch: Arch) -> PathBuf {
    version_dir.join("natives").join(legacy_platform_name(arch))
}

/// 把元数据里的 native classifier 归一化为 `natives-<os>-<arch>`
///
/// 例如：
/// - `natives-linux` -> `natives-linux-x86_64`
/// - `natives-windows-x86` -> `natives-windows-x86`
/// - `natives-osx` -> `natives-macos-x86_64`
pub(crate) fn platform_dir_name(platform: &str) -> String {
    // Maven 风格的 name 可能带 `@jar` 后缀，目录名不应包含它。
    let platform = platform.split('@').next().unwrap_or(platform);
    let Some((os, rest)) = split_platform(platform) else {
        return platform.to_owned();
    };
    let arch = rest.map(arch_from_rest).unwrap_or(Arch::X64);
    format!("natives-{os}-{}", arch_name(arch))
}

fn split_platform(platform: &str) -> Option<(&str, Option<&str>)> {
    let raw = platform.strip_prefix("natives-").unwrap_or(platform);
    for os in ["windows", "linux", "macos"] {
        if raw == os {
            return Some((os, None));
        }
        if let Some(rest) = raw.strip_prefix(os).and_then(|rest| rest.strip_prefix('-')) {
            return Some((os, Some(rest)));
        }
    }
    if raw == "osx" {
        return Some(("macos", None));
    }
    if let Some(rest) = raw.strip_prefix("osx-") {
        return Some(("macos", Some(rest)));
    }
    None
}

fn arch_from_rest(rest: &str) -> Arch {
    let rest = rest.to_ascii_lowercase();
    if rest.contains("arm64") || rest.contains("aarch64") {
        Arch::ARM64
    } else if rest.contains("x86_64") || rest.contains("amd64") || rest == "64" {
        Arch::X64
    } else if rest.contains("x86") || rest == "32" {
        Arch::X86
    } else {
        // macOS 的 natives-macos-patch 是 x86_64 的补丁包；未知后缀也按
        // 最常见的 x86_64 处理，避免把目录名写成无法识别的形式。
        Arch::X64
    }
}

fn classifier_arch(platform: &str) -> Option<Arch> {
    split_platform(platform)
        .and_then(|(_, rest)| rest)
        .map(arch_from_rest)
}

fn is_platform_native_dir(name: &str, arch: Arch) -> bool {
    let Some((os, _)) = split_platform(name) else {
        return false;
    };
    if os != os_name() {
        return false;
    }
    classifier_arch(name).is_none_or(|dir_arch| dir_arch == arch)
}

fn is_blauncher_native_dir(name: &str) -> bool {
    name.strip_prefix("natives-").is_some_and(|hash| {
        hash.len() == 12 && hash.chars().all(|character| character.is_ascii_hexdigit())
    })
}

/// 在版本目录中寻找可复用的原生库目录
///
/// 优先级：
/// 1. 新默认目录 `natives-<os>-<arch>`；
/// 2. 旧 SCL 目录 `natives/natives-<platform>`；
/// 3. PCL 的 `<版本>-natives`；
/// 4. 版本目录下其他匹配当前平台的 `natives-*` 目录（兼容旧 HMCL 等）；
/// 5. Blauncher 的 `natives-<hash>`。
///
/// 如果都没有，返回新默认目录，由调用方创建并解压。
pub fn resolve_native_dir(
    version_dir: &Path,
    version: &str,
    meta: &VersionMeta,
    arch: Arch,
) -> PathBuf {
    let subdir = meta.natives_use_subdirectories().then_some(NATIVE_SUBDIR);
    let default = default_native_dir(version_dir, arch);
    if has_native_files(&default, subdir) {
        return default;
    }

    let legacy = legacy_native_dir(version_dir, arch);
    if has_native_files(&legacy, subdir) {
        return legacy;
    }

    let pcl = version_dir.join(format!("{version}-natives"));
    if has_native_files(&pcl, subdir) {
        return pcl;
    }

    let mut blauncher = None;
    if let Ok(entries) = fs_err::read_dir(version_dir) {
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let default_name = default
                .file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_default();
            if name == default_name.as_ref() {
                continue;
            }
            if is_platform_native_dir(&name, arch) && has_native_files(&entry.path(), subdir) {
                return entry.path();
            }
            if blauncher.is_none()
                && is_blauncher_native_dir(&name)
                && has_native_files(&entry.path(), subdir)
            {
                blauncher = Some(entry.path());
            }
        }
    }
    blauncher.unwrap_or(default)
}

fn has_native_files(dir: &Path, subdir: Option<&str>) -> bool {
    has_native_files_recursive(&native_extract_dir(dir, subdir), 0)
}

fn has_native_files_recursive(dir: &Path, depth: usize) -> bool {
    if depth > 3 {
        return false;
    }
    let Ok(entries) = fs_err::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file() {
            if entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| NATIVE_EXTS.contains(&extension))
            {
                return true;
            }
        } else if file_type.is_dir() && has_native_files_recursive(&entry.path(), depth + 1) {
            return true;
        }
    }
    false
}

/// 计算原生库解压目录
///
/// 26.3+ 需要多一层 `java` 子目录，与 JVM 参数里的
/// `${natives_directory}/java` 对齐；老版本直接使用目录本身。
pub(crate) fn native_extract_dir(base: &Path, subdir: Option<&str>) -> PathBuf {
    match subdir {
        Some(subdir) => base.join(subdir),
        None => base.to_path_buf(),
    }
}

/// 判断 native 压缩包中是否还有未落盘或大小不正确的文件
///
/// 打开/解析失败时返回 `true`，交给 [`unzip_natives`] 给出具体错误。
pub(crate) fn natives_need_extract(unzip_file: &Path, unzip_dir: &Path) -> bool {
    let Ok(file) = fs_err::File::open(unzip_file) else {
        return true;
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return true;
    };
    for index in 0..archive.len() {
        let Ok(file) = archive.by_index(index) else {
            return true;
        };
        let Some(path) = file
            .enclosed_name()
            .and_then(|path| path.file_name().map(|name| name.to_owned()))
        else {
            continue;
        };
        let Some(extension) = Path::new(&path).extension() else {
            continue;
        };
        if !NATIVE_EXTS.contains(&extension.to_str().unwrap_or_default()) {
            continue;
        }
        let target = unzip_dir.join(&path);
        if !target.is_file() {
            return true;
        }
        if let Ok(metadata) = fs_err::metadata(&target) {
            if metadata.len() != file.size() {
                return true;
            }
        }
    }
    false
}

/// 解压指定 ZIP 压缩文件中的原生库到指定文件夹
///
/// 只解压 `dll`、`so`、`dylib`、`jnilib`，并沿用 SCL 原有的扁平化布局：
/// 压缩包内的目录层级会被丢弃，只保留文件名。
pub(crate) fn unzip_natives(unzip_file: &Path, unzip_dir: &Path) -> DynResult {
    let file = fs_err::File::open(unzip_file)?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("解压原生库 {} 时发生错误", unzip_file.display()))?;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let Some(path) = file
            .enclosed_name()
            .and_then(|path| path.file_name().map(|name| name.to_owned()))
        else {
            continue;
        };
        let Some(extension) = Path::new(&path).extension() else {
            continue;
        };
        if !NATIVE_EXTS.contains(&extension.to_str().unwrap_or_default()) {
            continue;
        }
        let save_path = unzip_dir.join(&path);
        // 已经存在且大小一致的文件不再重写，避免在 Windows 上碰到被其它
        // 进程占用的 dll，也减少不必要的磁盘写入。
        if let Ok(metadata) = fs_err::metadata(&save_path) {
            if metadata.is_file() && metadata.len() == file.size() {
                continue;
            }
        }
        let save_dir = save_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("原生库目标路径没有父目录：{}", save_path.display()))?;
        fs_err::create_dir_all(save_dir)?;
        let mut output = fs_err::File::create(&save_path)?;
        std::io::copy(&mut file, &mut output)?;
    }
    Ok(())
}

/// 启动前检查原生库，并只解压缺失/大小不正确的文件
///
/// 该函数只使用 `libraries/` 中已经存在的 native jar，不会下载资源、游戏
/// JAR 或普通依赖库。缺失的压缩包会记录在 [`NativeCheckReport::missing`]
/// 中，由调用方决定是否做一次只针对原生库的补全。
pub fn ensure_native_libraries(
    meta: &VersionMeta,
    version_dir: &Path,
    libraries_dir: &Path,
    arch: Arch,
) -> DynResult<NativeCheckReport> {
    let version = version_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let natives_dir = resolve_native_dir(version_dir, version, meta, arch);
    let subdir = meta.natives_use_subdirectories().then_some(NATIVE_SUBDIR);
    let target = native_extract_dir(&natives_dir, subdir);
    let candidates = selected_candidates(meta, libraries_dir, arch);
    let mut report = NativeCheckReport {
        archives: candidates.len(),
        ..Default::default()
    };

    for candidate in candidates {
        if !candidate.archive.is_file() {
            report.missing.push(candidate.archive);
            continue;
        }
        if natives_need_extract(&candidate.archive, &target) {
            fs_err::create_dir_all(&target)?;
            unzip_natives(&candidate.archive, &target)?;
            report.extracted += 1;
        }
    }

    if !report.missing.is_empty() {
        warn!(
            "原生库压缩包缺失 {} 个，首次启动前无法解压：{}",
            report.missing.len(),
            report
                .missing
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if report.extracted > 0 {
        info!(
            "启动前检查原生库：从 {} 个压缩包补全了 {} 个到 {}",
            report.archives,
            report.extracted,
            target.display()
        );
    }
    Ok(report)
}

struct NativeCandidate {
    group: String,
    exact: bool,
    archive: PathBuf,
}

fn selected_candidates(
    meta: &VersionMeta,
    libraries_dir: &Path,
    arch: Arch,
) -> Vec<NativeCandidate> {
    let candidates = meta
        .libraries
        .iter()
        .filter(|library| library.rules.is_allowed())
        .filter_map(|library| native_candidate(library, libraries_dir, arch))
        .collect::<Vec<_>>();
    // 同一组依赖同时提供通用包和架构专用包时，只使用架构专用包，避免
    // x86_64 的 dll/so 覆盖 arm64 版本。
    let exact_groups = candidates
        .iter()
        .filter(|candidate| candidate.exact)
        .map(|candidate| candidate.group.clone())
        .collect::<HashSet<_>>();
    candidates
        .into_iter()
        .filter(|candidate| candidate.exact || !exact_groups.contains(&candidate.group))
        .collect()
}

fn native_candidate(
    library: &Library,
    libraries_dir: &Path,
    arch: Arch,
) -> Option<NativeCandidate> {
    let group = library
        .name
        .split(':')
        .take(2)
        .collect::<Vec<_>>()
        .join(":");

    if let Some(natives) = &library.natives {
        let classifier = select_legacy_classifier(natives, arch)?;
        let item = library
            .downloads
            .as_ref()?
            .classifiers
            .as_ref()?
            .get(&classifier)?;
        let archive = resolve_archive(library, item, Some(&classifier), libraries_dir)?;
        return Some(NativeCandidate {
            group,
            exact: classifier_arch(&classifier) == Some(arch),
            archive,
        });
    }

    let classifier = library.name.split(':').nth(3)?;
    let classifier = classifier.split('@').next().unwrap_or(classifier);
    if !classifier.starts_with("natives-") {
        return None;
    }
    let (os, _) = split_platform(classifier)?;
    if os != os_name() {
        return None;
    }
    let candidate_arch = classifier_arch(classifier);
    if candidate_arch.is_some_and(|candidate_arch| candidate_arch != arch) {
        return None;
    }
    let item = library.downloads.as_ref()?.artifact.as_ref()?;
    let archive = resolve_archive(library, item, Some(classifier), libraries_dir)?;
    Some(NativeCandidate {
        group,
        exact: candidate_arch == Some(arch),
        archive,
    })
}

fn select_legacy_classifier(natives: &BTreeMap<String, String>, arch: Arch) -> Option<String> {
    let os = os_name();
    let arch_name = arch_name(arch);
    let bits = arch_bits(arch);
    let mut os_keys = vec![os];
    if os == "macos" {
        os_keys.push("osx");
    }

    // 先尝试带架构/位数的 key，再尝试通用 key，避免 ARM64 误选 x86_64。
    let mut keys = Vec::with_capacity(os_keys.len() * 9);
    for os_key in os_keys {
        keys.push(format!("{os_key}-{arch_name}"));
        keys.push(format!("{os_key}-{bits}"));
        keys.push(os_key.to_owned());
        keys.push(format!("natives-{os_key}-{arch_name}"));
        keys.push(format!("natives-{os_key}-{bits}"));
        keys.push(format!("natives-{os_key}"));
        keys.push(format!("native-{os_key}-{arch_name}"));
        keys.push(format!("native-{os_key}-{bits}"));
        keys.push(format!("native-{os_key}"));
    }

    for key in keys {
        if let Some(value) = natives.get(&key) {
            return Some(value.replace("${arch}", bits));
        }
    }
    None
}

fn resolve_archive(
    library: &Library,
    item: &DownloadItem,
    classifier: Option<&str>,
    libraries_dir: &Path,
) -> Option<PathBuf> {
    let path = if item.path.is_empty() {
        artifact_path_from_name(&library.name, classifier)?
    } else {
        item.path.clone()
    };
    Some(libraries_dir.join(path))
}

fn artifact_path_from_name(name: &str, classifier: Option<&str>) -> Option<String> {
    let mut parts = name.split(':');
    let group = parts.next()?;
    let artifact = parts.next()?;
    let version = parts.next()?;
    let existing_classifier = parts.next();
    let classifier = classifier.or(existing_classifier);
    let file = match classifier {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    Some(format!(
        "{}/{}/{}/{}",
        group.replace('.', "/"),
        artifact,
        version,
        file
    ))
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    static TEST_ID: AtomicUsize = AtomicUsize::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "scl_native_{tag}_{}_{}_{}",
                std::process::id(),
                id,
                nanos
            ));
            let _ = fs_err::remove_dir_all(&path);
            fs_err::create_dir_all(&path).expect("创建测试目录");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs_err::remove_dir_all(&self.0);
        }
    }

    fn meta(use_subdirectories: bool) -> VersionMeta {
        let jvm = if use_subdirectories {
            r#""-Djava.library.path=${natives_directory}/java""#
        } else {
            r#""-Djava.library.path=${natives_directory}""#
        };
        serde_json::from_str(&format!(
            r#"{{
                "mainClass": "net.minecraft.client.main.Main",
                "arguments": {{ "game": [], "jvm": [{jvm}] }},
                "libraries": []
            }}"#
        ))
        .expect("解析测试元数据")
    }

    fn write_native_jar(path: &Path, entry: &str) {
        fs_err::create_dir_all(path.parent().unwrap()).expect("创建 jar 目录");
        let file = fs_err::File::create(path).expect("创建 jar");
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(entry, zip::write::SimpleFileOptions::default())
            .expect("写入 zip 项");
        zip.write_all(b"native").expect("写入原生库内容");
        zip.finish().expect("完成 zip");
    }

    fn dummy_native(dir: &Path, subdir: Option<&str>) {
        let dir = native_extract_dir(dir, subdir);
        fs_err::create_dir_all(&dir).expect("创建原生库目录");
        fs_err::write(dir.join("libtest.so"), b"native").expect("写入原生库");
    }

    #[test]
    fn platform_dir_names_match_hmcl_layout() {
        assert_eq!(platform_dir_name("natives-linux"), "natives-linux-x86_64");
        assert_eq!(
            platform_dir_name("natives-linux@jar"),
            "natives-linux-x86_64"
        );
        assert_eq!(
            platform_dir_name("natives-windows"),
            "natives-windows-x86_64"
        );
        assert_eq!(
            platform_dir_name("natives-windows-x86"),
            "natives-windows-x86"
        );
        assert_eq!(
            platform_dir_name("natives-windows-arm64"),
            "natives-windows-arm64"
        );
        assert_eq!(platform_dir_name("natives-osx"), "natives-macos-x86_64");
        assert_eq!(
            platform_dir_name("natives-macos-arm64"),
            "natives-macos-arm64"
        );
        assert_eq!(
            platform_dir_name("natives-macos-patch"),
            "natives-macos-x86_64"
        );
    }

    #[test]
    fn resolves_new_default_directory_first() {
        let temp = TempDir::new("default");
        let version_dir = temp.0.join("1.21.1");
        let default = default_native_dir(&version_dir, Arch::X64);
        dummy_native(&default, None);
        let legacy = legacy_native_dir(&version_dir, Arch::X64);
        dummy_native(&legacy, None);

        assert_eq!(
            resolve_native_dir(&version_dir, "1.21.1", &meta(false), Arch::X64),
            default
        );
    }

    #[test]
    fn falls_back_to_legacy_scl_directory() {
        let temp = TempDir::new("legacy");
        let version_dir = temp.0.join("1.21.1");
        let legacy = legacy_native_dir(&version_dir, Arch::X64);
        dummy_native(&legacy, None);

        assert_eq!(
            resolve_native_dir(&version_dir, "1.21.1", &meta(false), Arch::X64),
            legacy
        );
    }

    #[test]
    fn falls_back_to_pcl_and_hmcl_directories() {
        let temp = TempDir::new("fallback");
        let version_dir = temp.0.join("1.21.1");

        let pcl = version_dir.join("1.21.1-natives");
        dummy_native(&pcl, None);
        assert_eq!(
            resolve_native_dir(&version_dir, "1.21.1", &meta(false), Arch::X64),
            pcl
        );
        fs_err::remove_dir_all(&pcl).unwrap();

        let hmcl = version_dir.join("natives-linux");
        dummy_native(&hmcl, None);
        let expected = if os_name() == "linux" {
            hmcl
        } else {
            // 其它平台测试不依赖 Linux 目录名，直接跳过该分支。
            return;
        };
        assert_eq!(
            resolve_native_dir(&version_dir, "1.21.1", &meta(false), Arch::X64),
            expected
        );
    }

    #[test]
    fn falls_back_to_blauncher_native_directory() {
        let temp = TempDir::new("blauncher");
        let version_dir = temp.0.join("blauncher");
        let blauncher = version_dir.join("natives-da39a3ee5e6b");
        dummy_native(&blauncher, None);

        assert_eq!(
            resolve_native_dir(&version_dir, "blauncher", &meta(false), Arch::X64),
            blauncher
        );
    }

    #[test]
    fn resolves_subdirectory_layout() {
        let temp = TempDir::new("subdir");
        let version_dir = temp.0.join("26.3");
        let legacy = legacy_native_dir(&version_dir, Arch::X64);
        dummy_native(&legacy, Some(NATIVE_SUBDIR));

        assert_eq!(
            resolve_native_dir(&version_dir, "26.3", &meta(true), Arch::X64),
            legacy
        );
    }

    #[test]
    fn extracts_only_missing_native_files() {
        let temp = TempDir::new("extract");
        let version_dir = temp.0.join("1.21.1");
        let libraries_dir = temp.0.join("libraries");
        let classifier = format!("natives-{}", os_name());
        let jar_path = libraries_dir
            .join("org/example/native/1.0")
            .join(format!("native-1.0-{classifier}.jar"));
        write_native_jar(&jar_path, "libtest.so");

        let meta: VersionMeta = serde_json::from_str(&format!(
            r#"{{
                "mainClass": "net.minecraft.client.main.Main",
                "arguments": {{ "game": [], "jvm": ["-Djava.library.path=${{natives_directory}}"] }},
                "libraries": [{{
                    "name": "org.example:native:1.0:{classifier}",
                    "downloads": {{
                        "artifact": {{
                            "path": "org/example/native/1.0/native-1.0-{classifier}.jar",
                            "sha1": "",
                            "size": 0,
                            "url": ""
                        }}
                    }}
                }}]
            }}"#
        ))
        .expect("解析现代 native 元数据");

        let report =
            ensure_native_libraries(&meta, &version_dir, &libraries_dir, Arch::X64).unwrap();
        assert_eq!(report.archives, 1);
        assert_eq!(report.extracted, 1);
        assert!(report.missing.is_empty());
        let target = default_native_dir(&version_dir, Arch::X64);
        assert!(target.join("libtest.so").is_file());

        let report =
            ensure_native_libraries(&meta, &version_dir, &libraries_dir, Arch::X64).unwrap();
        assert_eq!(report.extracted, 0);
    }

    #[test]
    fn unzip_natives_keeps_matching_files() {
        let temp = TempDir::new("keep");
        let jar_path = temp.0.join("native.jar");
        write_native_jar(&jar_path, "libtest.so");
        let target = temp.0.join("natives");
        fs_err::create_dir_all(&target).expect("创建原生库目录");
        let existing = target.join("libtest.so");
        fs_err::write(&existing, b"keep!!").expect("写入已有原生库");

        unzip_natives(&jar_path, &target).expect("解压原生库");
        assert_eq!(
            fs_err::read(&existing).expect("读取已有原生库"),
            b"keep!!",
            "大小一致的原生库文件不应被重写"
        );
    }

    #[test]
    fn extracts_legacy_classifier_and_subdirectory() {
        let temp = TempDir::new("legacy_extract");
        let version_dir = temp.0.join("1.12.2");
        let libraries_dir = temp.0.join("libraries");
        let os = if os_name() == "macos" {
            "osx"
        } else {
            os_name()
        };
        let classifier = format!("natives-{os}");
        let jar_path = libraries_dir
            .join("org/example/native/1.0")
            .join(format!("native-1.0-{classifier}.jar"));
        write_native_jar(&jar_path, "libtest.so");

        let meta: VersionMeta = serde_json::from_str(&format!(
            r#"{{
                "mainClass": "net.minecraft.client.main.Main",
                "arguments": {{ "game": [], "jvm": ["-Djava.library.path=${{natives_directory}}/java"] }},
                "libraries": [{{
                    "name": "org.example:native:1.0",
                    "natives": {{ "{os}": "{classifier}" }},
                    "downloads": {{
                        "classifiers": {{
                            "{classifier}": {{
                                "path": "org/example/native/1.0/native-1.0-{classifier}.jar",
                                "sha1": "",
                                "size": 0,
                                "url": ""
                            }}
                        }}
                    }}
                }}]
            }}"#
        ))
        .expect("解析旧版 native 元数据");

        let report =
            ensure_native_libraries(&meta, &version_dir, &libraries_dir, Arch::X64).unwrap();
        assert_eq!(report.extracted, 1);
        let target = default_native_dir(&version_dir, Arch::X64).join(NATIVE_SUBDIR);
        assert!(target.join("libtest.so").is_file());
    }

    #[test]
    fn reports_missing_archives_without_panicking() {
        let temp = TempDir::new("missing");
        let version_dir = temp.0.join("1.21.1");
        let libraries_dir = temp.0.join("libraries");
        let classifier = format!("natives-{}", os_name());
        let meta: VersionMeta = serde_json::from_str(&format!(
            r#"{{
                "mainClass": "net.minecraft.client.main.Main",
                "arguments": {{ "game": [], "jvm": ["-Djava.library.path=${{natives_directory}}"] }},
                "libraries": [{{
                    "name": "org.example:native:1.0:{classifier}",
                    "downloads": {{
                        "artifact": {{
                            "path": "org/example/native/1.0/native-1.0-{classifier}.jar",
                            "sha1": "",
                            "size": 0,
                            "url": ""
                        }}
                    }}
                }}]
            }}"#
        ))
        .expect("解析元数据");

        let report =
            ensure_native_libraries(&meta, &version_dir, &libraries_dir, Arch::X64).unwrap();
        assert_eq!(report.archives, 1);
        assert_eq!(report.extracted, 0);
        assert_eq!(report.missing.len(), 1);
    }
}
