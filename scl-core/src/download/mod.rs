//! 游戏资源下载模块，所有的游戏/模组/模组中文名称等数据的获取和安装都在这里

pub mod authlib;
pub mod curseforge;
pub mod fabric;
pub mod forge;
pub mod mcmod;
pub mod modrinth;
pub mod neoforge;
pub mod optifine;
pub mod quiltmc;
pub mod structs;
pub mod vanilla;

use std::{
    fmt::Display,
    path::Path,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, OnceLock,
    },
};

use anyhow::Context;
pub use authlib::AuthlibDownloadExt;
pub use fabric::FabricDownloadExt;
pub use forge::ForgeDownloadExt;
pub use neoforge::NeoForgeDownloadExt;
pub use optifine::OptifineDownloadExt;
pub use quiltmc::QuiltMCDownloadExt;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
pub use vanilla::VanillaDownloadExt;

use self::structs::VersionInfo;
use crate::{path::*, prelude::*, progress::*};

/// 下载线程池的线程数上限
///
/// 下载属于阻塞 I/O，不受 CPU 核数限制；默认并发量也是 64。
const DOWNLOAD_THREADS: usize = 64;

/// 全局下载线程池（rayon）
pub(crate) fn download_pool() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(DOWNLOAD_THREADS)
            .thread_name(|index| format!("scl-download-{index}"))
            .build()
            .expect("创建下载线程池失败")
    })
}

/// 在下载线程池中并行执行一组任务，并按 `parallel_amount` 分批限制并发。
///
/// `parallel_amount == 0` 表示不限制（一次性提交全部任务）。
pub(crate) fn run_parallel_tasks<T, E>(
    parallel_amount: usize,
    tasks: &[T],
    f: impl Fn(&T) -> Result<(), E> + Sync + Send,
) -> Result<(), E>
where
    T: Sync,
    E: Send,
{
    if tasks.is_empty() {
        return Ok(());
    }
    let limit = if parallel_amount == 0 {
        tasks.len()
    } else {
        parallel_amount
    };
    for chunk in tasks.chunks(limit) {
        download_pool().install(|| chunk.par_iter().try_for_each(&f))?;
    }
    Ok(())
}

/// 游戏的下载来源，支持和 BMCLAPI 同格式的自定义镜像源
///
/// 通常国内的镜像源速度是比官方快的，但是更新不如官方的及时
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum DownloadSource {
    /// 全部使用原始来源下载
    ///
    /// 兼容旧配置：已失效的 MCBBS 下载源在反序列化时回退到默认源。
    #[default]
    #[serde(alias = "MCBBS")]
    Default,
    /// 全部使用 BMCLAPI 提供的镜像源下载
    ///
    /// 为了支持镜像源，在这里鼓励大家前去支持一下：<https://afdian.net/a/bangbang93>
    BMCLAPI,
    /// 使用符合 BMCLAPI 镜像链接格式的自定义镜像源下载
    Custom(url::Url),
}

impl Display for DownloadSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DownloadSource::Default => "默认（官方）下载源",
                DownloadSource::BMCLAPI => "BMCLAPI 下载源",
                DownloadSource::Custom(_) => "自定义",
            }
        )
    }
}

impl FromStr for DownloadSource {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Offical" => Ok(Self::Default),
            "BMCLAPI" => Ok(Self::BMCLAPI),
            // 已失效的 MCBBS 下载源：旧配置回退到默认源。
            "MCBBS" => Ok(Self::Default),
            s => {
                let url = s.parse::<url::Url>();
                if let Ok(url) = url {
                    Ok(Self::Custom(url))
                } else {
                    Ok(Self::Default)
                }
            }
        }
    }
}

/// 下载结构，用于存储下载所需的信息，并通过附带的扩展特质下载需要的东西
#[derive(Debug)]
pub struct Downloader<R> {
    /// 使用的下载源
    pub source: DownloadSource,
    /// 当前的 Minecraft 游戏目录路径
    pub(crate) minecraft_path: String,
    /// 当前的 Minecraft 依赖库目录路径
    pub(crate) minecraft_library_path: String,
    /// 当前的 Minecraft 版本文件夹目录路径
    pub(crate) minecraft_version_path: String,
    /// 当前的 Minecraft 资源文件夹目录路径
    pub(crate) minecraft_assets_path: String,
    /// 是否使用版本独立方式安装
    ///
    /// 这个会影响 Optifine 以模组形式的安装路径
    ///
    /// （会被安装在 版本/mods 文件夹里还是 .minecraft/mods 文件夹里）
    pub game_independent: bool,
    /// 是否验证已存在的文件是否正确
    pub verify_data: bool,
    /// 任意的 Java 运行时执行文件目录
    ///
    /// 在安装 Forge 时会使用
    pub java_path: String,
    /// 下载并发量
    parallel_amount: usize,
    /// 本次下载/修复过程中实际写入或复制成功的文件数
    ///
    /// 用于让「补全/修复版本文件」报告真实动作数，而不是处理成功的版本数。
    repaired_files: Arc<AtomicUsize>,
    /// 下载的进度报告对象
    pub reporter: Option<R>,
}

