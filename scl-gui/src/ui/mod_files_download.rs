//! 模组文件选择页面。

use iced::widget::{column, container};
use iced::{Element, Length};
use scl_gui_widgets::icons;

use crate::{
    app::{CF_DOWNLOAD_MOD, Message},
    app_state::AppState,
    ui::common,
};

/// 创建模组文件选择页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let info = match &state.mod_file_downloads_info {
        Some(info) => common::mod_info_row(&theme, info, None),
        None => iced::widget::space().into(),
    };

    let files = match &state.mod_file_downloads_files {
        Some(files) => {
            if files.is_empty() {
                common::empty_state(&theme, "没有可用文件")
            } else {
                let mut list = column![].spacing(0);
                for file in files.iter() {
                    list = list.push(common::list_item(
                        &theme,
                        Some(icons::MODS),
                        file.file_name.clone(),
                        None,
                        false,
                        Some(common::command(CF_DOWNLOAD_MOD, file.clone())),
                    ));
                }
                common::scroll(&theme, list)
            }
        }
        None => common::loading(&theme, state.animation_phase, "正在获取文件列表……"),
    };

    column![info, container(files).height(Length::Fill),]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
