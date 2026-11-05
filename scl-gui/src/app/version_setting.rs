//! 版本设置页面的后台逻辑。

use druid::{EventCtx, Target};
use scl_core::version::{
    mods::Mod, structs::VersionInfo as CoreVersionInfo, VersionType as CoreVersionType,
};

use crate::{
    app::{AppEvent, APP_EVENT},
    app_state::{AppState, LocalMod},
    core::{CoreTransformer, SCLLaunchConfig, VersionType},
    ui::{PAGE_MODS_DOWNLOAD, PAGE_VERSION},
    utils::{fix_minecraft::fix_minecraft_version, progress::Progress, thread::thread_main},
};

/// 把版本类型转换为界面使用的数字
pub fn version_type_index(version_type: CoreVersionType) -> u8 {
    match version_type {
        CoreVersionType::Vanilla => 0,
        CoreVersionType::Forge => 1,
        CoreVersionType::NeoForge => 2,
        CoreVersionType::Fabric => 3,
        CoreVersionType::QuiltMC => 4,
        CoreVersionType::Optifine => 5,
        CoreVersionType::Unknown => 6,
    }
}

/// 纯函数：缓存条目属于同一版本且文件未被改动时命中。
pub fn cache_entry_matches(
    entry_version: &str,
    entry_epoch: u64,
    current_epoch: u64,
    version_name: &str,
) -> bool {
    entry_version == version_name && entry_epoch == current_epoch
}

/// 纯函数：判断迟到的后台回调是否仍属于当前打开的版本设置页。
///
/// 退出页面或切换到另一个版本时 token 会递增，事件里的版本名与 token
/// 任一不匹配即丢弃，避免把 A 版本的模组/计数写进 B 版本。
pub fn accepts_load(current_name: &str, current_token: u64, name: &str, token: u64) -> bool {
    !current_name.is_empty() && current_name == name && current_token == token
}

/// 从模组元数据中取出图标在 jar 内的路径（与 `Mod::try_get_mod_icon` 同判据）。
fn mod_icon_path(meta: &scl_core::version::mods::ModMeta) -> Option<String> {
    use scl_core::version::mods::{FabricModIcon, ModMeta};
    match meta {
        ModMeta::Fabric(meta) => match meta.icon.as_ref()? {
            FabricModIcon::Multiply(icon) => icon.iter().next().map(|x| x.0.to_owned()),
            FabricModIcon::Single(icon) => Some(icon.clone()),
        },
        ModMeta::Forge(meta) => Some(meta.logo_file.clone()),
    }
}

/// 直接读取并解码 jar 内图标，避免再次解析一遍元数据。
fn read_mod_icon(path: &std::path::Path, icon: &str) -> Option<image::DynamicImage> {
    use std::io::Read;
    let file = fs_err::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let mut entry = zip.by_name(icon.trim_start_matches(['/', '\\'])).ok()?;
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf).ok()?;
    image::load_from_memory(&buf).ok()
}

/// 统计目录下的条目数量；目录不存在或不可读时返回 `None`。
///
/// 官方把「读取失败」与「读取成功但为 0」区分开：前者显示「无」，
/// 后者显示「已游玩 0」（实测空 saves 目录 vs 无 saves 目录）。
fn count_dir_entries(path: &std::path::Path) -> Option<usize> {
    fs_err::read_dir(path)
        .ok()
        .map(|dir| dir.filter_map(|entry| entry.ok()).count())
}

/// 读取模组列表（含图标解码），与三条目录计数相互独立。
fn read_mods(info: &CoreVersionInfo) -> anyhow::Result<Vec<LocalMod>> {
    read_mods_with(info, true)
}

