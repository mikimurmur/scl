//! 获取 authlib-injector 第三方登录代理 jar

use std::time::Duration;

use super::{timeout_after, DownloadSource, Downloader};
use crate::prelude::*;

/// Authlib-Injector 元数据请求的兜底超时
const META_TIMEOUT: Duration = Duration::from_secs(20);
/// Authlib-Injector 文件下载的兜底超时
const FILE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Deserialize)]
struct LatestData {
    pub version: String,
    pub download_url: String,
}

/// Authlib 第三方正版登录模块的下载特质
///
/// 你可以通过引入本特质和 [`crate::download::Downloader`] 来下载并安装 Authlib Injector
pub trait AuthlibDownloadExt: Sync {
    /// 下载最新版本的 Authlib Injector 并存放到指定路径，如果路径的文件夹不存在则会先创建它，如果文件已存在则会被覆盖
    async fn download_authlib_injector(&self, dest_path: &str) -> DynResult;
    /// 安装最新版本的 Authlib Injector
    async fn install_authlib_injector(&self) -> DynResult;
}

impl<R: Reporter> AuthlibDownloadExt for Downloader<R> {
    async fn download_authlib_injector(&self, dest_path: &str) -> DynResult {
        // https://authlib-injector.yushi.moe/
        // /artifact/latest.json
        // download_url
        let r = self.reporter.clone();
        r.add_max_progress(2.);
        r.set_message("正在获取 Authlib-Injector 版本元数据".into());
        let meta_uri = match self.source {
            DownloadSource::BMCLAPI => {
                "https://bmclapi2.bangbang93.com/mirrors/authlib-injector/artifact/latest.json"
            }
            _ => "https://authlib-injector.yushi.moe/artifact/latest.json",
        };
        let latest_data: LatestData = match timeout_after(
            META_TIMEOUT,
            crate::http::get(meta_uri).recv_json(),
        )
        .await
        {
            Some(res) => res,
            None => anyhow::bail!("获取 Authlib-Injector 版本元数据超时：{meta_uri}"),
        }
        .map_err(|e| anyhow::anyhow!(e))?;
        r.add_progress(1.);
        r.set_message(format!("正在下载 Authlib-Injector {}", latest_data.version));
        let download_url = latest_data.download_url;
        let resp = match timeout_after(FILE_TIMEOUT, crate::http::get(&download_url)).await {
            Some(res) => res,
            None => anyhow::bail!("下载 Authlib-Injector 超时：{download_url}"),
        }
        .map_err(|e| anyhow::anyhow!(e))?;
        let temp_dest_path = format!("{dest_path}.tmp");
        let f = inner_future::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&temp_dest_path)
            .await?;
        match timeout_after(FILE_TIMEOUT, inner_future::io::copy(resp, f)).await {
            Some(res) => {
                res?;
            }
            None => anyhow::bail!("写入 Authlib-Injector 超时：{temp_dest_path}"),
        }
        r.add_progress(1.);
        inner_future::fs::rename(temp_dest_path, dest_path).await?;
        Ok(())
    }

    async fn install_authlib_injector(&self) -> DynResult {
        let dest_path = format!(
            "{}{sep}authlib-injector.jar",
            self.minecraft_path.as_str(),
            sep = std::path::MAIN_SEPARATOR
        );
        let p = std::path::Path::new(&dest_path);
        if !p.is_file() {
            inner_future::fs::create_dir_all(p.parent().unwrap()).await?;
            self.download_authlib_injector(&dest_path).await?;
        }
        Ok(())
    }
}
