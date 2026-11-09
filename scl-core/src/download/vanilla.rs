//! 原版游戏的下载模块

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::Path,
    time::Duration,
};

use anyhow::Context;
use tracing::*;

use super::{
    download_pool, run_parallel_tasks,
    structs::{AssetIndexes, AssetItem, VersionManifest},
    DownloadSource, Downloader,
};

/// 客户端 JAR 单源的有界等待时间（体积最大，留足慢速网络的余量）
const JAR_SOURCE_TIMEOUT: Duration = Duration::from_secs(300);
/// 依赖库/资源对象单源的有界等待时间
///
/// 正常网络下远小于该值；只有镜像源不可达（TCP 连接停在 SYN-SENT）时才会触发。
const ASSET_SOURCE_TIMEOUT: Duration = Duration::from_secs(120);
/// 元数据（版本清单、资源索引）单源的有界等待时间
const META_TIMEOUT: Duration = Duration::from_secs(15);

/// 依次尝试各下载源，每个源限时等待，超时/失败后继续下一个源
///
/// 直接复用 [`crate::http::download_verified_with_timeout_progress`] 的单源重试、
/// 临时文件与 SHA1/大小校验语义，避免坏内容被 rename 到目标路径；流式字节
/// 增量交给调用方累计进度。
fn download_with_sources_progress(
    uris: &[String],
    dest: &str,
    sha1: Option<&str>,
    size: Option<u64>,
    per_source: Duration,
    on_progress: &mut dyn FnMut(i64),
) -> DynResult {
    let mut last_err = None;
    for uri in uris {
        match crate::http::download_verified_with_timeout_progress(
            std::slice::from_ref(uri),
            dest,
            0,
            sha1,
            size,
            Some(per_source),
            on_progress,
        ) {
            Ok(()) => return Ok(()),
            Err(err) => {
                warn!("从 {} 下载 {} 失败：{:?}", uri, dest, err);
                last_err = Some(err);
            }
        }
        // 重复的库条目会并发下载同一目标（1.5.2 的 jinput-platform 就有两条），
        // 另一任务 rename 成功会让本任务的临时文件消失并报 ENOENT。仅在
        // 「错误是 NotFound 且目标文件已存在」时视为本次补全已完成，避免把
        // 真正的下载失败或残留损坏文件误判为成功。
        let not_found = last_err
            .as_ref()
            .and_then(|err| err.downcast_ref::<std::io::Error>())
            .map(|err| err.kind() == std::io::ErrorKind::NotFound)
            .unwrap_or(false);
        if not_found && std::path::Path::new(dest).is_file() {
            return Ok(());
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("没有可用的下载源")))
}

/// 判断本地文件是否满足期望的大小/哈希。
///
/// `verify_data=false` 时只校验大小（P0「修复默认只补缺失」）；只有显式启用
/// 深度校验时才计算 SHA1。期望大小为 0 或 SHA1 为空表示跳过对应检查。
fn local_file_matches(path: &Path, sha1: &str, size: u64, verify_data: bool) -> DynResult<bool> {
    if !path.is_file() {
        return Ok(false);
    }
    if size > 0 {
        if fs_err::metadata(path)?.len() != size {
            return Ok(false);
        }
    }
    if verify_data && !sha1.is_empty() {
        let mut file = fs_err::File::open(path)?;
        return Ok(crate::utils::get_data_sha1(&mut file)?.eq_ignore_ascii_case(sha1));
    }
    Ok(true)
}

fn asset_object_path(objects_dir: &str, sha1: &str) -> String {
    format!("{objects_dir}/{}/{sha1}", &sha1[..2])
}

fn asset_virtual_path(objects_dir: &str, virtual_id: &str, name: &str) -> String {
    let assets_root = Path::new(objects_dir)
        .parent()
        .unwrap_or_else(|| Path::new(objects_dir));
    format!(
        "{}/virtual/{virtual_id}/{name}",
        assets_root.to_string_lossy()
    )
}

/// 一个待下载的依赖库任务
enum LibraryTask<'a> {
    Common {
        sha1: Cow<'a, str>,
        path: Cow<'a, str>,
        size: u64,
    },
    Native {
        platform: Cow<'a, str>,
        sha1: Cow<'a, str>,
        path: Cow<'a, str>,
        size: u64,
    },
}