/// `with_icons=false` 时跳过图标解码：版本设置页只显示三行描述，
/// 模组列表的图标在进入模组管理页时再加载（官方模组行 586ms、图标不阻塞描述）。
///
/// 返回 `Err` 表示 mods 目录读取失败（官方此时显示「无」而不是「已安装 0」）。
fn read_mods_with(info: &CoreVersionInfo, with_icons: bool) -> anyhow::Result<Vec<LocalMod>> {
    let mods = info.get_mods()?;
    let total = mods.len();
    if total == 0 {
        return Ok(Vec::new());
    }
    // 每个模组都要多次打开 jar（名称/版本/图标），串行会被磁盘 IO 拖到数秒。
    // 这里用固定 8 个原生线程分块并行，最后按下标还原 `read_dir` 原始顺序。
    let workers = total.min(32);
    let mut indexed: Vec<(usize, LocalMod)> = Vec::with_capacity(total);
    let slots: Vec<std::sync::Mutex<Vec<(usize, LocalMod)>>> = (0..workers)
        .map(|_| std::sync::Mutex::new(Vec::new()))
        .collect();
    std::thread::scope(|scope| {
        for (worker, slot) in slots.iter().enumerate() {
            let chunk: Vec<(usize, Mod)> = mods
                .iter()
                .cloned()
                .enumerate()
                .filter(|(index, _)| index % workers == worker)
                .collect();
            scope.spawn(move || {
                let mut out = Vec::with_capacity(chunk.len());
                for (index, item) in chunk {
                    let meta = item.try_get_mod_meta().ok();
                    // 只解析一次元数据，名称/版本/图标路径都从同一份结果取；
                    // 解析失败时官方直接显示原始文件名（含 `.jar` /
                    // `.jar.disabled`），不做去后缀处理（真实 103 模组对照）。
                    let name = meta
                        .as_ref()
                        .map(|m| m.name().to_owned())
                        .unwrap_or_else(|| item.file_name().to_owned());
                    let version = meta
                        .as_ref()
                        .map(|m| m.version().to_owned())
                        .unwrap_or_default();
                    let icon = if with_icons {
                        meta.as_ref()
                            .and_then(mod_icon_path)
                            .and_then(|icon_path| {
                                read_mod_icon(item.path(), &icon_path).and_then(|image| {
                                    crate::utils::thread::image_buf_from_dynamic(image).ok()
                                })
                            })
                            .unwrap_or_else(druid::ImageBuf::empty)
                    } else {
                        druid::ImageBuf::empty()
                    };
                    out.push((
                        index,
                        LocalMod {
                            name,
                            file_name: item.file_name().to_owned(),
                            version,
                            enabled: item.is_enabled(),
                            icon,
                            path: item.path().to_string_lossy().to_string(),
                        },
                    ));
                }
                *slot.lock().unwrap() = out;
            });
        }
    });
    for slot in slots {
        if let Ok(mut items) = slot.lock() {
            indexed.append(&mut items);
        }
    }
    indexed.sort_by_key(|(index, _)| *index);
    Ok(indexed.into_iter().map(|(_, item)| item).collect())
}

/// 统计地图与资源包数量（两者都是与模组解析无关的纯目录计数）。
fn read_counts(game_dir: &std::path::Path) -> (String, String) {
    // 官方：saves 目录不存在/不可读时显示「无」，存在（即使为空）显示「已游玩 N」。
    let maps_desc = match count_dir_entries(&game_dir.join("saves")) {
        Some(count) => format!("已游玩 {count}"),
        None => "无".to_owned(),
    };
    let resource_dir = if game_dir.join("resourcepacks").exists() {
        game_dir.join("resourcepacks")
    } else {
        game_dir.join("texturepacks")
    };
    // 「已安装」行即使为 0（甚至目录不存在）也显示数字。
    let resource_packs_desc = format!("已安装 {}", count_dir_entries(&resource_dir).unwrap_or(0));
    (maps_desc, resource_packs_desc)
}

/// 没有版本级 `.scl.json` 时，把全局「版本独立方案」计算出的有效值写进
/// `VersionInfo`，让 `version_path()` / `get_mods()` / 目录计数与官方
/// `should_version_independent` 的行为一致。
///
/// 真实复现：GregTech Leisure 版本目录有 103 个模组且无 `.scl.json`，
/// 全局方案为「模组加载器隔离」；不写该有效值时 `version_path()` 会退回
/// 父级 `.minecraft`，读到 0 个模组（官方同场景显示「已安装 103」）。
fn ensure_effective_independence(
    info: &mut CoreVersionInfo,
    method: crate::config::VersionIndependentMethod,
) {
    if info.scl_launch_config.is_none() {
        let game_independent = method.should_apply(
            &info.minecraft_version,
            VersionType::from(info.version_type),
        );
        info.scl_launch_config = Some(scl_core::version::structs::SCLLaunchConfig {
            game_independent,
            ..Default::default()
        });
    }
}

