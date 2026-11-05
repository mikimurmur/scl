//! Modrinth 模组搜索和文件获取。

use druid::{EventCtx, ExtEventSink, Target};
use scl_core::download::modrinth::{self, SearchParams};

use crate::{
    app::{AppEvent, APP_EVENT},
    app_state::{AppState, ModFileState, ModSource, ModState},
    utils::thread::thread_main,
};

fn to_state(item: modrinth::ModResult) -> ModState {
    let slug = item.slug;
    ModState {
        id: if slug.is_empty() {
            item.project_id
        } else {
            slug.clone()
        },
        name: item.title,
        description: item.description,
        icon: String::new(),
        source: ModSource::Modrinth,
        download_count: 0,
        slug,
        cname: String::new(),
    }
}

/// 后台为每条结果请求中文名（gitee scl-data 的 MCMOD 译名），完成后回传
fn spawn_cname_requests(sink: &ExtEventSink, mods: &[ModState]) {
    for item in mods {
        let key = item.slug.clone();
        let sink = sink.clone();
        thread_main(move || {
            let cname = scl_core::download::mcmod::get_mod_cname(&key);
            if !cname.is_empty() {
                let _ = sink.submit_command(
                    APP_EVENT,
                    AppEvent::ModCnameLoaded {
                        source: 1,
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
    let filter = data.mods_download_modrinth_search_filter.trim().to_owned();
    let generation = data.mods_download_modrinth_search_generation;
    thread_main(move || {
        // 搜索失败时弹出「执行后台任务发生错误」窗，不向界面提交命令；
        // 成功才把结果送回界面，图标与中文名再逐个后台补上。
        let items = modrinth::search_mods(SearchParams {
            search_filter: filter,
            index: page.max(1),
            page_size: 20,
        })?;
        let mods = items.into_iter().map(to_state).collect::<Vec<_>>();
        let icons = mods
            .iter()
            .map(|x| (x.id.to_owned(), x.name.to_owned()))
            .collect::<Vec<_>>();
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::ModrinthLoaded {
                mods: mods.clone(),
                page,
                generation,
            },
            Target::Global,
        );
        spawn_cname_requests(&sink, &mods);
        for (id, _) in icons {
            let sink = sink.clone();
            let id_for_task = id.to_owned();
            thread_main(move || {
                if let Ok(image) = modrinth::get_mod_icon(&id_for_task) {
                    if let Ok(icon) = crate::utils::image::from_image_to_string(&image) {
                        let _ = sink.submit_command(
                            APP_EVENT,
                            AppEvent::ModIconLoaded {
                                source: 1,
                                id: id_for_task,
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
    data.mods_download_modrinth_page = 1;
    data.mods_download_modrinth_searching = true;
    data.mods_download_modrinth_search_generation = data
        .mods_download_modrinth_search_generation
        .wrapping_add(1);
    start_search(ctx, data, 1);
}

/// 加载下一页
pub fn load_more(ctx: &mut EventCtx, data: &mut AppState) {
    if data.mods_download_modrinth_searching {
        return;
    }
    data.mods_download_modrinth_page = data.mods_download_modrinth_page.saturating_add(1).max(1);
    data.mods_download_modrinth_searching = true;
    data.mods_download_modrinth_search_generation = data
        .mods_download_modrinth_search_generation
        .wrapping_add(1);
    start_search(ctx, data, data.mods_download_modrinth_page);
}

/// 页面打开时自动搜索
pub fn on_app_push_page(ctx: &mut EventCtx, data: &mut AppState) {
    if !data.mods_download_modrinth_search_filter.is_empty() {
        // 程序化清空会触发输入框的「变动即搜索」控制器；抑制该次重复请求，
        // 由下面的 apply_search 唯一发起默认搜索。
        crate::ui::mods_download_modrinth::SKIP_NEXT_INPUT_SEARCH
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
    data.mods_download_modrinth_search_filter.clear();
    data.mods_download_modrinth_page = 1;
    data.mods_download_modrinth_searching = false;
    data.mods_download_modrinth_result_mods = None;
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
    let id = mod_state.id.clone();
    thread_main(move || {
        // 文件请求/解析失败时弹出「执行后台任务发生错误」窗，
        // 文件列表停留空白。
        let versions = modrinth::get_mod_files(&id)?;
        let mut files = Vec::new();
        for version in versions {
            for file in version.files {
                files.push(ModFileState {
                    file_name: file.filename,
                    download_url: file.url,
                    game_versions: version.game_versions.clone().into(),
                    loaders: version.loaders.clone().into(),
                    primary: file.primary,
                });
            }
        }
        files.sort_by_key(|file| std::cmp::Reverse(file.primary));
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::ModFilesLoaded { files, generation },
            Target::Global,
        );
        Ok(())
    });
}
