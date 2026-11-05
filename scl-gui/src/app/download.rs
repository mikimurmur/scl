//! 下载页面的后台逻辑。

use std::str::FromStr;

use druid::{EventCtx, Target};
use scl_core::download::{
    fabric::FabricDownloadExt, forge::ForgeDownloadExt, neoforge::NeoForgeDownloadExt,
    optifine::OptifineDownloadExt, quiltmc::QuiltMCDownloadExt, vanilla::VanillaDownloadExt,
    Downloader, GameDownload,
};

use crate::{
    app::{AppEvent, DownloadRequest, APP_EVENT},
    app_state::{AppState, NetworkData},
    core::CoreTransformer,
    utils::{thread::thread_main_report, Progress},
};

/// 页面打开时加载所需数据
pub fn on_app_push_page(ctx: &mut EventCtx, data: &mut AppState) {
    if data.download_state_vanilla.get().is_none()
        && data.download_state_vanilla.error.is_none()
        && data.download_state_vanilla.loading
    {
        load_vanilla(ctx, data);
    }
}

/// 加载原版游戏版本列表
pub fn load_vanilla(ctx: &mut EventCtx, data: &mut AppState) {
    data.download_state_vanilla = NetworkData::new_loading();
    let sink = ctx.get_external_handle();
    let config = data.config.to_owned();
    let path = data.minecraft_folder();
    thread_main_report(move || {
        // 元数据请求不属于下载队列，原版使用 `Downloader<()>`，不会在界面底部
        // 创建进度条（`Progress::new` 会注册一条空进度并长期占用进度区域）。
        let downloader = Downloader::<()>::default()
            .with_minecraft_path(&path)
            .with_source(config.download_source.to_core())
            .with_parallel_amount(config.download_parallel_amount);
        let result = downloader
            .get_avaliable_vanilla_versions()
            .map(|manifest| {
                manifest
                    .versions
                    .into_iter()
                    .map(VersionInfo::from_core)
                    .collect::<Vec<_>>()
            })
            .map_err(|err| format!("{err:?}"));
        let err = result.as_ref().err().cloned();
        let _ = sink.submit_command(APP_EVENT, AppEvent::VanillaLoaded(result), Target::Global);
        // 原版版本列表请求失败同样弹出后台任务错误窗，页面保持下载页。
        match err {
            Some(err) => Err(err),
            None => Ok(()),
        }
    });
}

use crate::core::VersionInfo;

/// 根据当前分页重新加载加载器版本列表
pub fn reload_current_loader(ctx: &mut EventCtx, data: &mut AppState) {
    // 原版下载页会一次展示全部加载器状态，因此选择原版版本后需要同时查询
    // Fabric / QuiltMC / Forge / NeoForge / Optifine 的可用版本列表。
    let Some(vanilla) = data.selected_download_version.to_owned() else {
        return;
    };
    data.download_state_fabric = NetworkData::new_loading();
    data.download_state_quiltmc = NetworkData::new_loading();
    data.download_state_forge = NetworkData::new_loading();
    data.download_state_neoforge = NetworkData::new_loading();
    data.download_state_optifine = NetworkData::new_loading();
    let sink = ctx.get_external_handle();
    let config = data.config.to_owned();
    let path = data.minecraft_folder();
    let vanilla_name = vanilla.name.to_owned();
    for kind in ["fabric", "quiltmc", "forge", "neoforge", "optifine"] {
        let sink = sink.clone();
        let config = config.clone();
        let path = path.clone();
        let vanilla_name = vanilla_name.clone();
        thread_main_report(move || {
            // 与版本列表元数据请求相同：加载器列表只查询元数据，不创建下载进度。
            let downloader = Downloader::<()>::default()
                .with_minecraft_path(&path)
                .with_source(config.download_source.to_core())
                .with_parallel_amount(config.download_parallel_amount);
            let result = match kind {
                "fabric" => FabricDownloadExt::get_avaliable_loaders(&downloader, &vanilla_name)
                    .map(|list| {
                        list.into_iter()
                            .map(|x| (x.loader.version, !x.loader.stable))
                            .collect()
                    })
                    .map_err(|err| format!("{err:?}")),
                "quiltmc" => QuiltMCDownloadExt::get_avaliable_loaders(&downloader, &vanilla_name)
                    .map(|list| {
                        list.into_iter()
                            .map(|x| (x.loader.version, false))
                            .collect()
                    })
                    .map_err(|err| format!("{err:?}")),
                "forge" => ForgeDownloadExt::get_avaliable_installers(&downloader, &vanilla_name)
                    .map(|data| {
                        data.all_versions
                            .into_iter()
                            .map(|x| (x.version, false))
                            .collect()
                    })
                    .map_err(|err| format!("{err:?}")),
                "neoforge" => {
                    NeoForgeDownloadExt::get_avaliable_installers(&downloader, &vanilla_name)
                        .map(|data| {
                            data.all_versions
                                .into_iter()
                                .map(|x| {
                                    let is_test = x.version.contains("beta");
                                    (x.version, is_test)
                                })
                                .collect()
                        })
                        .map_err(|err| format!("{err:?}"))
                }
                "optifine" => {
                    OptifineDownloadExt::get_avaliable_installers(&downloader, &vanilla_name)
                        .map(|list| {
                            list.into_iter()
                                .map(|x| (format!("{} {}", x.version_type, x.patch), false))
                                .collect()
                        })
                        .map_err(|err| format!("{err:?}"))
                }
                _ => Ok(Vec::new()),
            };
            // 加载器元数据查询失败只记 WARN 并保留“请求失败”行状态，不弹后台任务错误窗。
            // 任务恒返回 Ok，不进入 `thread_main_report` 的错误弹窗分支——否则 GTK
            // 模态窗会遮挡下载页并吞掉点击（页面呈现为整块黑区）。
            let loader_name = match kind {
                "fabric" => Some("Fabric"),
                "quiltmc" => Some("QuiltMC"),
                "forge" => Some("Forge"),
                "neoforge" => Some("NeoForge"),
                _ => None,
            };
            if let (Some(name), Err(err)) = (loader_name, &result) {
                tracing::warn!("查询 {name} 可用版本时发生错误：{err}");
            }
            let _ = sink.submit_command(
                APP_EVENT,
                AppEvent::LoaderLoaded {
                    kind: kind.to_owned(),
                    result,
                },
                Target::Global,
            );
            Ok(())
        });
    }
}