impl<R> Downloader<R> {
    /// 设置安装的目录，传入一个 `.minecraft` 文件夹路径作为参数
    ///
    /// 游戏将会被安装到此处
    pub fn set_minecraft_path(&mut self, dot_minecraft_path: impl AsRef<Path>) {
        let dot_minecraft_path = dot_minecraft_path.as_ref().to_path_buf();
        self.minecraft_path = dot_minecraft_path.to_string_lossy().to_string();
        self.minecraft_library_path = dot_minecraft_path
            .join("libraries")
            .to_string_lossy()
            .to_string();
        self.minecraft_version_path = dot_minecraft_path
            .join("versions")
            .to_string_lossy()
            .to_string();
        self.minecraft_assets_path = dot_minecraft_path
            .join("assets")
            .to_string_lossy()
            .to_string();
    }

    /// Builder 模式的 [`Downloader::set_minecraft_path`]
    pub fn with_minecraft_path(mut self, dot_minecraft_path: impl AsRef<Path>) -> Self {
        self.set_minecraft_path(dot_minecraft_path);
        self
    }
}

impl<R: Reporter> Clone for Downloader<R> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            minecraft_path: self.minecraft_path.clone(),
            minecraft_library_path: self.minecraft_library_path.clone(),
            minecraft_version_path: self.minecraft_version_path.clone(),
            minecraft_assets_path: self.minecraft_assets_path.clone(),
            game_independent: self.game_independent,
            verify_data: self.verify_data,
            java_path: self.java_path.clone(),
            reporter: self.reporter.clone(),
            parallel_amount: self.parallel_amount,
            repaired_files: self.repaired_files.clone(),
        }
    }
}

impl<R: Reporter> Downloader<R> {
    /// 设置一个进度报告对象，下载进度将会被上报给这个对象
    #[must_use]
    pub fn with_reporter(mut self, reporter: R) -> Self {
        self.reporter = Some(reporter);
        self
    }
    /// 设置一个下载源
    #[must_use]
    pub fn with_source(mut self, source: DownloadSource) -> Self {
        self.source = source;
        self
    }
    /// 设置一个 Java 运行时，安装 Forge 和 Optifine 时需要用到
    #[must_use]
    pub fn with_java(mut self, java_path: String) -> Self {
        self.java_path = java_path;
        self
    }
    /// 设置是否使用版本独立方式安装
    ///
    /// 这个会影响 Optifine 以模组形式的安装路径
    ///
    /// （会被安装在 版本/mods 文件夹里还是 .minecraft/mods 文件夹里）
    #[must_use]
    pub fn with_game_independent(mut self, game_independent: bool) -> Self {
        self.game_independent = game_independent;
        self
    }
    /// 设置下载时的并发量，如果为 0 则不限制
    #[must_use]
    pub fn with_parallel_amount(mut self, limit: usize) -> Self {
        self.parallel_amount = limit;
        self
    }
    /// 是否强制校验已下载的文件以确认是否需要重新下载
    ///
    /// 如不强制则仅检测文件是否存在
    #[must_use]
    pub fn with_verify_data(mut self) -> Self {
        self.verify_data = true;
        self
    }

    /// 记录一个实际写入或复制的文件
    pub(crate) fn mark_file_repaired(&self) {
        self.repaired_files.fetch_add(1, Ordering::Relaxed);
    }

    /// 本次下载器实际写入或复制的文件数
    pub fn repaired_file_count(&self) -> usize {
        self.repaired_files.load(Ordering::Relaxed)
    }
}
impl<R: Reporter> Default for Downloader<R> {
    fn default() -> Self {
        Self {
            source: DownloadSource::Default,
            minecraft_path: minecraft_path().to_owned(),
            minecraft_library_path: minecraft_libraries_path(),
            minecraft_version_path: minecraft_versions_path(),
            minecraft_assets_path: minecraft_assets_path(),
            game_independent: false,
            verify_data: false,
            java_path: {
                #[cfg(windows)]
                {
                    "javaw.exe".into()
                }
                #[cfg(not(windows))]
                {
                    "java".into()
                }
            },
            reporter: None,
            parallel_amount: 64,
            repaired_files: Arc::new(AtomicUsize::new(0)),
        }
    }
}

