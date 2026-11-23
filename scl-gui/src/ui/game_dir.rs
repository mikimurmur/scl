//! 游戏文件夹管理页面。

use iced::widget::{column, container};
use iced::{Element, Length};
use scl_gui_widgets::icons;

use crate::{
    app::{Message, SEL_MC_FOLDER},
    app_state::AppState,
    config::MinecraftFolder,
    ui::common,
};

/// 创建游戏文件夹管理页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let current = &state.config.minecraft_folder;

    let mut list = column![].spacing(0);

    let folders = std::iter::once((0, MinecraftFolder::CurrentDirectory, "当前游戏文件夹"))
        .chain(std::iter::once((
            1,
            MinecraftFolder::OfficalDirectory,
            "官方游戏文件夹",
        )))
        .chain(
            state
                .config
                .minecraft_folders
                .iter()
                .enumerate()
                .map(|(index, path)| {
                    (
                        index + 2,
                        MinecraftFolder::Other(path.clone()),
                        path.as_str(),
                    )
                }),
        );

    for (index, folder, title) in folders {
        let selected = *current == folder;
        list = list.push(common::module_row(
            &theme,
            icons::FOLDER,
            title,
            if selected { "●" } else { "" },
            common::command(SEL_MC_FOLDER, index),
        ));
    }

    list = list.push(common::module_row(
        &theme,
        icons::ADD,
        "添加新的文件夹",
        "",
        Message::PickFolder,
    ));

    container(common::scroll(&theme, list))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
