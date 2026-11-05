//! 模组安装目标版本选择与下载。

use std::path::PathBuf;

use druid::{EventCtx, ExtEventSink, FileInfo, Target};

use crate::{
    app::{AppEvent, APP_EVENT},
    app_state::{AppState, LocalVersionInfo, ModFileState},
    core::VersionType,
    utils::{prelude, thread::thread_main, Progress},
};

/// 根据版本名称获取模组安装目录
async fn mods_folder(
    method: crate::config::VersionIndependentMethod,
    minecraft_folder: PathBuf,
    version_name: &str,
) -> PathBuf {
    let mut info = scl_core::version::structs::VersionInfo {
        version_base: minecraft_folder
            .join("versions")
            .to_string_lossy()
            .to_string(),
        version: version_name.to_owned(),
        ..Default::default()
    };
    let independent = if info.load().await.is_ok() {
        match info.scl_launch_config.as_ref() {
            Some(scl) => scl.game_independent,
            None => prelude::should_version_independent(
                method,
                &info.minecraft_version,
                VersionType::from(info.version_type),
            ),
        }
    } else {
        // 元数据加载失败时无法判定隔离，按官方语义退回全局 mods 目录
        false
    };
    if independent {
        minecraft_folder
            .join("versions")
            .join(version_name)
            .join("mods")
    } else {
        minecraft_folder.join("mods")
    }
}

/// 过滤可安装模组的版本列表
pub fn filter_versions(data: &AppState) -> Vec<LocalVersionInfo> {
    let game_versions = data
        .mod_file_downloads_selected
        .as_ref()
        .map(|file| file.game_versions.clone())
        .unwrap_or_default();
    let has_forge = game_versions.iter().any(|x| x == "forge" || x == "Forge");
    let has_fabric = game_versions.iter().any(|x| x == "fabric" || x == "Fabric");
    data.versions
        .iter()
        .filter(|x| match x.version_type {
            VersionType::Fabric => has_fabric || !has_forge,
            VersionType::Forge => has_forge || !has_fabric,
            _ => !has_forge && !has_fabric,
        })
        .cloned()
        .collect()
}

/// 选择一个安装目标版本
pub fn select_version(data: &mut AppState, version: &str) {
    data.mod_download_install_version = version.to_owned();
}

/// 获取当前选中的待安装模组文件
fn selected_file(data: &AppState) -> Option<ModFileState> {
    data.mod_file_downloads_selected.clone().or_else(|| {
        data.mod_file_downloads_files
            .as_ref()
            .and_then(|files| files.front().cloned())
    })
}

fn download_to(ctx: &EventCtx, data: &mut AppState, file: ModFileState, dest: PathBuf) {
    let sink = ctx.get_external_handle();
    data.starting_tip = Some(format!("正在下载模组 {}", file.file_name));
    thread_main(
        async move { download_file(&sink, &file.file_name, &file.download_url, dest).await },
    );
}

/// 下载一个模组文件到指定目录，完成后通过 [`APP_EVENT`] 通知界面。
///
/// 失败时错误向上抛给 [`thread_main`]，由异步错误弹窗展示
/// （官方 install 任务经 do_async_ok_inner，Err 同样走“执行异步任务发生错误”弹窗，
/// 且页面在点击版本行时已经 QUERY_POP_PAGE 弹回，不等待下载完成）。
async fn download_file(
    sink: &ExtEventSink,
    file_name: &str,
    download_url: &str,
    dest: PathBuf,
) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        smol::fs::create_dir_all(parent).await?;
    }
    scl_core::download::curseforge::download_mod(
        Some(Progress::new(sink.clone())),
        file_name,
        download_url,
        dest,
    )
    .await?;
    let _ = sink.submit_command(
        APP_EVENT,
        AppEvent::ModDownloaded(Ok(String::new())),
        Target::Global,
    );
    Ok(())
}

/// 把当前选中的模组文件安装到指定版本
pub fn install(ctx: &EventCtx, data: &mut AppState, version: &str) {
    data.mod_download_install_version = version.to_owned();
    let Some(file) = selected_file(data) else {
        return;
    };
    let sink = ctx.get_external_handle();
    let method = data.config.version_independent_method;
    let minecraft_folder = data.minecraft_folder();
    let version_name = version.to_owned();
    data.starting_tip = Some(format!("安装模组 {} 到 {}", file.file_name, version_name));
    thread_main(async move {
        let folder = mods_folder(method, minecraft_folder, &version_name).await;
        download_file(
            &sink,
            &file.file_name,
            &file.download_url,
            folder.join(&file.file_name),
        )
        .await
    });
}

/// 把当前选中的模组文件保存到指定路径
pub fn save_mod(ctx: &EventCtx, data: &mut AppState, file_info: &FileInfo) {
    let Some(file) = selected_file(data) else {
        return;
    };
    let dest = file_info.path().to_path_buf();
    download_to(ctx, data, file, dest);
}