/// 生成安装后的版本名称
pub fn format_version_name(
    vanilla: &str,
    fabric: &str,
    quiltmc: &str,
    forge: &str,
    neoforge: &str,
    optifine: &str,
) -> String {
    if !fabric.is_empty() {
        format!("{vanilla}-fabric-{fabric}")
    } else if !quiltmc.is_empty() {
        format!("{vanilla}-quiltmc-{quiltmc}")
    } else if !forge.is_empty() {
        let forge = forge.strip_prefix(&format!("{vanilla}-")).unwrap_or(forge);
        format!("{vanilla}-forge-{forge}")
    } else if !neoforge.is_empty() {
        let neoforge = neoforge.strip_prefix("neoforge-").unwrap_or(neoforge);
        format!("{vanilla}-neoforge-{neoforge}")
    } else if !optifine.is_empty() {
        let patch = optifine.split_whitespace().nth(1).unwrap_or(optifine);
        format!("{vanilla}-optifine-{patch}")
    } else {
        vanilla.to_owned()
    }
}

/// 切换当前加载器选择，并同步安装后的版本名称
///
/// `version` 为空表示“不安装”。每个加载器保留各自的选中版本，互不覆盖。
pub fn select_loader_version(data: &mut AppState, kind: &str, version: &str) {
    data.set_loader_version(kind, version);
    update_download_version_name(data);
    tracing::debug!(
        "加载器选中版本: forge={:?}, neoforge={:?}, fabric={:?}, quiltmc={:?}, optifine={:?}",
        data.download_forge_version,
        data.download_neoforge_version,
        data.download_fabric_version,
        data.download_quiltmc_version,
        data.download_optifine_version,
    );
}

/// 根据当前选择刷新「游戏版本名称」输入框
pub fn update_download_version_name(data: &mut AppState) {
    let Some(vanilla) = data.selected_download_version.as_ref() else {
        return;
    };
    let fabric = &data.download_fabric_version;
    let quiltmc = &data.download_quiltmc_version;
    let forge = &data.download_forge_version;
    let neoforge = &data.download_neoforge_version;
    let optifine = &data.download_optifine_version;
    data.download_version_name =
        format_version_name(&vanilla.name, fabric, quiltmc, forge, neoforge, optifine);
}

/// 开始下载游戏
pub fn start_download(ctx: &mut EventCtx, data: &mut AppState, request: DownloadRequest) {
    let config = data.config.to_owned();
    let path = data.minecraft_folder();
    let sink = ctx.get_external_handle();
    let version_name = request.version_name.to_owned();
    let version_type = if !request.fabric.is_empty() {
        crate::core::VersionType::Fabric
    } else if !request.quiltmc.is_empty() {
        crate::core::VersionType::QuiltMC
    } else if !request.forge.is_empty() {
        crate::core::VersionType::Forge
    } else if !request.neoforge.is_empty() {
        crate::core::VersionType::NeoForge
    } else if !request.optifine.is_empty() {
        crate::core::VersionType::Optifine
    } else {
        crate::core::VersionType::Vanilla
    };
    let minecraft_version =
        scl_core::semver::MinecraftVersion::from_str(&request.vanilla.name).unwrap_or_default();
    let game_independent = crate::utils::prelude::should_version_independent(
        config.version_independent_method,
        &minecraft_version,
        version_type,
    );
    data.starting_tip = Some(format!("正在下载游戏 {version_name}"));
    thread_main_report(move || {
        let downloader = Downloader::default()
            .with_minecraft_path(&path)
            .with_source(config.download_source.to_core())
            .with_parallel_amount(config.download_parallel_amount)
            .with_reporter(Progress::new(sink.clone()))
            .with_java(
                config
                    .java_for_version(16)
                    .map(|x| x.path.to_owned())
                    .unwrap_or_else(|| "java".into()),
            )
            .with_game_independent(game_independent);
        let result = downloader
            .download_game(
                &version_name,
                request.vanilla.into_core(),
                &request.fabric,
                &request.quiltmc,
                &request.forge,
                &request.neoforge,
                &request.optifine,
            )
            .map(|_| version_name)
            .map_err(|err| format!("{err:?}"));
        let err = result.as_ref().err().cloned();
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::DownloadFinished(result),
            Target::Global,
        );
        // 安装失败在 GUI 侧弹回主页并显示原版后台任务错误窗，不再进入启动失败页。
        match err {
            Some(err) => Err(err),
            None => Ok(()),
        }
    });
}