fn load_mods_task(
    ctx: &EventCtx,
    version_name: String,
    version_base: std::path::PathBuf,
    token: u64,
    with_icons: bool,
    cache_epoch: u64,
    independence_method: crate::config::VersionIndependentMethod,
) {
    tracing::debug!("版本设置计数加载开始：{version_name} token={token} icons={with_icons}");
    let sink = ctx.get_external_handle();
    thread_main(move || {
        let mut info = CoreVersionInfo {
            version_base: version_base.to_string_lossy().to_string(),
            version: version_name.clone(),
            ..Default::default()
        };
        let (mods, mods_desc) = if info.load().is_ok() {
            ensure_effective_independence(&mut info, independence_method);
            match read_mods_with(&info, with_icons) {
                Ok(mods) => {
                    let desc = format!("已安装 {}", mods.len());
                    (mods, desc)
                }
                // 官方读取 mods 目录失败（如权限不足）时显示「无」；目录存在但为空
                // 才是「已安装 0」（实测 C-Fail/EACCES → 无，F-Empty → 已安装 0）。
                Err(_) => (Vec::new(), "无".to_owned()),
            }
        } else {
            (Vec::new(), "无".to_owned())
        };
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::VersionModsLoaded {
                version_name,
                token,
                cache_epoch,
                mods,
                mods_desc,
            },
            Target::Global,
        );
        Ok(())
    });
}

/// 打开一个版本的设置页面
///
/// 与官方点击闭包等价：只做 `VersionInfo::load` 读取版本配置，随即发送
/// `VersionConfigLoaded`（处理器推入页面）；模组解析与目录计数由
/// `on_app_push_page` 在页面出现之后后台执行，避免卡住进页。
pub fn on_app_event_command(ctx: &mut EventCtx, data: &mut AppState, name: &str) {
    let version_base = data.versions_folder();
    let independence_method = data.config.version_independent_method;
    // 同版本命中缓存时两条目录计数也直接复用，A→B→A 不再重复读盘。
    let cached_counts = data
        .version_setting_cache_get(name)
        .map(|entry| (entry.maps_desc.clone(), entry.resource_packs_desc.clone()));
    let name = name.to_owned();
    let sink = ctx.get_external_handle();
    thread_main(move || {
        let mut info = CoreVersionInfo {
            version_base: version_base.to_string_lossy().to_string(),
            version: name.to_owned(),
            ..Default::default()
        };
        let result = (|| {
            info.load()?;
            let had_config = info.scl_launch_config.is_some();
            let config = info
                .scl_launch_config
                .as_ref()
                .map(|x| SCLLaunchConfig::from_core(x.to_owned()))
                .unwrap_or_default();
            // 没有独立配置文件时，按全局“版本隔离方式”给出该版本的有效值，
            if !had_config {
                ensure_effective_independence(&mut info, independence_method);
            }
            let version_type = version_type_index(info.version_type);
            // 两条目录计数都在毫秒级；提前算好，首帧即显示官方首帧的
            // 「无 / 已安装 0」，避免占位到最终值切换时闪一帧空白。
            let (maps_desc, resource_packs_desc) = match cached_counts {
                Some(counts) => counts,
                None => read_counts(&info.version_path()),
            };
            anyhow::Ok((config, version_type, maps_desc, resource_packs_desc))
        })();
        match result {
            Ok((config, version_type, maps_desc, resource_packs_desc)) => {
                let _ = sink.submit_command(
                    APP_EVENT,
                    AppEvent::VersionConfigLoaded {
                        name,
                        config,
                        version_type,
                        maps_desc,
                        resource_packs_desc,
                    },
                    Target::Global,
                );
            }
            Err(err) => {
                let _ = sink.submit_command(
                    APP_EVENT,
                    AppEvent::LaunchFailed(format!("读取版本设置失败：{err:?}")),
                    Target::Global,
                );
            }
        }
        Ok(())
    });
}

