//! Fabric 下载源数据结构
use serde::Deserialize;

use super::{run_parallel_tasks, DownloadSource, Downloader};
use crate::{package::PackageName, prelude::*, version::structs::VersionMeta};

/// Fabric 加载器的版本元数据和其源码对照表的版本元数据
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct LoaderMetaItem {
    /// Fabric 加载器的元数据
    pub loader: LoaderStruct,
    /// 源码对照表的元数据
    pub intermediary: IntermediaryStruct,
}

/// 源码对照表的信息
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct IntermediaryStruct {
    /// 源码对照表对应的 Maven 仓库文件链接
    pub maven: String,
    /// 源码对照表的版本
    pub version: String,
    /// 是否是稳定版本
    pub stable: bool,
}

/// Fabric 加载器的信息
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct LoaderStruct {
    /// Fabric 加载器对应的 Maven 仓库文件链接
    pub maven: String,
    /// Fabric 加载器的版本
    pub version: String,
    /// 是否是稳定版本
    pub stable: bool,
}

/// Fabric 模组加载器的安装特质
///
/// 可以通过引入本特质和使用 [`crate::download::Downloader`] 来安装模组加载器
pub trait FabricDownloadExt: Sync {
    /// 根据原版版本号获取该版本下可用的 Fabric 模组加载器
    fn get_avaliable_loaders(&self, vanilla_version: &str) -> DynResult<Vec<LoaderMetaItem>>;
    // http://resources.download.minecraft.net
    /// 下载 Fabric 模组加载器所需的库文件
    fn download_library(&self, name: &str) -> DynResult;
    /// 下载 Fabric 模组加载器，安装时需要配合 download_fabric_post 来合并版本元数据
    fn download_fabric_pre(
        &self,
        version_name: &str,
        version_id: &str,
        loader_version: &str,
    ) -> DynResult;
    /// 将 Fabric 模组加载器的版本元数据与原版版本元数据合并以完成最后安装步骤
    fn download_fabric_post(&self, version_name: &str) -> DynResult;
}

impl<R: Reporter> FabricDownloadExt for Downloader<R> {
    fn get_avaliable_loaders(&self, vanilla_version: &str) -> DynResult<Vec<LoaderMetaItem>> {
        let response = crate::http::retry_get(match self.source {
            DownloadSource::Default => {
                format!("https://meta.fabricmc.net/v2/versions/loader/{vanilla_version}")
            }
            DownloadSource::BMCLAPI => format!(
                "https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader/{vanilla_version}"
            ),
            _ => format!("https://meta.fabricmc.net/v2/versions/loader/{vanilla_version}"),
        })
        .map_err(|e| {
            anyhow::anyhow!(
                "获取为原版 {} 可用的 Fabric Loader 版本失败 {:?}",
                vanilla_version,
                e
            )
        })?;
        if response.is_success() {
            response.into_json().map_err(|e| anyhow::anyhow!(e))
        } else {
            Ok(vec![])
        }
    }

    fn download_library(&self, name: &str) -> DynResult {
        let package_name = name.parse::<PackageName>().unwrap();
        let full_path = package_name.to_maven_jar_path(self.minecraft_library_path.as_str());
        let r = self.reporter.sub();
        fs_err::create_dir_all(&full_path[..full_path.rfind('/').unwrap_or(full_path.len())])
            .unwrap_or_default();
        r.set_message(format!("正在下载 Fabric 支持库 {name}"));
        r.add_max_progress(1.);
        if std::path::Path::new(&full_path).is_file() {
            if self.verify_data {
                let mut file = fs_err::File::open(&full_path)?;
                r.set_message(format!("正在获取数据摘要以验证完整性 {name}"));
                r.add_max_progress(1.);
                let sha1 = crate::http::retry_get_string(format!(
                    "{}.sha1",
                    package_name.to_maven_jar_path(match self.source {
                        DownloadSource::BMCLAPI => "https://bmclapi2.bangbang93.com/maven",
                        _ => "https://maven.fabricmc.net",
                    })
                ))?;
                let current_sha1 = crate::utils::get_data_sha1(&mut file)?;
                if sha1 == current_sha1 {
                    r.add_progress(1.);
                    return Ok(());
                }
            } else {
                r.add_progress(1.);
                return Ok(());
            }
        }
        let uris = [
            package_name.to_maven_jar_path(match self.source {
                DownloadSource::BMCLAPI => "https://bmclapi2.bangbang93.com/maven",
                _ => "https://maven.fabricmc.net",
            }),
            package_name.to_maven_jar_path("https://bmclapi2.bangbang93.com/maven"),
            package_name.to_maven_jar_path("https://maven.fabricmc.net"),
        ];
        crate::http::download(&uris, &full_path, 0)
            .map_err(|e| anyhow::anyhow!("下载 Fabric 依赖库失败：{:?}", e))?;
        Ok(())
    }

    fn download_fabric_pre(
        &self,
        version_name: &str,
        version_id: &str,
        loader_version: &str,
    ) -> DynResult {
        let response = crate::http::retry_get(format!(
            "https://meta.fabricmc.net/v2/versions/loader/{version_id}/{loader_version}/profile/json"
        ))
        .map_err(|e| anyhow::anyhow!("获取 Fabric 版本元数据失败：{:?}", e))?;
        let res = response.into_bytes().map_err(|e| anyhow::anyhow!(e))?;
        fs_err::write(
            format!(
                "{}/{}/{}-fabric-loader.tmp.json",
                self.minecraft_version_path.as_str(),
                version_name,
                version_name
            ),
            &res,
        )?;
        let meta: VersionMeta = serde_json::from_slice(&res)?;

        let library_names: Vec<String> = meta
            .libraries
            .iter()
            .filter(|lib| !lib.name.is_empty())
            .map(|lib| lib.name.to_owned())
            .collect();
        run_parallel_tasks(self.parallel_amount, &library_names, |name| {
            self.download_library(name)
        })?;
        // net.fabricmc:sponge-mixin:0.9.2+mixin.0.8.2
        // https://maven.fabricmc.net/net/fabricmc/sponge-mixin/0.9.2+mixin.0.8.2/sponge-mixin-0.9.2+mixin.0.8.2.jar

        Ok(())
    }

    fn download_fabric_post(&self, version_name: &str) -> DynResult {
        // 将元数据与加载器的元数据进行合并

        let vanilla_path = format!(
            "{}/{}/{}.json",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        let vanilla_meta = fs_err::read(&vanilla_path)?;
        let loader_path = format!(
            "{}/{}/{}-fabric-loader.tmp.json",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        let loader_meta = fs_err::read(&loader_path)?;
        fs_err::remove_file(loader_path)?;

        let mut vanilla_meta: VersionMeta = serde_json::from_slice(&vanilla_meta)?;
        let loader_meta: VersionMeta = serde_json::from_slice(&loader_meta)?;

        vanilla_meta += loader_meta;
        fs_err::write(&vanilla_path, serde_json::to_vec(&vanilla_meta)?)?;

        Ok(())
    }
}