/// 一个游戏安装特质，如果你并不需要单独安装其它部件，则可以单独引入这个特质来安装游戏
pub trait GameDownload:
    FabricDownloadExt + ForgeDownloadExt + VanillaDownloadExt + QuiltMCDownloadExt
{
    /// 根据参数安装一个游戏，允许安装模组加载器
    #[allow(clippy::too_many_arguments)]
    fn download_game(
        &self,
        version_name: &str,
        vanilla: VersionInfo,
        fabric: &str,
        quiltmc: &str,
        forge: &str,
        neoforge: &str,
        optifine: &str,
    ) -> DynResult;
}

impl<R: Reporter> GameDownload for Downloader<R> {
    fn download_game(
        &self,
        version_name: &str,
        vanilla: VersionInfo,
        fabric: &str,
        quiltmc: &str,
        forge: &str,
        neoforge: &str,
        optifine: &str,
    ) -> DynResult {
        self.reporter
            .set_message(format!("正在下载游戏 {version_name}"));

        let launcher_profiles_path =
            std::path::Path::new(&self.minecraft_path).join("launcher_profiles.json");

        if !launcher_profiles_path.exists() {
            fs_err::create_dir_all(launcher_profiles_path.parent().unwrap())?;
            fs_err::write(
                launcher_profiles_path,
                r#"{"profiles":{},"selectedProfile":null,"authenticationDatabase":{},"selectedUser":{"account":"00000111112222233333444445555566","profile":"66666555554444433333222221111100"}}"#,
            )?;
        }

        if !fabric.is_empty() {
            let (vanilla_result, fabric_result) = rayon::join(
                || self.install_vanilla(version_name, &vanilla),
                || self.download_fabric_pre(version_name, &vanilla.id, fabric),
            );
            vanilla_result?;
            fabric_result?;
            self.download_fabric_post(version_name)?;
        } else if !quiltmc.is_empty() {
            let (vanilla_result, quiltmc_result) = rayon::join(
                || self.install_vanilla(version_name, &vanilla),
                || self.download_quiltmc_pre(version_name, &vanilla.id, quiltmc),
            );
            vanilla_result?;
            quiltmc_result?;
            self.download_quiltmc_post(version_name)?;
        } else if !forge.is_empty() {
            self.install_vanilla(&vanilla.id, &vanilla)?; // Forge 安装需要原版，如果安装器没有解析到则会从官方源下载，速度很慢
            let (vanilla_result, forge_result) = rayon::join(
                || self.install_vanilla(version_name, &vanilla),
                || self.install_forge_pre(version_name, &vanilla.id, forge),
            );
            vanilla_result?;
            forge_result?;
            self.install_forge_post(version_name, &vanilla.id, forge)?;
        } else if !neoforge.is_empty() {
            self.install_vanilla(&vanilla.id, &vanilla)?; // NeoForge 安装需要原版，如果安装器没有解析到则会从官方源下载，速度很慢
            let (vanilla_result, neoforge_result) = rayon::join(
                || self.install_vanilla(version_name, &vanilla),
                || self.install_neoforge_pre(version_name, &vanilla.id, neoforge),
            );
            vanilla_result?;
            neoforge_result?;
            self.install_neoforge_post(version_name, &vanilla.id, neoforge)?;
        } else {
            self.install_vanilla(version_name, &vanilla)?;
        }
        if !optifine.is_empty() {
            if forge.is_empty() && fabric.is_empty() {
                self.install_vanilla(&vanilla.id, &vanilla)?; // Optifine 安装需要原版，如果安装器没有解析到则会从官方源下载，速度很慢
            }
            let (optifine_type, optifine_patch) =
                optifine.split_at(optifine.find(' ').context("Optifine 版本字符串不合法！")?);
            self.install_optifine(
                version_name,
                &vanilla.id,
                optifine_type,
                &optifine_patch[1..],
                !forge.is_empty() || !fabric.is_empty(),
            )?;
        }

        // 这俩都需要安装器，而安装后会生成一个新的版本元数据
        // 因此需要最后扫描一遍生成出来的版本元数据依赖，再进行一次下载
        if !optifine.is_empty() || !forge.is_empty() {
            let mut version_info = crate::version::structs::VersionInfo {
                version_base: self.minecraft_version_path.to_owned(),
                version: version_name.to_owned(),
                ..Default::default()
            };

            if version_info
                .load()
                .context("无法读取安装完成后的版本元数据！")
                .is_ok()
            {
                if let Some(meta) = &mut version_info.meta {
                    meta.fix_libraries();
                    self.download_libraries(&meta.libraries)?;
                }
            }
        }
        Ok(())
    }
}