/// 页面打开后加载模组列表与菜单行计数
pub fn on_app_push_page(ctx: &mut EventCtx, data: &mut AppState) {
    let name = data.version_setting_version_name.to_owned();
    if name.is_empty() {
        return;
    }
    let version_base = data.versions_folder();
    // 同版本、且期间没有改过版本目录时直接复用已加载的数据：
    // 从「高级设置」等子页弹栈回来、或 A→B→A 切回，都不再重新读盘。
    if let Some(entry) = data.version_setting_cache_get(&name).cloned() {
        tracing::debug!("版本 {name} 的三行数据命中缓存，跳过重载");
        data.version_mods = entry.mods;
        data.rebuild_mods_filter();
        data.version_setting_mods_desc = entry.mods_desc;
        data.version_setting_maps_desc = entry.maps_desc;
        data.version_setting_resource_packs_desc = entry.resource_packs_desc;
        return;
    }
    // 同一批请求已经启动（推页时提前启动、弹栈动画结束后再次触发）时不重复读盘。
    if data.version_setting_loading_for == name
        && data.version_setting_loading_token == data.version_setting_load_token
    {
        tracing::debug!("版本 {name} 的计数加载仍在进行，跳过重复请求");
        return;
    }
    // 每次进页/切版本都换一个新 token，旧的回调即使后到也会被丢弃。
    let token = data.version_setting_load_token.wrapping_add(1);
    data.version_setting_load_token = token;
    data.version_setting_loading_for = name.clone();
    data.version_setting_loading_token = token;
    // 模组行在解析完成前显示「读取中」。
    data.version_setting_mods_desc = "读取中".to_owned();
    let cache_epoch = data.version_setting_cache_epoch;
    let independence_method = data.config.version_independent_method;
    load_mods_task(
        ctx,
        name,
        version_base,
        token,
        false,
        cache_epoch,
        independence_method,
    );
}

/// 重新加载当前版本的模组列表（进入模组管理页时连图标一起加载）
pub fn load_mods(ctx: &mut EventCtx, data: &mut AppState) {
    let name = data.version_setting_version_name.to_owned();
    if name.is_empty() {
        return;
    }
    let version_base = data.versions_folder();
    let token = data.version_setting_load_token.wrapping_add(1);
    data.version_setting_load_token = token;
    data.version_setting_loading_for = name.clone();
    data.version_setting_loading_token = token;
    let cache_epoch = data.version_setting_cache_epoch;
    let independence_method = data.config.version_independent_method;
    load_mods_task(
        ctx,
        name,
        version_base,
        token,
        true,
        cache_epoch,
        independence_method,
    );
}

/// 目录内容被改动（开关/删除/安装/修复）后让缓存失效。
///
/// 纪元递增后所有旧条目都不再命中；清空列表顺带释放缓存的图标内存。
pub fn invalidate_cache(data: &mut AppState) {
    data.version_setting_cache_epoch = data.version_setting_cache_epoch.wrapping_add(1);
    data.version_setting_cache.clear();
}

/// 决定是否把版本设置写入 `<版本目录>/.scl.json`
///
/// 官方 `.scl.json` 的字段里只有 `game_independent`，没有界面用的 `enable`；
/// `enable` 由“文件是否存在”推断。因此只要用户勾了“使用版本隔离”，
/// 即使没勾“使用独立版本设置”，也必须保存，否则选择会在下次打开时丢失。
fn launch_config_to_save(
    config: &SCLLaunchConfig,
) -> Option<scl_core::version::structs::SCLLaunchConfig> {
    // 「手动调节内存」的数值以输入框文本为准：保存时把文本同步到 max_mem，
    // 避免只改了文本、没触发输入框控制器时丢失（与官方保存链的最终取值一致）。
    let mut config = config.clone();
    if config.override_max_mem {
        if let Ok(value) = config.max_mem_text.trim().parse::<usize>() {
            config.max_mem = Some(value);
        }
    }
    if config.enable || config.game_independent {
        Some(config.into_core())
    } else {
        None
    }
}

