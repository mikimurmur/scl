//! 启动器常用的一些基础工具函数。

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;

/// 临时文件序号：同一个进程内每次安全写入都使用不同的临时文件名。
///
/// 之前所有写入共用 `<目标>.scl-tmp`，异步防抖保存与关窗同步保存并发时，
/// 先完成的一方 rename 后，另一方的 rename 源文件已不存在，会报
/// `No such file or directory (os error 2)`。
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

use crate::{
    config::VersionIndependentMethod,
    core::{LocalVersionInfo, VersionType},
};

/// 把 [`SystemTime`] 转换为 Unix 毫秒时间戳
fn to_millis(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// 扫描指定的版本文件夹，加载所有可启动的游戏版本信息
pub async fn load_game_versions<P: AsRef<Path> + Send>(
    path: P,
) -> anyhow::Result<Vec<LocalVersionInfo>> {
    let versions = scl_core::version::get_avaliable_versions(&path).await?;
    let path = path.as_ref().to_string_lossy().to_string();
    Ok(versions
        .into_iter()
        .map(|version| LocalVersionInfo {
            name: version.name,
            version_type: version.version_type.into(),
            path: path.to_owned(),
            created_date: to_millis(version.created_date),
            access_date: to_millis(version.access_date),
        })
        .collect())
}

/// 根据版本独立方案判断某个版本是否需要被独立隔离存放
pub fn should_version_independent(
    method: VersionIndependentMethod,
    minecraft_version: &scl_core::semver::MinecraftVersion,
    version_type: VersionType,
) -> bool {
    method.should_apply(minecraft_version, version_type)
}

/// 安全地复制文件：先复制到目标位置的临时文件，再原子地替换目标文件
pub async fn copy_file_safe_async<P1: AsRef<Path>, P2: AsRef<Path>>(
    from: P1,
    to: P2,
) -> anyhow::Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    prepare_parent_async(to).await?;
    let temp = temp_path(to);
    let result = async {
        let data = smol::fs::read(from)
            .await
            .with_context(|| format!("读取文件 {} 失败", from.display()))?;
        smol::fs::write(&temp, &data)
            .await
            .with_context(|| format!("写入临时文件 {} 失败", temp.display()))?;
        smol::fs::rename(&temp, to)
            .await
            .with_context(|| format!("替换文件 {} 失败", to.display()))?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = smol::fs::remove_file(&temp).await;
    }
    result
}

/// 安全地写入文件：先写入临时文件，再原子地替换目标文件
pub async fn write_file_safe_async<P: AsRef<Path>>(
    path: P,
    content: impl AsRef<[u8]> + Send,
) -> anyhow::Result<()> {
    let path = path.as_ref();
    prepare_parent_async(path).await?;
    let temp = temp_path(path);
    let result = async {
        smol::fs::write(&temp, content.as_ref())
            .await
            .with_context(|| format!("写入临时文件 {} 失败", temp.display()))?;
        smol::fs::rename(&temp, path)
            .await
            .with_context(|| format!("替换文件 {} 失败", path.display()))?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = smol::fs::remove_file(&temp).await;
    }
    result
}

/// 安全地写入文件（同步版本）：先写入临时文件，再原子地替换目标文件。
///
/// 与 [`write_file_safe_async`] 使用同一套临时文件命名和错误文案，
/// 避免关窗等同步落盘场景直接覆盖目标文件，在写入过程中崩溃会截断配置。
pub fn write_file_safe_sync<P: AsRef<Path>>(
    path: P,
    content: impl AsRef<[u8]>,
) -> anyhow::Result<()> {
    let path = path.as_ref();
    prepare_parent_sync(path)?;
    let temp = temp_path(path);
    let result = (|| {
        std::fs::write(&temp, content.as_ref())
            .with_context(|| format!("写入临时文件 {} 失败", temp.display()))?;
        std::fs::rename(&temp, path)
            .with_context(|| format!("替换文件 {} 失败", path.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

/// 创建目标文件的父目录。配置首次保存到还不存在的目录时，
/// 临时文件本身也会因父目录缺失而无法创建。
fn prepare_parent_sync(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录 {} 失败", parent.display()))?;
    }
    Ok(())
}

async fn prepare_parent_async(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        smol::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("创建目录 {} 失败", parent.display()))?;
    }
    Ok(())
}

/// 生成每次调用都唯一的临时文件路径。
///
/// * 必须与目标文件位于同一目录，`rename` 才是同文件系统内的原子替换；
/// * 必须包含 pid 与进程内自增序号，避免异步保存/同步关窗保存共用同一路径，
///   也避免多个启动器实例互相消费对方的临时文件。
fn temp_path(path: &Path) -> PathBuf {
    let base = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "scl-tmp-file".to_owned());
    let sequence = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!("{base}.{}.{sequence}.scl-tmp", std::process::id()))
}

