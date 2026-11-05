//! 修复 `.minecraft` 游戏文件夹：补全损坏或缺失的版本文件。

use std::path::Path;

use anyhow::Context;
use futures::TryStreamExt;
use scl_core::{
    download::{authlib::AuthlibDownloadExt, vanilla::VanillaDownloadExt, Downloader},
    progress::Reporter,
    version::structs::{VersionInfo, VersionMeta},
};

use crate::{core::DownloadSource, utils::prelude};

/// 检查并修复指定游戏文件夹下的所有版本，返回修复的版本数量
pub async fn fix_minecraft<P: AsRef<Path>, R: Reporter>(
    minecraft_folder: P,
    source: DownloadSource,
    parallel_amount: usize,
    reporter: R,
) -> anyhow::Result<usize> {
    let folder = minecraft_folder.as_ref();
    let versions_dir = folder.join("versions");
    let mut entries = match smol::fs::read_dir(&versions_dir).await {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(err) => {
            return Err(err).context(format!("读取版本目录 {} 失败", versions_dir.display()))
        }
    };

    let downloader = Downloader::<R>::default()
        .with_source(source.to_core())
        .with_minecraft_path(folder)
        .with_parallel_amount(parallel_amount)
        .with_verify_data()
        .with_reporter(reporter);

    if let Err(err) = downloader.install_authlib_injector().await {
        tracing::warn!("安装 Authlib-Injector 失败：{err:?}");
    }

    let mut fixed = 0;
    while let Some(entry) = entries.try_next().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        fixed += fix_version_dir(&downloader, &path, name).await?;
    }
    Ok(fixed)
}

/// 修复单个版本目录（版本级修复）。
///
/// 官方「补全/修复版本文件」按钮只对当前打开的那个版本调用
/// `fix_minecraft`，不会扫描整个 `.minecraft/versions`。
pub async fn fix_minecraft_version<P: AsRef<Path>, R: Reporter>(
    minecraft_folder: P,
    version: &VersionInfo,
    source: DownloadSource,
    parallel_amount: usize,
    reporter: R,
) -> anyhow::Result<usize> {
    let folder = minecraft_folder.as_ref();
    let version_path = Path::new(&version.version_base).join(&version.version);
    if !version_path.is_dir() {
        anyhow::bail!("版本目录不存在：{}", version_path.display());
    }
    tracing::info!("开始版本级修复：{}", version_path.display());

    let downloader = Downloader::<R>::default()
        .with_source(source.to_core())
        .with_minecraft_path(folder)
        .with_parallel_amount(parallel_amount)
        .with_verify_data()
        .with_reporter(reporter);

    if let Err(err) = downloader.install_authlib_injector().await {
        tracing::warn!("安装 Authlib-Injector 失败：{err:?}");
    }

    fix_version_dir(&downloader, &version_path, &version.version).await
}