/// 把一个版本的设置写回 `<版本目录>/.scl.json`，并按需重命名版本。
///
/// 这是保存链路的纯函数部分，页面弹出保存与高级设置页的「保存设置」共用。
fn persist_version_setting(
    version_base: std::path::PathBuf,
    old_name: String,
    new_name: String,
    config: SCLLaunchConfig,
) -> anyhow::Result<String> {
    let mut info = CoreVersionInfo {
        version_base: version_base.to_string_lossy().to_string(),
        version: old_name.to_owned(),
        ..Default::default()
    };
    info.load()?;
    let mut name = old_name;
    if !new_name.is_empty() && new_name != name {
        info.rename_version(&new_name)?;
        name = new_name;
    }
    info.scl_launch_config = launch_config_to_save(&config);
    info.save()?;
    Ok(name)
}

/// 关闭窗口前同步落盘当前版本设置。
///
/// 官方只在页面弹出时保存，直接关窗会丢修改（DEV-10）；按登记册口径
/// 这里保留修复：关窗时若仍停留在版本设置相关页面，先写版本级 `.scl.json`。
pub fn save_on_close(data: &AppState) {
    let name = data.version_setting_version_name.to_owned();
    if name.is_empty() {
        return;
    }
    let version_dir = data.versions_folder().join(&name);
    if !version_dir.is_dir() {
        return;
    }
    let path = version_dir.join(".scl.json");
    match launch_config_to_save(&data.version_setting_config) {
        Some(config) => match serde_json::to_string(&config) {
            Ok(json) => {
                if let Err(err) = crate::utils::prelude::write_file_safe_sync(&path, json) {
                    tracing::error!("关闭前保存版本设置失败：{err:?}");
                } else {
                    tracing::debug!("关闭前已保存版本设置");
                }
            }
            Err(err) => tracing::error!("关闭前序列化版本设置失败：{err:?}"),
        },
        // 与页面弹出的保存语义一致：两项都未启用时删除版本级配置文件。
        None => {
            if path.is_file() {
                if let Err(err) = fs_err::remove_file(&path) {
                    tracing::error!("关闭前移除版本设置失败：{err:?}");
                }
            }
        }
    }
}

/// 保存版本设置
pub fn save_version(ctx: &mut EventCtx, data: &mut AppState) {
    let old_name = data.version_setting_version_name.to_owned();
    if old_name.is_empty() {
        return;
    }
    let new_name = data.version_setting_version_name_rename.trim().to_owned();
    let version_base = data.versions_folder();
    let config = data.version_setting_config.to_owned();
    let sink = ctx.get_external_handle();
    data.starting_tip = Some("正在保存版本设置……".into());
    thread_main(move || {
        match persist_version_setting(version_base, old_name, new_name, config) {
            Ok(name) => {
                let _ = sink.submit_command(
                    APP_EVENT,
                    AppEvent::DownloadFinished(Ok(name)),
                    Target::Global,
                );
            }
            Err(err) => {
                let _ = sink.submit_command(
                    APP_EVENT,
                    AppEvent::LaunchFailed(format!("保存版本设置失败：{err:?}")),
                    Target::Global,
                );
            }
        }
        Ok(())
    });
}

/// 页面弹出时保存版本设置与重命名结果。
///
/// 官方没有「保存设置」按钮，`app::version_setting::on_app_pop_page` 在页面
/// 弹出时后台把配置写入 `<版本目录>/.scl.json`，并在名称输入框变化时重命名版本。
pub fn on_app_pop_page(ctx: &mut EventCtx, data: &mut AppState) {
    // 退出页面即作废所有在途的模组/计数回调。
    data.version_setting_load_token = data.version_setting_load_token.wrapping_add(1);
    data.version_setting_loading_for.clear();
    let old_name = data.version_setting_version_name.to_owned();
    if old_name.is_empty() {
        return;
    }
    let new_name = data.version_setting_version_name_rename.trim().to_owned();
    let version_base = data.versions_folder();
    let config = data.version_setting_config.to_owned();
    let sink = ctx.get_external_handle();
    thread_main(move || {
        match persist_version_setting(version_base, old_name, new_name, config) {
            Ok(name) => {
                let _ = sink.submit_command(
                    APP_EVENT,
                    AppEvent::VersionSettingSaved(name),
                    Target::Global,
                );
            }
            Err(err) => {
                tracing::error!("保存版本设置失败：{err:?}");
            }
        }
        Ok(())
    });
}

