//! CurseForge 模组搜索页面。

use iced::widget::{column, container, space};
use iced::{Element, Length};
use scl_gui_widgets::icons;
use scl_gui_widgets::widgets::ButtonKind;

use crate::{
    app::{CF_APPLY_SEARCH, CF_LOAD_MORE, CF_SELECT_MOD, Message},
    app_state::AppState,
    ui::{TextField, UiMessage, common},
};

fn results_view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let Some(mods) = &state.mods_download_curseforge_result_mods else {
        return common::loading(&theme, state.animation_phase, "正在搜索模组……");
    };

    if mods.is_empty() {
        return common::empty_state(&theme, "没有结果");
    }

    let mut list = column![].spacing(0);
    for item in mods.iter() {
        list = list.push(common::mod_info_row(
            &theme,
            item,
            Some(common::command(CF_SELECT_MOD, item.clone())),
        ));
    }

    list = list.push(
        container(common::button(
            &theme,
            "加载更多",
            ButtonKind::Standard,
            common::command(CF_LOAD_MORE, ()),
        ))
        .padding([10, 4]),
    );

    common::scroll(&theme, list)
}

/// 创建 CurseForge 搜索页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let toolbar = container(
        iced::widget::row![
            scl_gui_widgets::widgets::icon_button(
                &theme,
                icons::SEARCH,
                18.0,
                ButtonKind::Accent,
                common::command(CF_APPLY_SEARCH, ())
            ),
            space().width(Length::Fixed(6.0)),
            container(common::text_input_submit(
                &theme,
                "搜索…",
                &state.mods_download_curseforge_search_filter,
                move |value| Message::Ui(UiMessage::Text(TextField::CurseforgeFilter, value)),
                common::command(CF_APPLY_SEARCH, ())
            ))
            .width(Length::Fill),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([8, 10]);

    column![toolbar, container(results_view(state)).height(Length::Fill),]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