/// 从版本元数据构建依赖库下载任务
///
/// 独立成函数是为了在启动并行下载前先登记总字节数，避免一边下载一边增大分母。
fn build_library_tasks(libraries: &[Library]) -> Vec<LibraryTask<'_>> {
    // Minecraft 在早期和现在对记录原生库的方式不一样：
    // 早期记录 `natives` + `downloads.classifiers`；现代直接记录
    // `org.lwjgl:lwjgl:3.2.2:natives-windows` 这样的 name。
    // SCL 下载所有平台的原生库，再按平台分别解压。
    let mut tasks = Vec::with_capacity(libraries.len() * 2);
    for lib in libraries.iter() {
        if let Some(downloads) = &lib.downloads {
            if let Some(classifier) = &downloads.classifiers {
                for (platform, meta) in classifier.iter() {
                    let target_platform = match platform.as_str() {
                        "natives-osx" => "natives-macos",
                        other => other,
                    };
                    tasks.push(LibraryTask::Native {
                        platform: target_platform.into(),
                        sha1: meta.sha1.as_str().into(),
                        path: meta.path.as_str().into(),
                        size: meta.size as u64,
                    });
                }
            }
            if let Some(artifact) = &downloads.artifact {
                if let Some(i) = lib.name.find(":natives-") {
                    let (_, platform) = lib.name.split_at(i + 1);
                    tasks.push(LibraryTask::Native {
                        platform: platform.into(),
                        sha1: artifact.sha1.as_str().into(),
                        path: artifact.path.as_str().into(),
                        size: artifact.size as u64,
                    });
                } else if lib.rules.is_allowed() {
                    tasks.push(LibraryTask::Common {
                        sha1: artifact.sha1.as_str().into(),
                        path: artifact.path.as_str().into(),
                        size: artifact.size as u64,
                    });
                }
            }
        }
    }
    tasks
}