/// 版本级补全/修复：只处理当前打开的版本目录。
pub fn fix_version(ctx: &mut EventCtx, data: &mut AppState) {
    if data.starting_tip.is_some() {
        return;
    }
    let name = data.version_setting_version_name.to_owned();
    if name.is_empty() {
        return;
    }
    data.starting_tip = Some("修复中".into());
    let version_base = data.versions_folder();
    let minecraft_folder = data.minecraft_folder();
    let source = data.config.download_source.clone();
    let parallel_amount = data.config.download_parallel_amount;
    let sink = ctx.get_external_handle();
    thread_main(move || {
        let progress = Progress::new(sink.clone());
        scl_core::progress::Reporter::send(
            &progress,
            scl_core::progress::ReportState::SetMessage("正在修复版本文件".into()),
        );
        let result = (|| {
            // 不再先 `load()` 版本元数据：JSON 损坏正是需要修复的场景，
            // 预加载会在修复入口直接失败，导致「补全/修复版本文件」对损坏
            // 版本毫无动作。修复本身会从客户端 JAR 里恢复 JSON。
            let info = CoreVersionInfo {
                version_base: version_base.to_string_lossy().to_string(),
                version: name,
                ..Default::default()
            };
            // 仅调试构建：用于截图验证「修复中」禁用态的可选延时。
            #[cfg(debug_assertions)]
            if let Some(ms) = std::env::var("SCL_MOCK_FIX_DELAY_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
            {
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
            fix_minecraft_version(
                &minecraft_folder,
                &info,
                source,
                parallel_amount,
                progress.clone(),
            )
        })();
        progress.remove();
        let result = result.map_err(|err| format!("{err:?}"));
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::VersionFixFinished(result),
            Target::Global,
        );
        Ok(())
    });
}

/// 删除当前版本
pub fn remove_version(ctx: &mut EventCtx, data: &mut AppState) {
    let name = data.version_setting_version_name.to_owned();
    if name.is_empty() {
        return;
    }
    let version_base = data.versions_folder();
    let sink = ctx.get_external_handle();
    thread_main(move || {
        let info = CoreVersionInfo {
            version_base: version_base.to_string_lossy().to_string(),
            version: name.to_owned(),
            ..Default::default()
        };
        info.delete();
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::DownloadFinished(Ok(String::new())),
            Target::Global,
        );
        Ok(())
    });
    let _ = PAGE_VERSION;
    let _ = PAGE_MODS_DOWNLOAD;
}

/// 启用/禁用模组
pub fn toggle_mod(ctx: &EventCtx, data: &mut AppState, path: &str) {
    // 开关会改写文件名（.disabled），三行计数缓存随之作废。
    invalidate_cache(data);
    // 每次改写都换 token，连续快速开关时只接受最后一次的回调。
    let token = data.version_setting_load_token.wrapping_add(1);
    data.version_setting_load_token = token;
    let version_name = data.version_setting_version_name.to_owned();
    data.version_setting_loading_for = version_name.clone();
    data.version_setting_loading_token = token;
    let sink = ctx.get_external_handle();
    let path = path.to_owned();
    let cache_epoch = data.version_setting_cache_epoch;
    let independence_method = data.config.version_independent_method;
    let version_base = data.versions_folder();
    thread_main(move || {
        let mut item = Mod::from_path(&path)?;
        if item.is_enabled() {
            item.disable()?;
        } else {
            item.enable()?;
        }
        let mut info = CoreVersionInfo {
            version_base: version_base.to_string_lossy().to_string(),
            version: version_name.clone(),
            ..Default::default()
        };
        let (mods, mods_desc) = if info.load().is_ok() {
            ensure_effective_independence(&mut info, independence_method);
            match read_mods(&info) {
                Ok(mods) => {
                    let desc = format!("已安装 {}", mods.len());
                    (mods, desc)
                }
                Err(_) => (Vec::new(), "无".to_owned()),
            }
        } else {
            (Vec::new(), "无".to_owned())
        };
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::VersionModsLoaded {
                version_name,
                token,
                cache_epoch,
                mods,
                mods_desc,
            },
            Target::Global,
        );
        Ok(())
    });
}

