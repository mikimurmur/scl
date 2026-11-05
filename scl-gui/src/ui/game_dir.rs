//! 游戏文件夹管理页面。

use druid::{
    widget::{CrossAxisAlignment, Flex, List, Scroll},
    Env, FileDialogOptions, Lens, Target, Widget, WidgetExt,
};
use im::Vector;
use scl_gui_widgets::widgets::DownloadModuleItem;

use crate::{
    app::{ADD_MC_FOLDER, SEL_MC_FOLDER},
    app_state::AppState,
    config::MinecraftFolder,
};

/// 文件夹列表项的数据：(是否当前选中, 索引, 文件夹)
pub type FolderItemData = (bool, usize, MinecraftFolder);

/// 根据当前配置生成文件夹列表
pub struct FoldersLens;

impl Lens<AppState, Vector<FolderItemData>> for FoldersLens {
    fn with<V, F: FnOnce(&Vector<FolderItemData>) -> V>(&self, data: &AppState, f: F) -> V {
        let current = &data.config.minecraft_folder;
        let mut folders: Vector<FolderItemData> = im::vector![
            (
                current == &MinecraftFolder::CurrentDirectory,
                0,
                MinecraftFolder::CurrentDirectory,
            ),
            (
                current == &MinecraftFolder::OfficalDirectory,
                1,
                MinecraftFolder::OfficalDirectory,
            ),
        ];
        // minecraft_folders 是纯路径字符串数组（官方形态），展示时包装为 Other
        folders.extend(
            data.config
                .minecraft_folders
                .iter()
                .enumerate()
                .map(|(index, path)| {
                    let folder = MinecraftFolder::Other(path.clone());
                    (folder == *current, index + 2, folder)
                }),
        );
        f(&folders)
    }

    fn with_mut<V, F: FnOnce(&mut Vector<FolderItemData>) -> V>(
        &self,
        data: &mut AppState,
        f: F,
    ) -> V {
        let current = data.config.minecraft_folder.clone();
        let mut folders: Vector<FolderItemData> = im::vector![
            (
                current == MinecraftFolder::CurrentDirectory,
                0,
                MinecraftFolder::CurrentDirectory,
            ),
            (
                current == MinecraftFolder::OfficalDirectory,
                1,
                MinecraftFolder::OfficalDirectory,
            ),
        ];
        folders.extend(
            data.config
                .minecraft_folders
                .iter()
                .enumerate()
                .map(|(index, path)| {
                    let folder = MinecraftFolder::Other(path.clone());
                    (folder == current, index + 2, folder)
                }),
        );
        let result = f(&mut folders);
        // 自定义条目（index >= 2）按构造只会是 Other，还原成路径字符串保存
        data.config.minecraft_folders = folders
            .into_iter()
            .filter(|(_, index, _)| *index >= 2)
            .filter_map(|(_, _, folder)| {
                if let MinecraftFolder::Other(path) = folder {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();
        result
    }
}

fn folder_title(data: &FolderItemData, _: &Env) -> String {
    match &data.2 {
        MinecraftFolder::CurrentDirectory => "当前游戏文件夹".to_owned(),
        MinecraftFolder::OfficalDirectory => "官方游戏文件夹".to_owned(),
        MinecraftFolder::Other(path) => path.clone(),
    }
}

fn folder_checked(data: &FolderItemData, _: &Env) -> String {
    if data.0 {
        "●".to_owned()
    } else {
        String::new()
    }
}

fn folder_row() -> impl Widget<FolderItemData> {
    DownloadModuleItem::dynamic(crate::theme::icons::FOLDER, folder_title, folder_checked).on_click(
        |ctx, data, _env| {
            ctx.submit_command(SEL_MC_FOLDER.with(data.1).to(Target::Global));
        },
    )
}

/// 创建游戏文件夹管理页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let list = List::new(folder_row).lens(FoldersLens).expand_width();

    let add_row = DownloadModuleItem::new(crate::theme::icons::ADD, "添加新的文件夹").on_click(
        |ctx, _data, _env| {
            // SelectFolder 面板不传 default_name：GTK 的
            // `gtk_file_chooser_set_current_name` 只接受 SAVE/CREATE_FOLDER，
            // druid 的 GTK 后端会在所有面板上无条件调用，导致断言 critical。
            ctx.submit_command(
                druid::commands::SHOW_OPEN_PANEL.with(
                    FileDialogOptions::new()
                        .select_directories()
                        .title("请选择一个 .minecraft 游戏文件夹")
                        .accept_command(ADD_MC_FOLDER),
                ),
            );
        },
    );

    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_flex_child(Scroll::new(list).vertical().expand(), 1.0)
            .with_child(add_row)
            .expand(),
    )
}