fn library_tasks_total_size(tasks: &[LibraryTask<'_>]) -> u64 {
    tasks
        .iter()
        .map(|task| match task {
            LibraryTask::Common { size, .. } | LibraryTask::Native { size, .. } => (*size).max(1),
        })
        .sum()
}

/// 原生库解压阶段的进度单位数（每个平台一组）
///
/// 在下载开始前就可从任务列表推导出来，用于提前登记分母，避免资源与依赖库
/// 都下载完成后再增大分母造成完成度回退。
fn library_tasks_native_platforms(tasks: &[LibraryTask<'_>]) -> usize {
    tasks
        .iter()
        .filter_map(|task| match task {
            LibraryTask::Native { platform, .. } => Some(platform.as_ref()),
            LibraryTask::Common { .. } => None,
        })
        .collect::<HashSet<_>>()
        .len()
}
use crate::{
    download::VersionInfo,
    prelude::*,
    progress::Reporter,
    version::structs::{Allowed, Library, VersionMeta},
};

/// 一个用于下载安装原版的扩展特质，可以使用 [`crate::download::Downloader`] 来安装
pub trait VanillaDownloadExt: Sync {
    /// 获取现在所有可下载版本
    fn get_avaliable_vanilla_versions(&self) -> DynResult<VersionManifest>;

    /// 下载原版客户端 JAR 文件
    fn download_vanilla_jar(&self, path: &str, save_path: &str, sha1: &str, size: u64)
        -> DynResult;

    /// 下载一个依赖库，并存放到指定位置
    fn download_library(&self, sha1: String, path: String, size: u64, save_path: &str)
        -> DynResult;

    /// 下载一组依赖库，安装位置由特质实现而定
    fn download_libraries(&self, libraries: &[Library]) -> DynResult<HashMap<String, Vec<String>>>;

    /// 下载游戏资源索引
    fn download_asset_index(
        &self,
        name: &str,
        url: &str,
        save_path: &str,
    ) -> DynResult<AssetIndexes>;

    /// 下载一个游戏素材对象到 `assets/objects`。
    ///
    /// 旧版索引的 `assets/virtual/<id>/<name>` 副本由
    /// [`VanillaDownloadExt::materialize_virtual_assets`] 统一生成。
    fn download_asset(
        &self,
        sha1: &str,
        name: &str,
        save_path: &str,
        size: u64,
        r: Option<impl Reporter>,
    ) -> DynResult;

    /// 把对象存储中的资源复制到 `assets/virtual/<id>/<name>`（1.6.x legacy / pre-1.6）
    fn materialize_virtual_assets(
        &self,
        save_path: &str,
        virtual_id: &str,
        index: &AssetIndexes,
        r: Option<impl Reporter>,
    ) -> DynResult<usize>;

    /// 下载一个游戏版本
    fn download_vanilla(
        &self,
        version_name: &str,
        version_meta: &VersionMeta,
        is_repair: bool,
    ) -> DynResult;

    /// 安装一个游戏版本
    fn install_vanilla(&self, version_name: &str, version_info: &VersionInfo) -> DynResult;

    /// 只准备原版运行前提（依赖库、资源、原生库），不写版本 JSON 与游戏 JAR
    ///
    /// 供安装器型加载器使用：它们会自行生成目标版本元数据，被补丁过的游戏 JAR
    /// 也位于 libraries 目录，版本目录不应再出现一份原版 JAR。
    fn install_vanilla_prerequisites(
        &self,
        version_name: &str,
        version_info: &VersionInfo,
    ) -> DynResult;
}

impl<R: Reporter> VanillaDownloadExt for Downloader<R> {
    fn get_avaliable_vanilla_versions(&self) -> DynResult<VersionManifest> {
        let uri = match self.source {
            DownloadSource::Default => {
                "https://piston-meta.mojang.com/mc/game/version_manifest.json"
            }
            DownloadSource::BMCLAPI => {
                "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json"
            }
            _ => "https://piston-meta.mojang.com/mc/game/version_manifest.json",
        };
        crate::http::retry_get_json_timeout(uri, Some(META_TIMEOUT))
            .map_err(|e| anyhow::anyhow!("获取可用原版列表失败：{:?}", e))
    }

    fn download_vanilla_jar(
        &self,
        path: &str,
        save_path: &str,
        sha1: &str,
        size: u64,
    ) -> DynResult {
        let r = self.reporter.sub();
        r.add_max_progress(size.max(1) as f64);
        self.download_vanilla_jar_reported(&r, path, save_path, sha1, size)
    }

    fn download_library(
        &self,
        sha1: String,
        path: String,
        size: u64,
        save_path: &str,
    ) -> DynResult {
        let r = self.reporter.sub();
        r.add_max_progress(size.max(1) as f64);
        self.download_library_reported(&r, sha1, path, size, save_path)
    }

    fn download_asset_index(
        &self,
        name: &str,
        url: &str,
        save_path: &str,
    ) -> DynResult<AssetIndexes> {
        let r = self.reporter.sub();
        let full_path = format!("{save_path}/indexes/{name}.json");
        // 存在且可解析的索引直接复用，避免每次安装/修复都重新请求并覆盖写盘。
        if let Ok(data) = fs_err::read(&full_path) {
            match serde_json::from_slice::<AssetIndexes>(&data) {
                Ok(index) => {
                    debug!("复用已有资源索引 {name}");
                    return Ok(index);
                }
                Err(err) => {
                    warn!("资源索引 {name} 无法解析，重新下载：{err}");
                }
            }
        }
        r.set_message(format!("正在下载原版资源索引 {name}"));
        fs_err::create_dir_all(
            &full_path[..full_path
                .rfind(std::path::is_separator)
                .unwrap_or(full_path.len())],
        )?;
        let p = {
            let url = url.parse::<url::Url>()?;
            let p = url.path();
            p.to_owned()
        };
        let uris = [
            match self.source {
                DownloadSource::Default => {
                    format!("https://launchermeta.mojang.com{p}")
                }
                DownloadSource::BMCLAPI => {
                    format!("https://bmclapi2.bangbang93.com{p}")
                }
                _ => format!("https://launchermeta.mojang.com{p}"),
            },
            format!("https://bmclapi2.bangbang93.com{p}"),
            format!("https://launchermeta.mojang.com{p}"),
        ];
        for uri in &uris {
            let res = match crate::http::retry_get_bytes_timeout(uri, Some(META_TIMEOUT)) {
                Ok(res) => res,
                Err(err) => {
                    warn!("获取资源索引 {name} 失败：{uri}：{err:?}");
                    continue;
                }
            };
            if res.is_empty() {
                warn!("资源索引 {name} 返回为空，尝试下一个链接");
                continue;
            }
            fs_err::write(&full_path, &res)?;
            self.mark_file_repaired();
            return Ok(serde_json::from_slice(&res)?);
        }
        anyhow::bail!("获取素材索引失败，已尝试的链接：{}", uris.join("\n"))
    }
    fn download_asset(
        &self,
        sha1: &str,
        name: &str,
        save_path: &str,
        size: u64,
        r: Option<impl Reporter>,
    ) -> DynResult {
        let object_path = asset_object_path(save_path, sha1);
        r.set_message(format!("正在下载原版资源 {name}"));
        if local_file_matches(Path::new(&object_path), sha1, size, self.verify_data)? {
            r.add_progress(size.max(1) as f64);
            return Ok(());
        }
        fs_err::create_dir_all(
            &object_path[..object_path
                .rfind(std::path::is_separator)
                .unwrap_or(object_path.len())],
        )?;
        let sub_hash = &sha1[..2];
        let uris = [
            match self.source {
                DownloadSource::Default => {
                    format!("https://resources.download.minecraft.net/{sub_hash}/{sha1}")
                }
                DownloadSource::BMCLAPI => {
                    format!("https://bmclapi2.bangbang93.com/assets/{sub_hash}/{sha1}")
                }
                _ => format!("https://resources.download.minecraft.net/{sub_hash}/{sha1}"),
            },
            format!("https://bmclapi2.bangbang93.com/assets/{sub_hash}/{sha1}"),
            format!("https://resources.download.minecraft.net/{sub_hash}/{sha1}"),
        ];
        let mut report = |delta: i64| r.add_progress(delta as f64);
        download_with_sources_progress(
            &uris,
            &object_path,
            Some(sha1),
            (size > 0).then_some(size),
            ASSET_SOURCE_TIMEOUT,
            &mut report,
        )
        .map_err(|e| {
            anyhow::anyhow!(
                "下载资源文件失败 {:?}，已尝试的链接：{}",
                e,
                uris.join("\n")
            )
        })?;
        self.mark_file_repaired();
        if size == 0 {
            r.add_progress(1.);
        }
        Ok(())
    }

    fn materialize_virtual_assets(
        &self,
        save_path: &str,
        virtual_id: &str,
        index: &AssetIndexes,
        r: Option<impl Reporter>,
    ) -> DynResult<usize> {
        let mut copied = 0;
        r.set_message("正在映射旧版资源".into());
        r.add_max_progress(index.objects.len() as f64);
        for (name, obj) in &index.objects {
            let object_path = asset_object_path(save_path, &obj.hash);
            let virtual_path = asset_virtual_path(save_path, virtual_id, name);
            let complete = local_file_matches(
                Path::new(&virtual_path),
                &obj.hash,
                obj.size as u64,
                self.verify_data,
            )
            .unwrap_or(false);
            if !complete {
                if !Path::new(&object_path).is_file() {
                    warn!("资源对象缺失，无法映射到虚拟目录：{name} ({})", obj.hash);
                    r.add_progress(1.);
                    continue;
                }
                fs_err::create_dir_all(
                    &virtual_path[..virtual_path
                        .rfind(std::path::is_separator)
                        .unwrap_or(virtual_path.len())],
                )?;
                fs_err::copy(&object_path, &virtual_path)?;
                self.mark_file_repaired();
                copied += 1;
            }
            r.add_progress(1.);
        }
        Ok(copied)
    }

    fn download_libraries(&self, libraries: &[Library]) -> DynResult<HashMap<String, Vec<String>>> {
        let lr = self.reporter.sub();
        lr.set_message("正在检索并安装需要安装的依赖库".into());
        let tasks = build_library_tasks(libraries);
        // 总量在任务开始前一次性登记，避免下载过程中分母增长导致完成度回退。
        lr.add_max_progress(library_tasks_total_size(&tasks) as f64);
        let result = self.run_library_tasks(&lr, &tasks);
        lr.remove_progress();
        result
    }

    fn download_vanilla(
        &self,
        version_name: &str,
        version_meta: &VersionMeta,
        is_repair: bool,
    ) -> DynResult {
        info!("开始下载原版游戏 {version_name}");
        let r = self.reporter.sub();
        let game_file = format!(
            "{}/{}/{}.jar",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        r.set_message(format!("正在下载原版游戏 {version_name}"));
        let main_jar = version_meta
            .downloads
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("无法获取下载清单"))?
            .get("client")
            .ok_or_else(|| anyhow::anyhow!("无法获取客户端下载元数据"))?;

        if is_repair {
            let lib_path = std::path::Path::new(self.minecraft_library_path.as_str());
            let lib_path = lib_path
                .join("org")
                .join("glavo")
                .join("1.0")
                .join("log4j-patch");
            if !lib_path.is_dir() {
                fs_err::create_dir_all(&lib_path)?;
            }
            let log4j_path = lib_path.join("log4j-patch-agent-1.0.jar");
            // 修复只补缺失：补丁已存在时不重复写盘。
            if !log4j_path.is_file() {
                fs_err::write(&log4j_path, crate::client::LOG4J_PATCH)?;
            }
        }

        let asset_index_meta = version_meta
            .asset_index
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("版本元数据缺少资源索引"))?;
        let assets_index = self.download_asset_index(
            &asset_index_meta.id,
            &asset_index_meta.url,
            self.minecraft_assets_path.as_str(),
        )?;
        // legacy 索引（1.6.x）靠 virtual 标志；pre-1.6 靠 map_to_resources。
        // 两者都需要在 assets/virtual/<索引 id> 下生成按文件名映射的副本。
        let virtual_id: Option<&str> = if assets_index.is_virtual || assets_index.map_to_resources {
            Some(asset_index_meta.id.as_str())
        } else {
            None
        };

        let ar = r.sub();
        let minecraft_assets_objects_path = format!("{}/objects", self.minecraft_assets_path);

        // 对象按 hash 去重下载；virtual/legacy 索引的全部文件名在下载完成后
        // 由 materialize_virtual_assets 统一映射，避免同名 hash 丢名字，也避免
        // 多个任务并发写同一个对象临时文件。
        let mut seen_hashes = HashSet::new();
        let assets_download_tasks: Vec<(&String, &AssetItem)> = assets_index
            .objects
            .iter()
            .filter(|(_, obj)| seen_hashes.insert(obj.hash.clone()))
            .filter(|(_, obj)| {
                !local_file_matches(
                    Path::new(&asset_object_path(
                        &minecraft_assets_objects_path,
                        &obj.hash,
                    )),
                    &obj.hash,
                    obj.size as u64,
                    self.verify_data,
                )
                .unwrap_or(false)
            })
            .collect();
        let assets_total_size: u64 = assets_download_tasks
            .iter()
            .map(|(_, obj)| (obj.size as u64).max(1))
            .sum();

        ar.set_message("下载资源文件".into());
        ar.add_max_progress(assets_total_size as _);
        if !is_repair {
            // JAR 也按字节计入同一个进度，避免 40MB 的下载在进度条上只占 1 个单位。
            r.add_max_progress(main_jar.size.max(1) as f64);
        }

        // 依赖库的总量同样在开始前登记，且与资源/JAR 真正并行下载；
        // 否则库阶段会在资源阶段结束后才把分母加上，造成完成度大幅回退。
        let library_tasks = build_library_tasks(&version_meta.libraries);
        let lr = self.reporter.sub();
        lr.set_message("正在检索并安装需要安装的依赖库".into());
        lr.add_max_progress(library_tasks_total_size(&library_tasks) as f64);
        // 原生库按平台逐个解压，每个平台 1 个进度单位；数量可由任务列表
        // 提前推导，和资源/JAR/依赖库一起登记到根分母。
        r.add_max_progress(library_tasks_native_platforms(&library_tasks) as f64);

        let native_jars = download_pool().install(|| -> DynResult<_> {
            if is_repair {
                let (assets_result, libraries_result) = rayon::join(
                    || {
                        run_parallel_tasks(
                            self.parallel_amount,
                            &assets_download_tasks,
                            |(name, obj)| -> DynResult {
                                self.download_asset(
                                    &obj.hash,
                                    name,
                                    &minecraft_assets_objects_path,
                                    obj.size as u64,
                                    ar.clone(),
                                )
                            },
                        )
                    },
                    || self.run_library_tasks(&lr, &library_tasks),
                );
                assets_result?;
                libraries_result
            } else {
                let ((jar_result, libraries_result), assets_result) = rayon::join(
                    || {
                        rayon::join(
                            || {
                                self.download_vanilla_jar_reported(
                                    &r,
                                    &main_jar.url,
                                    &game_file,
                                    &main_jar.sha1,
                                    main_jar.size as u64,
                                )
                            },
                            || self.run_library_tasks(&lr, &library_tasks),
                        )
                    },
                    || {
                        run_parallel_tasks(
                            self.parallel_amount,
                            &assets_download_tasks,
                            |(name, obj)| -> DynResult {
                                self.download_asset(
                                    &obj.hash,
                                    name,
                                    &minecraft_assets_objects_path,
                                    obj.size as u64,
                                    ar.clone(),
                                )
                            },
                        )
                    },
                );
                jar_result?;
                assets_result?;
                libraries_result
            }
        })?;
        lr.remove_progress();

        // 1.6.x legacy / pre-1.6：对象下载完成后，为索引中的每个文件名生成虚拟副本。
        if let Some(virtual_id) = virtual_id {
            self.materialize_virtual_assets(
                &minecraft_assets_objects_path,
                virtual_id,
                &assets_index,
                ar.sub(),
            )?;
        }

        let native_dir = format!(
            "{}/{}/natives",
            self.minecraft_version_path.as_str(),
            version_name
        );
        // 26.3+ 的 JVM 参数要求 java.library.path 指向 `${natives_directory}/java`，
        // 原生库必须解压到该子目录；老版本仍解压到 `${natives_directory}` 本身。
        let natives_subdir = version_meta
            .natives_use_subdirectories()
            .then_some(NATIVE_SUBDIR);
        let nr = r.sub();
        // 根分母已在下载开始前登记，这里只设置本进度条自己的分母。
        nr.set_max_progress(native_jars.len() as f64);
        nr.set_message("正在解压原生库".into());
        for (platform, items) in native_jars.iter() {
            let native_dir = native_extract_dir(&native_dir, platform, natives_subdir);
            for item in items {
                // 修复只补缺失：目标目录里已有全部 native 文件时跳过解压。
                if natives_need_extract(item, &native_dir) {
                    unzip_natives(item, &native_dir)?;
                }
            }
            nr.add_progress(1.);
        }
        r.remove_progress();

        info!("原版游戏 {version_name} 下载完成！");
        Ok(())
    }

    fn install_vanilla(&self, version_name: &str, version_info: &VersionInfo) -> DynResult {
        self.install_vanilla_with_jar(version_name, version_info, true)
    }

    fn install_vanilla_prerequisites(
        &self,
        version_name: &str,
        version_info: &VersionInfo,
    ) -> DynResult {
        self.install_vanilla_with_jar(version_name, version_info, false)
    }
}

impl<R: Reporter> Downloader<R> {
    /// 安装原版的公共流程
    ///
    /// `with_jar` 为 false 时只准备依赖库、资源和原生库，不写版本 JSON 与游戏
    /// JAR。安装器型加载器（如 NeoForge 26+）会自行生成版本元数据，并把被补丁
    /// 过的游戏 JAR 放到 libraries 下；启动器不应再往版本目录写一份原版 JAR。
    fn install_vanilla_with_jar(
        &self,
        version_name: &str,
        version_info: &VersionInfo,
        with_jar: bool,
    ) -> DynResult {
        // 不再在安装开始时把根分母重置为固定步数：各阶段（资源、JAR、依赖库、
        // 原生库）会在开始前用 `add_max_progress` 一次性登记自己的字节总量，
        // 分母随进度一起单调累加，避免绝对 `set` 把已累计的分母抹掉。
        self.reporter
            .set_message(format!("正在获取版本元数据 {version_name}"));

        fs_err::create_dir_all(format!("{}/indexes", self.minecraft_assets_path))?;
        fs_err::create_dir_all(format!("{}/objects", self.minecraft_assets_path))?;
        fs_err::create_dir_all(self.minecraft_library_path.as_str())?;
        fs_err::create_dir_all(format!(
            "{}/{}",
            self.minecraft_version_path.as_str(),
            version_name
        ))?;

        let version_file = format!(
            "{}/{}/{}.json",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        let url = version_info.url.parse::<url::Url>()?;
        let url_path = url.path();

        let meta_uri = match self.source {
            DownloadSource::Default => format!("https://launchermeta.mojang.com{url_path}"),
            DownloadSource::BMCLAPI => format!("https://bmclapi2.bangbang93.com{url_path}"),
            _ => format!("https://launchermeta.mojang.com{url_path}"),
        };
        let res = crate::http::retry_get_bytes_timeout(&meta_uri, Some(META_TIMEOUT))
            .map_err(|e| anyhow::anyhow!("下载版本元数据失败：{:?}", e))?;

        if with_jar {
            fs_err::write(&version_file, &res)?;
        }

        self.reporter
            .set_message(format!("正在下载游戏文件 {version_name}"));

        let mut version_meta: VersionMeta = serde_json::from_slice(&res)?;

        version_meta.fix_libraries();

        // 修复模式只补依赖库/资源/原生库，不下载客户端 JAR；正好是安装器型
        // 加载器所需的「原版运行前提」。
        self.download_vanilla(version_name, &version_meta, !with_jar)?;

        Ok(())
    }

    /// 执行已登记的依赖库任务，并同时整理原生库解压映射
    fn run_library_tasks(
        &self,
        r: &impl Reporter,
        tasks: &[LibraryTask<'_>],
    ) -> DynResult<HashMap<String, Vec<String>>> {
        let mut native_jars_mapping: HashMap<String, Vec<String>> =
            HashMap::with_capacity(tasks.len());
        for task in tasks {
            if let LibraryTask::Native {
                platform,
                sha1,
                path,
                size: _,
            } = task
            {
                debug!("原生库 {platform} {sha1} {path}");
                let p = format!("{}/{}", self.minecraft_library_path.as_str(), path);
                if let Some(native_jars) = native_jars_mapping.get_mut(&platform.to_string()) {
                    if !native_jars.contains(&p) {
                        native_jars.push(p);
                    }
                } else {
                    let mut native_jars = Vec::with_capacity(tasks.len());
                    native_jars.push(p);
                    native_jars_mapping.insert(platform.to_string(), native_jars);
                }
            }
        }

        run_parallel_tasks(self.parallel_amount, tasks, |task| -> DynResult {
            match task {
                LibraryTask::Common { sha1, path, size } => self.download_library_reported(
                    r,
                    sha1.to_string(),
                    path.to_string(),
                    *size,
                    self.minecraft_library_path.as_str(),
                ),
                LibraryTask::Native {
                    sha1, path, size, ..
                } => self.download_library_reported(
                    r,
                    sha1.to_string(),
                    path.to_string(),
                    *size,
                    self.minecraft_library_path.as_str(),
                ),
            }
        })?;

        Ok(native_jars_mapping)
    }

    /// 下载原版 JAR 并把文件体积作为进度增量
    ///
    /// 调用方负责预先登记 `max_progress`；这里只累加已完成字节数，避免
    /// 多个并行任务在下载过程中反复修改总分母。
    fn download_vanilla_jar_reported(
        &self,
        r: &impl Reporter,
        path: &str,
        save_path: &str,
        sha1: &str,
        size: u64,
    ) -> DynResult {
        let name = &save_path[save_path.rfind(std::path::is_separator).unwrap_or(0) + 1..];
        r.set_message(format!("正在下载原版 {name}"));
        if local_file_matches(Path::new(save_path), sha1, size, self.verify_data)? {
            r.add_progress(size.max(1) as f64);
            return Ok(());
        }
        fs_err::create_dir_all(
            &save_path[..save_path
                .rfind(std::path::is_separator)
                .unwrap_or(save_path.len())],
        )?;
        let path = path.parse::<url::Url>()?;
        let uris = [
            match self.source {
                DownloadSource::Default => {
                    format!("https://launcher.mojang.com{}", path.path())
                }
                DownloadSource::BMCLAPI => {
                    format!("https://bmclapi2.bangbang93.com{}", path.path())
                }
                _ => format!("https://launcher.mojang.com{}", path.path()),
            },
            format!("https://bmclapi2.bangbang93.com{}", path.path()),
            format!("https://launcher.mojang.com{}", path.path()),
        ];
        let mut report = |delta: i64| r.add_progress(delta as f64);
        download_with_sources_progress(
            &uris,
            save_path,
            Some(sha1),
            (size > 0).then_some(size),
            JAR_SOURCE_TIMEOUT,
            &mut report,
        )
        .map_err(|e| anyhow::anyhow!("下载原版游戏 Jar 失败：{:?}", e))?;
        self.mark_file_repaired();
        if size == 0 {
            r.add_progress(1.);
        }
        Ok(())
    }

    /// 下载依赖库并把文件体积作为进度增量（不修改总分母）
    fn download_library_reported(
        &self,
        r: &impl Reporter,
        sha1: String,
        path: String,
        size: u64,
        save_path: &str,
    ) -> DynResult {
        let full_path = format!("{save_path}/{path}");
        r.set_message(format!("正在下载原版库 {path}"));
        if local_file_matches(Path::new(&full_path), &sha1, size, self.verify_data)? {
            r.add_progress(size.max(1) as f64);
            return Ok(());
        }
        fs_err::create_dir_all(
            &full_path[..full_path
                .rfind(std::path::is_separator)
                .unwrap_or(full_path.len())],
        )?;
        let default_uris = [
            match self.source {
                DownloadSource::Default => {
                    format!("https://libraries.minecraft.net/{path}")
                }
                DownloadSource::BMCLAPI => {
                    format!("https://bmclapi2.bangbang93.com/maven/{path}")
                }
                _ => format!("https://libraries.minecraft.net/{path}"),
            },
            format!("https://bmclapi2.bangbang93.com/maven/{path}"),
            format!("https://libraries.minecraft.net/{path}"),
        ];
        let mut report = |delta: i64| r.add_progress(delta as f64);
        download_with_sources_progress(
            &default_uris,
            &full_path,
            (!sha1.is_empty()).then_some(sha1.as_str()),
            (size > 0).then_some(size),
            ASSET_SOURCE_TIMEOUT,
            &mut report,
        )
        .map_err(|e| anyhow::anyhow!("下载库 {} 失败：{:?}", path, e))?;
        self.mark_file_repaired();
        if size == 0 {
            r.add_progress(1.);
        }
        Ok(())
    }
}

const NATIVE_EXTS: &[&str] = &["dll", "so", "dylib", "jnilib"];
/// 26.3+ 原生库的 JVM 搜索子目录名（对应 `java.library.path=${natives_directory}/java`）
const NATIVE_SUBDIR: &str = "java";

/// 计算某个平台的原生库解压目录
///
/// 老版本直接解压到 `<natives>/<platform>`；26.3+ 需要多一层 `java` 子目录，
/// 与 JVM 参数里的 `${natives_directory}/java` 对齐。
fn native_extract_dir(base: &str, platform: &str, subdir: Option<&str>) -> String {
    match subdir {
        Some(subdir) => format!("{base}/{platform}/{subdir}"),
        None => format!("{base}/{platform}"),
    }
}

/// 判断 native 压缩包中是否还有未落盘的文件。
///
/// 打开/解析失败时返回 `true`，交给 [`unzip_natives`] 给出具体错误。
fn natives_need_extract(unzip_file: &str, unzip_dir: &str) -> bool {
    let Ok(file) = fs_err::File::open(unzip_file) else {
        return true;
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return true;
    };
    let dir = Path::new(unzip_dir);
    for i in 0..archive.len() {
        let Ok(file) = archive.by_index(i) else {
            return true;
        };
        let Some(p) = file
            .enclosed_name()
            .and_then(|p| p.file_name().map(|name| name.to_owned()))
        else {
            continue;
        };
        let Some(ext) = Path::new(&p).extension() else {
            continue;
        };
        if !NATIVE_EXTS.contains(&ext.to_str().unwrap_or_default()) {
            continue;
        }
        if !dir.join(&p).is_file() {
            return true;
        }
    }
    false
}

/// 解压指定 ZIP 压缩文件的内容到指定文件夹
///
/// 如果文件夹不存在则创建
pub fn unzip_natives(unzip_file: &str, unzip_dir: &str) -> DynResult {
    let file = fs_err::File::open(unzip_file)?;
    let dir = std::path::PathBuf::from(unzip_dir);
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("解压原生库 {unzip_file} 时发生错误"))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        // zip 2.x 的 `enclosed_name` 返回拥有所有权的 PathBuf，这里直接取出文件名。
        let p = match file
            .enclosed_name()
            .and_then(|p| p.file_name().map(|name| name.to_owned()))
        {
            Some(p) => p.to_owned(),
            None => continue,
        };
        if let Some(ext) = Path::new(&p).extension() {
            if !NATIVE_EXTS.contains(&ext.to_str().unwrap_or_default()) {
                continue;
            }
        } else {
            continue;
        }
        let save_path = dir.join(&p);
        let save_dir = save_path.parent().unwrap();
        let _ = fs_err::create_dir_all(save_dir);
        debug!(
            "解压原生库 {} 到 {}",
            p.to_string_lossy(),
            save_path.display()
        );
        let mut output = fs_err::File::create(save_path)?;
        std::io::copy(&mut file, &mut output)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_extract_dir_matches_metadata_generation() {
        let legacy: VersionMeta = serde_json::from_str(
            r#"{
                "mainClass": "net.minecraft.client.main.Main",
                "arguments": {
                    "game": [],
                    "jvm": ["-Djava.library.path=${natives_directory}"]
                },
                "libraries": []
            }"#,
        )
        .expect("解析旧版元数据");
        assert!(
            !legacy.natives_use_subdirectories(),
            "1.21.x 应保持平铺布局"
        );
        assert_eq!(
            native_extract_dir("/v/1.21.1/natives", "natives-linux", None),
            "/v/1.21.1/natives/natives-linux"
        );

        let modern: VersionMeta = serde_json::from_str(
            r#"{
                "mainClass": "net.minecraft.client.main.Main",
                "arguments": {
                    "game": [],
                    "jvm": [
                        "-Djava.library.path=${natives_directory}/java",
                        "-Djna.tmpdir=${natives_directory}/jna",
                        "-Dorg.lwjgl.system.SharedLibraryExtractPath=${natives_directory}/lwjgl",
                        "-Dio.netty.native.workdir=${natives_directory}/netty"
                    ]
                },
                "libraries": []
            }"#,
        )
        .expect("解析 26.3 元数据");
        assert!(
            modern.natives_use_subdirectories(),
            "26.3 应识别为子目录布局"
        );
        assert_eq!(
            native_extract_dir("/v/26.3/natives", "natives-linux", Some(NATIVE_SUBDIR)),
            "/v/26.3/natives/natives-linux/java"
        );
    }

    /// 索引文件存在且可解析时必须复用，不能发起网络请求。
    #[test]
    fn reuses_existing_asset_index() {
        let dir =
            std::env::temp_dir().join(format!("scl_asset_index_reuse_{}", std::process::id()));
        let _ = fs_err::remove_dir_all(&dir);
        fs_err::create_dir_all(dir.join("assets").join("indexes")).expect("创建索引目录");
        fs_err::write(
            dir.join("assets").join("indexes").join("legacy.json"),
            br#"{"virtual":true,"objects":{}}"#,
        )
        .expect("写入索引");
        let downloader: Downloader<()> = Downloader::default().with_minecraft_path(&dir);
        let assets_path = dir.join("assets").to_string_lossy().to_string();
        let index = downloader
            .download_asset_index("legacy", "http://127.0.0.1:1/unreachable", &assets_path)
            .expect("已存在的索引应直接复用");
        assert!(index.is_virtual);
        assert!(index.objects.is_empty());
        let _ = fs_err::remove_dir_all(&dir);
    }
}
