//! CurseForge 模组搜索和文件获取。

use druid::{EventCtx, ExtEventSink, Target};
use scl_core::download::curseforge::{self, SearchParams, SearchSortMethod};

use crate::{
    app::{AppEvent, APP_EVENT},
    app_state::{AppState, ModFileState, ModSource, ModState},
    utils::thread::thread_main,
};

fn to_state(info: curseforge::ModInfo) -> ModState {
    ModState {
        id: info.id.to_string(),
        name: info.name,
        description: info.summary,
        icon: String::new(),
        source: ModSource::CurseForge,
        download_count: 0,
        slug: info.slug,
        cname: String::new(),
    }
}

/// 后台为每条结果请求中文名（gitee scl-data 的 MCMOD 译名），完成后回传
fn spawn_cname_requests(sink: &ExtEventSink, mods: &[ModState]) {
    for item in mods {
        let key = item.slug.clone();
        let sink = sink.clone();
        thread_main(async move {
            let cname = scl_core::download::mcmod::get_mod_cname(&key).await;
            if !cname.is_empty() {
                let _ = sink.submit_command(
                    APP_EVENT,
                    AppEvent::ModCnameLoaded {
                        source: 0,
                        key,
                        cname,
                    },
                    Target::Global,
                );
            }
            Ok(())
        });
    }
}

fn start_search(ctx: &EventCtx, data: &AppState, page: u64) {
    let sink = ctx.get_external_handle();
    let filter = data
        .mods_download_curseforge_search_filter
        .trim()
        .to_owned();
    let generation = data.mods_download_curseforge_search_generation;
    thread_main(async move {
        let items = curseforge::search_mods(SearchParams {
            game_version: String::new(),
            index: page.max(1),
            page_size: 20,
            category_id: 0,
            search_filter: filter,
            sort: SearchSortMethod::Featured,
        })
        .await?;
        // 搜索失败（网络/JSON 解析）时弹出「执行异步任务发生错误」窗，
        // 不向界面提交命令；成功才把结果送回界面，图标与中文名再逐个异步补上。
        let mut mods = Vec::with_capacity(items.len());
        let mut icons = Vec::with_capacity(items.len());
        for item in items {
            let id = item.id.to_string();
            let icon_url = item.logo.as_ref().map(|logo| logo.thumbnail_url.to_owned());
            if let Some(url) = icon_url {
                icons.push((id.clone(), url));
            }
            mods.push(to_state(item));
        }
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::CurseforgeLoaded {
                mods: mods.clone(),
                page,
                generation,
            },
            Target::Global,
        );
        spawn_cname_requests(&sink, &mods);
        for (id, url) in icons {
            let sink = sink.clone();
            thread_main(async move {
                if let Ok(image) = curseforge::get_mod_icon_url(&url).await {
                    if let Ok(icon) = crate::utils::image::from_image_to_string(&image) {
                        let _ = sink.submit_command(
                            APP_EVENT,
                            AppEvent::ModIconLoaded {
                                source: 0,
                                id,
                                icon,
                            },
                            Target::Global,
                        );
                    }
                }
                Ok(())
            });
        }
        Ok(())
    });
}

/// 应用搜索（从第一页开始）
pub fn apply_search(ctx: &mut EventCtx, data: &mut AppState) {
    data.mods_download_curseforge_page = 1;
    data.mods_download_curseforge_searching = false;
    data.mods_download_curseforge_result_mods = None;
    data.mods_download_curseforge_search_generation =
        data.mods_download_curseforge_search_generation.wrapping_add(1);
    tracing::debug!(
        "CurseForge 应用搜索: filter={:?}, game_version_selected={:?}, game_version_download={:?}",
        data.mods_download_curseforge_search_filter,
        data.selected_version.as_ref().map(|x| x.name.clone()),
        data.selected_download_version
            .as_ref()
            .map(|x| x.name.clone()),
    );
    start_search(ctx, data, 1);
}

/// 加载下一页
pub fn load_more(ctx: &mut EventCtx, data: &mut AppState) {
    if data.mods_download_curseforge_searching {
        return;
    }
    data.mods_download_curseforge_page =
        data.mods_download_curseforge_page.saturating_add(1).max(1);
    data.mods_download_curseforge_searching = true;
    data.mods_download_curseforge_search_generation =
        data.mods_download_curseforge_search_generation.wrapping_add(1);
    start_search(ctx, data, data.mods_download_curseforge_page);
}

/// 页面打开时自动搜索
pub fn on_app_push_page(ctx: &mut EventCtx, data: &mut AppState) {
    data.mods_download_curseforge_search_filter.clear();
    data.mods_download_curseforge_page = 1;
    data.mods_download_curseforge_searching = false;
    data.mods_download_curseforge_result_mods = None;
    apply_search(ctx, data);
}

/// 选择一个模组：立即进入文件页，文件列表后台拉取
pub fn select_mod(ctx: &mut EventCtx, data: &mut AppState, mod_state: &ModState) {
    data.mod_file_downloads_info = Some(mod_state.clone());
    data.mod_file_downloads_files = None;
    data.mod_file_downloads_generation = data.mod_file_downloads_generation.wrapping_add(1);
    let generation = data.mod_file_downloads_generation;
    ctx.submit_command(
        crate::ui::PUSH_PAGE
            .with(crate::ui::PAGE_MOD_FILE_DOWNLOAD)
            .to(Target::Global),
    );
    let sink = ctx.get_external_handle();
    let Ok(id_num) = mod_state.id.parse::<u64>() else {
        return;
    };
    thread_main(async move {
        // 文件请求/解析失败时弹出「执行异步任务发生错误」窗，
        // 文件列表停留空白。
        let versions = curseforge::get_mod_files(id_num).await?;
        let mut files = Vec::with_capacity(versions.len());
        for file in versions {
            files.push(ModFileState {
                file_name: file.file_name,
                download_url: file.download_url,
                game_versions: file.game_versions.into(),
                loaders: im::Vector::new(),
                primary: false,
            });
        }
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::ModFilesLoaded { files, generation },
            Target::Global,
        );
        Ok(())
    });
}