/// 执行单个版本目录的 JSON 修复与文件校验/补全，返回修复计数。
async fn fix_version_dir<R: Reporter>(
    downloader: &Downloader<R>,
    path: &Path,
    name: &str,
) -> anyhow::Result<usize> {
    let mut fixed = 0;
    let json_path = path.join(format!("{name}.json"));
    if !version_json_healthy(&json_path) {
        let jar_path = path.join(format!("{name}.jar"));
        if !jar_path.is_file() {
            return Ok(0);
        }
        match extract_version_json_from_jar(&jar_path) {
            Ok(Some(data)) => {
                prelude::write_file_safe_async(&json_path, data)
                    .await
                    .with_context(|| format!("写入版本文件 {} 失败", json_path.display()))?;
                tracing::info!("已从客户端 JAR 修复版本 {name}");
                fixed += 1;
            }
            Ok(None) => {
                tracing::warn!("版本 {name} 的客户端 JAR 中没有内嵌 version.json，无法自动修复");
                return Ok(fixed);
            }
            Err(err) => {
                tracing::warn!("解析版本 {name} 的客户端 JAR 失败：{err:?}");
                return Ok(fixed);
            }
        }
    }

    let data = match std::fs::read(&json_path) {
        Ok(data) => data,
        Err(err) => {
            tracing::warn!("读取版本 {name} 的 JSON 失败：{err:?}");
            return Ok(fixed);
        }
    };
    let mut meta = match serde_json::from_slice::<VersionMeta>(&data) {
        Ok(meta) => meta,
        Err(err) => {
            tracing::warn!("解析版本 {name} 的 JSON 失败：{err:?}");
            return Ok(fixed);
        }
    };
    // 带 inheritsFrom/clientVersion 的模组版本自身没有 downloads/assetIndex，
    // 必须先把父版本元数据合并进来再校验补全；只处理父版本会把加载器自身
    // 的依赖库整批漏掉（表现为「补全/修复版本文件」对模组版本毫无动作）。
    let parent_name = if !meta.inherits_from.is_empty() {
        meta.inherits_from.to_owned()
    } else if !meta.client_version.is_empty() && meta.client_version != name {
        meta.client_version.to_owned()
    } else {
        String::new()
    };
    if !parent_name.is_empty() {
        let parent_json = path
            .parent()
            .map(|versions| versions.join(&parent_name).join(format!("{parent_name}.json")))
            .unwrap_or_else(|| Path::new(".").join(format!("{parent_name}.json")));
        let parent_meta = std::fs::read(&parent_json)
            .ok()
            .and_then(|data| serde_json::from_slice::<VersionMeta>(&data).ok());
        match parent_meta {
            Some(parent) => {
                let mut merged = parent;
                merged += meta;
                meta = merged;
            }
            None => {
                tracing::warn!(
                    "版本 {name} 的父版本 {parent_name} 元数据不可用，跳过文件补全"
                );
                return Ok(fixed);
            }
        }
    }
    if meta.downloads.is_none() || meta.asset_index.is_none() {
        return Ok(fixed);
    }
    // 官方 `download_vanilla(is_repair = true)` 分支只校验补全依赖库/资源，
    // 不处理客户端 JAR。但「补全/修复版本文件」在 JAR 缺失时应能恢复，
    // 否则该版本必然启动失败（官方此行为属有害缺陷 DEV，不照抄）。这里只在
    // JAR 不存在时补下；已存在时保持官方语义，不重复做整文件 SHA1 校验。
    let jar_path = path.join(format!("{name}.jar"));
    if !jar_path.is_file() {
        if let Some(main_jar) = meta.downloads.as_ref().and_then(|d| d.get("client")) {
            if let Err(err) = downloader
                .download_vanilla_jar(
                    &main_jar.url,
                    &jar_path.to_string_lossy(),
                    &main_jar.sha1,
                )
                .await
            {
                tracing::warn!("补全版本 {name} 的客户端 JAR 失败：{err:?}");
            } else {
                tracing::info!("已补全版本 {name} 的客户端 JAR");
                fixed += 1;
            }
        }
    }
    // 修复模式：只校验并补全依赖库/资源，JAR 已在上方按缺失情况补齐。
    match downloader.download_vanilla(name, &meta, true).await {
        Ok(()) => {
            tracing::info!("已校验/补全版本 {name} 的游戏文件");
            fixed += 1;
        }
        Err(err) => {
            tracing::warn!("修复版本 {name} 的游戏文件失败：{err:?}");
        }
    }
    Ok(fixed)
}

/// 版本 JSON 是否存在且可以正常解析
fn version_json_healthy(json_path: &Path) -> bool {
    match std::fs::read(json_path) {
        Ok(data) => serde_json::from_slice::<serde_json::Value>(&data).is_ok(),
        Err(_) => false,
    }
}

/// 从客户端 JAR（zip 格式）中读取内嵌的 version.json
fn extract_version_json_from_jar(jar_path: &Path) -> anyhow::Result<Option<Vec<u8>>> {
    let file = std::fs::File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entry = match archive.by_name("version.json") {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut buffer)?;
    Ok(Some(buffer))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::core::DownloadSource;

    fn write_broken_version(root: &Path, name: &str) {
        let dir = root.join("versions").join(name);
        std::fs::create_dir_all(&dir).expect("创建版本目录");
        std::fs::write(dir.join(format!("{name}.json")), "{broken").expect("写入损坏 JSON");
        let jar = std::fs::File::create(dir.join(format!("{name}.jar"))).expect("创建 JAR");
        let mut zip = zip::ZipWriter::new(jar);
        zip.start_file("version.json", zip::write::FileOptions::default())
            .expect("写入 JAR 条目");
        zip.write_all(br#"{"mainClass":"net.minecraft.client.main.Main"}"#)
            .expect("写入内嵌 version.json");
        zip.finish().expect("完成 JAR");
    }

    #[test]
    fn version_level_fix_only_repairs_target_version() {
        let dir = std::env::temp_dir().join(format!("scl_fix_version_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mc = dir.join(".minecraft");
        write_broken_version(&mc, "Target");
        write_broken_version(&mc, "Other");
        // 避免 install_authlib_injector 触发网络下载
        std::fs::write(mc.join("authlib-injector.jar"), []).expect("创建占位注入器");

        let info = VersionInfo {
            version_base: mc.join("versions").to_string_lossy().to_string(),
            version: "Target".into(),
            ..Default::default()
        };
        let fixed = smol::block_on(fix_minecraft_version(
            &mc,
            &info,
            DownloadSource::Default,
            1,
            (),
        ))
        .expect("版本级修复应成功");
        assert_eq!(fixed, 1, "应修复目标版本 JSON");

        let target =
            std::fs::read_to_string(mc.join("versions/Target/Target.json")).expect("读取目标 JSON");
        assert!(
            serde_json::from_str::<serde_json::Value>(&target).is_ok(),
            "{target}"
        );
        let other =
            std::fs::read_to_string(mc.join("versions/Other/Other.json")).expect("读取其他 JSON");
        assert_eq!(other, "{broken", "版本级修复不应改动其它版本");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