#[cfg(test)]
mod safe_write_tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "scl_safe_write_{}_{}_{tag}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).expect("创建测试目录");
        dir
    }

    fn temp_residue(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .expect("读取测试目录")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".scl-tmp"))
            .collect()
    }

    #[test]
    fn sync_first_save_creates_missing_target_without_residue() {
        let dir = temp_dir("sync_first");
        let path = dir.join(".scl.json");
        assert!(!path.exists(), "用例前提：目标文件不存在");

        write_file_safe_sync(&path, b"first-save").expect("目标不存在时应能首次保存");

        assert!(path.exists(), "首次保存必须创建目标文件");
        assert_eq!(fs::read_to_string(&path).unwrap(), "first-save");
        assert!(
            temp_residue(&dir).is_empty(),
            "首次保存后不应残留临时文件：{:?}",
            temp_residue(&dir)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sync_save_overwrites_existing_target_without_residue() {
        let dir = temp_dir("sync_overwrite");
        let path = dir.join(".scl.json");
        fs::write(&path, "old-content").expect("预置旧内容");

        write_file_safe_sync(&path, b"new-content").expect("覆盖保存应成功");

        assert_eq!(fs::read_to_string(&path).unwrap(), "new-content");
        assert!(
            temp_residue(&dir).is_empty(),
            "覆盖保存后不应残留临时文件：{:?}",
            temp_residue(&dir)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sync_save_creates_missing_parent_directory() {
        let dir = temp_dir("sync_parent");
        let path = dir.join("nested").join(".scl.json");

        write_file_safe_sync(&path, b"nested").expect("父目录不存在时应创建父目录后保存");

        assert_eq!(fs::read_to_string(&path).unwrap(), "nested");
        assert!(temp_residue(path.parent().unwrap()).is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn async_first_save_and_overwrite_without_residue() {
        let dir = temp_dir("async");
        let path = dir.join(".scl.json");

        smol::block_on(write_file_safe_async(&path, b"async-first")).expect("异步首次保存应成功");
        assert!(path.exists(), "异步首次保存必须创建目标文件");
        assert_eq!(fs::read_to_string(&path).unwrap(), "async-first");

        smol::block_on(write_file_safe_async(&path, b"async-second")).expect("异步覆盖保存应成功");
        assert_eq!(fs::read_to_string(&path).unwrap(), "async-second");
        assert!(
            temp_residue(&dir).is_empty(),
            "异步保存后不应残留临时文件：{:?}",
            temp_residue(&dir)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn concurrent_sync_and_async_writes_to_same_target_all_succeed() {
        // 复现关窗场景：异步防抖保存还在写时，同步关窗保存同时对同一个
        // `.scl.json` 落盘。旧实现共用 `.scl.json.scl-tmp`，后执行 rename
        // 的一方会拿到 ENOENT。
        let dir = temp_dir("concurrent");
        let path = dir.join(".scl.json");

        let mut threads = Vec::new();
        for thread_index in 0..4 {
            let path = path.clone();
            threads.push(std::thread::spawn(move || {
                for write_index in 0..25 {
                    write_file_safe_sync(
                        &path,
                        format!("sync-{thread_index}-{write_index}").into_bytes(),
                    )
                    .expect("并发同步保存应成功");
                }
            }));
        }

        let async_path = path.clone();
        threads.push(std::thread::spawn(move || {
            for write_index in 0..25 {
                smol::block_on(write_file_safe_async(
                    &async_path,
                    format!("async-{write_index}").into_bytes(),
                ))
                .expect("并发异步保存应成功");
            }
        }));

        for thread in threads {
            thread.join().expect("并发写入线程不应 panic");
        }

        let content = fs::read_to_string(&path).expect("最终配置应可读取");
        assert!(
            content.starts_with("sync-") || content.starts_with("async-"),
            "最终内容应来自某次完整写入，而不是被截断：{content:?}"
        );
        assert!(
            temp_residue(&dir).is_empty(),
            "并发保存结束后不应残留临时文件：{:?}",
            temp_residue(&dir)
        );
        let _ = fs::remove_dir_all(dir);
    }
}
