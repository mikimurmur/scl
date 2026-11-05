//! QuiltMC 下载源数据结构
use anyhow::Context;
use serde::Deserialize;

use super::{run_parallel_tasks, Downloader};
use crate::{package::PackageName, prelude::*, version::structs::VersionMeta};

/// QuiltMC 加载器的版本元数据和其源码对照表的版本元数据
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct LoaderMetaItem {
    /// QuiltMC 加载器的元数据
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
}

/// QuiltMC 加载器的信息
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct LoaderStruct {
    /// QuiltMC 加载器对应的 Maven 仓库文件链接
    pub maven: String,
    /// QuiltMC 加载器的版本
    pub version: String,
}

/// QuiltMC 模组加载器的安装特质
///
/// 可以通过引入本特质和使用 [`crate::download::Downloader`] 来安装模组加载器
pub trait QuiltMCDownloadExt: Sync {
    /// 根据原版版本号获取该版本下可用的 QuiltMC 模组加载器
    fn get_avaliable_loaders(&self, vanilla_version: &str) -> DynResult<Vec<LoaderMetaItem>>;
    // http://resources.download.minecraft.net
    /// 下载 QuiltMC 模组加载器所需的库文件
    fn download_library(&self, name: &str, url: String) -> DynResult;
    /// 下载 QuiltMC 模组加载器，安装时需要配合 download_quiltmc_post 来合并版本元数据
    fn download_quiltmc_pre(
        &self,
        version_name: &str,
        version_id: &str,
        loader_version: &str,
    ) -> DynResult;
    /// 将 QuiltMC 模组加载器的版本元数据与原版版本元数据合并以完成最后安装步骤
    fn download_quiltmc_post(&self, version_name: &str) -> DynResult;
}

impl<R: Reporter> QuiltMCDownloadExt for Downloader<R> {
    fn get_avaliable_loaders(&self, vanilla_version: &str) -> DynResult<Vec<LoaderMetaItem>> {
        let response = crate::http::retry_get(format!(
            "https://meta.quiltmc.org/v3/versions/loader/{vanilla_version}"
        ))
        .map_err(|e| {
            anyhow::anyhow!(
                "获取为原版 {} 可用的 QuiltMC Loader 版本失败 {:?}",
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

    fn download_library(&self, name: &str, url: String) -> DynResult {
        let url = if url.is_empty() {
            "https://maven.quiltmc.org/repository/release"
        } else {
            url.trim_end_matches('/')
        };
        let package_name = name.parse::<PackageName>().unwrap();
        let full_path = package_name.to_maven_jar_path(self.minecraft_library_path.as_str());
        let r = self.reporter.sub();
        fs_err::create_dir_all(&full_path[..full_path.rfind('/').unwrap_or(full_path.len())])
            .unwrap_or_default();
        r.set_message(format!("正在下载 QuiltMC 支持库 {name}"));
        r.add_max_progress(1.);
        if std::path::Path::new(&full_path).is_file() {
            if self.verify_data {
                let mut file = fs_err::File::open(&full_path)?;
                r.set_message(format!("正在获取数据摘要以验证完整性 {name}"));
                r.add_max_progress(1.);
                let sha1 = crate::http::retry_get_string(format!(
                    "{}.sha1",
                    package_name.to_maven_jar_path(url)
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
        let uris = [package_name.to_maven_jar_path(url)];
        crate::http::download(&uris, &full_path, 0)
            .map_err(|e| anyhow::anyhow!("下载 QuiltMC 依赖库失败：{:?}", e))?;
        Ok(())
    }

    fn download_quiltmc_pre(
        &self,
        version_name: &str,
        version_id: &str,
        loader_version: &str,
    ) -> DynResult {
        let response = crate::http::retry_get(format!(
            "https://meta.quiltmc.org/v3/versions/loader/{version_id}/{loader_version}/profile/json"
        ))
        .map_err(|e| anyhow::anyhow!("获取 QuiltMC 版本元数据失败：{:?}", e))?;
        let res = response.into_bytes().map_err(|e| anyhow::anyhow!(e))?;
        fs_err::write(
            format!(
                "{}/{}/{}-quiltmc-loader.tmp.json",
                self.minecraft_version_path.as_str(),
                version_name,
                version_name
            ),
            &res,
        )?;
        let meta: VersionMeta =
            serde_json::from_slice(&res).context("无法解析 QuiltMC 版本元数据")?;

        let libraries: Vec<(String, String)> = meta
            .libraries
            .iter()
            .filter(|lib| !lib.name.is_empty())
            .map(|lib| {
                (
                    lib.name.to_owned(),
                    lib.url.as_ref().cloned().unwrap_or_default(),
                )
            })
            .collect();
        run_parallel_tasks(self.parallel_amount, &libraries, |(name, url)| {
            self.download_library(name, url.to_owned())
        })?;

        Ok(())
    }

    fn download_quiltmc_post(&self, version_name: &str) -> DynResult {
        // 将元数据与加载器的元数据进行合并
        tracing::trace!("合并元数据中");

        let vanilla_path = format!(
            "{}/{}/{}.json",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        let vanilla_meta = fs_err::read(&vanilla_path)?;
        let loader_path = format!(
            "{}/{}/{}-quiltmc-loader.tmp.json",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        let loader_meta = fs_err::read(&loader_path)?;
        fs_err::remove_file(loader_path)?;

        let mut vanilla_meta: VersionMeta =
            serde_json::from_slice(&vanilla_meta).context("无法解析原版版本元数据")?;
        let loader_meta: VersionMeta =
            serde_json::from_slice(&loader_meta).context("无法解析 QuiltMC 版本元数据")?;

        vanilla_meta += loader_meta;
        fs_err::write(&vanilla_path, serde_json::to_vec(&vanilla_meta)?)?;

        Ok(())
    }
}