/// 删除模组
pub fn delete_mod(ctx: &EventCtx, data: &mut AppState, path: &str) {
    // 删除会改变模组数量，缓存作废。
    invalidate_cache(data);
    let token = data.version_setting_load_token.wrapping_add(1);
    data.version_setting_load_token = token;
    let version_name = data.version_setting_version_name.to_owned();
    data.version_setting_loading_for = version_name.clone();
    data.version_setting_loading_token = token;
    let sink = ctx.get_external_handle();
    let path = path.to_owned();
    let cache_epoch = data.version_setting_cache_epoch;
    let independence_method = data.config.version_independent_method;
    let version_base = data.versions_folder();
    thread_main(move || {
        let item = Mod::from_path(&path)?;
        item.remove()?;
        let mut info = CoreVersionInfo {
            version_base: version_base.to_string_lossy().to_string(),
            version: version_name.clone(),
            ..Default::default()
        };
        let (mods, mods_desc) = if info.load().is_ok() {
            ensure_effective_independence(&mut info, independence_method);
            match read_mods(&info) {
                Ok(mods) => {
                    let desc = format!("已安装 {}", mods.len());
                    (mods, desc)
                }
                Err(_) => (Vec::new(), "无".to_owned()),
            }
        } else {
            (Vec::new(), "无".to_owned())
        };
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::VersionModsLoaded {
                version_name,
                token,
                cache_epoch,
                mods,
                mods_desc,
            },
            Target::Global,
        );
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 缓存命中要求版本名相同且失效纪元一致；目录被改动（纪元递增）后必须失效。
    #[test]
    fn cache_hit_requires_same_version_and_epoch() {
        assert!(cache_entry_matches("A", 3, 3, "A"));
        assert!(!cache_entry_matches("A", 3, 3, "B")); // 另一个版本
        assert!(!cache_entry_matches("A", 4, 3, "A")); // 目录改动后纪元已递增
    }

    /// 迟到的回调必须被丢弃：版本名或 token 任一不匹配都不应写入界面。
    #[test]
    fn late_version_setting_callback_is_dropped() {
        assert!(accepts_load("A", 7, "A", 7));
        assert!(!accepts_load("B", 8, "A", 7)); // 已切到 B 版本
        assert!(!accepts_load("A", 8, "A", 7)); // 同版本退出后重新进入
        assert!(!accepts_load("", 0, "A", 7)); // 页面已关闭
    }

    #[test]
    fn version_scl_json_round_trips_official_sample_byte_exact() {
        let official_sample = concat!(
            r#"{"max_mem":null,"java_path":"/usr/bin/java","game_independent":true,"#,
            r#""window_title":"","jvm_args":"-Xmx2G","game_args":"","wrapper_path":"","#,
            r#""wrapper_args":"--test"}"#
        );
        let config: scl_core::version::structs::SCLLaunchConfig =
            serde_json::from_str(official_sample).expect("官方版本级 .scl.json 应能读取");
        let json = serde_json::to_string(&config).expect("版本配置应能序列化");
        assert_eq!(
            json, official_sample,
            "版本级 .scl.json 的键序与紧凑格式应与官方逐字节一致"
        );

        // 全默认形态（等价于本地真实写盘的产物）：max_mem 为 null、空串照写。
        let default_sample = concat!(
            r#"{"max_mem":null,"java_path":"","game_independent":false,"#,
            r#""window_title":"","jvm_args":"","game_args":"","wrapper_path":"","#,
            r#""wrapper_args":""}"#
        );
        let config: scl_core::version::structs::SCLLaunchConfig =
            serde_json::from_str(default_sample).expect("默认形态应能读取");
        assert_eq!(
            serde_json::to_string(&config).expect("序列化"),
            default_sample,
            "默认形态也应逐字节往返"
        );

        // max_mem 为 Some 时写出数字
        let with_mem = r#"{"max_mem":4096,"java_path":"","game_independent":true,"window_title":"","jvm_args":"","game_args":"","wrapper_path":"","wrapper_args":""}"#;
        let config: scl_core::version::structs::SCLLaunchConfig =
            serde_json::from_str(with_mem).expect("带内存值应能读取");
        assert_eq!(serde_json::to_string(&config).unwrap(), with_mem);
    }

    #[test]
    fn only_game_independent_is_saved_and_kept_after_reopen() {
        // UI 只勾“使用版本隔离”、没勾“使用独立版本设置”时也必须落盘。
        let config = SCLLaunchConfig {
            enable: false,
            game_independent: true,
            ..Default::default()
        };
        let saved = launch_config_to_save(&config).expect("只勾隔离时必须保存");
        assert!(saved.game_independent);
        let json = serde_json::to_string(&saved).expect("版本配置应能序列化");
        assert!(json.contains("\"game_independent\":true"), "{json}");
        assert!(!json.contains("\"enable\""), "enable 是界面字段，不应写盘");

        // 模拟关窗落盘 → 重开读取：文件内容必须还能反序列化出隔离值。
        let dir =
            std::env::temp_dir().join(format!("scl_version_setting_test_{}", std::process::id()));
        fs_err::create_dir_all(&dir).expect("创建临时目录");
        let path = dir.join(".scl.json");
        fs_err::write(&path, &json).expect("写入 .scl.json");
        let disk = fs_err::read_to_string(&path).expect("读取 .scl.json");
        let disk_core: scl_core::version::structs::SCLLaunchConfig =
            serde_json::from_str(&disk).expect("重开时应能反序列化");
        let reopened = SCLLaunchConfig::from_core(disk_core);
        assert!(reopened.enable);
        assert!(reopened.game_independent);
        let _ = fs_err::remove_dir_all(&dir);

        // 启动路径直接读取 core 配置，game_independent 决定游戏目录。
        let info = CoreVersionInfo {
            version_base: "/tmp/mc/versions".into(),
            version: "26.3".into(),
            scl_launch_config: Some(saved),
            ..Default::default()
        };
        assert_eq!(
            info.version_path(),
            std::path::PathBuf::from("/tmp/mc/versions/26.3")
        );
    }

    #[test]
    fn isolation_only_save_survives_reopen_and_uses_version_dir() {
        // 端到端（文件级）：模拟版本设置页“只勾使用版本隔离”后弹出保存，
        // 再重新 load 该版本，确认 game_independent 仍在、没有写 enable 字段。
        let dir =
            std::env::temp_dir().join(format!("scl_version_setting_pop_{}", std::process::id()));
        let _ = fs_err::remove_dir_all(&dir);
        let versions = dir.join("versions");
        let version_dir = versions.join("26.3");
        fs_err::create_dir_all(&version_dir).expect("创建版本目录");
        fs_err::write(
            version_dir.join("26.3.json"),
            r#"{"mainClass":"net.minecraft.client.main.Main"}"#,
        )
        .expect("写入版本元数据");

        let config = SCLLaunchConfig {
            enable: true,
            game_independent: true,
            ..Default::default()
        };
        let saved_name =
            persist_version_setting(versions.clone(), "26.3".into(), "26.3".into(), config)
                .expect("保存版本设置");
        assert_eq!(saved_name, "26.3");

        let json = fs_err::read_to_string(version_dir.join(".scl.json")).expect("读取 .scl.json");
        assert!(json.contains("\"game_independent\":true"), "{json}");
        assert!(!json.contains("\"enable\""), "enable 是界面字段，不应写盘");

        // 重新打开该版本：文件存在即视为启用独立设置，隔离值必须保留。
        let mut reopened = CoreVersionInfo {
            version_base: versions.to_string_lossy().to_string(),
            version: "26.3".into(),
            ..Default::default()
        };
        reopened.load().expect("重新加载版本");
        let disk =
            SCLLaunchConfig::from_core(reopened.scl_launch_config.clone().expect("应存在版本配置"));
        assert!(disk.enable);
        assert!(disk.game_independent);
        assert_eq!(reopened.version_path(), version_dir);

        let _ = fs_err::remove_dir_all(&dir);
    }

    #[test]
    fn no_independent_flags_removes_version_config() {
        let config = SCLLaunchConfig::default();
        assert!(launch_config_to_save(&config).is_none());
    }

    #[test]
    fn enabled_config_keeps_game_independent() {
        let config = SCLLaunchConfig {
            enable: true,
            game_independent: true,
            ..Default::default()
        };
        let saved = launch_config_to_save(&config).expect("启用独立设置时必须保存");
        assert!(saved.game_independent);
    }
}
