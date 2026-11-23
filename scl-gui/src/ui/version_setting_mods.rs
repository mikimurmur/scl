//! 版本模组管理页面。

use iced::widget::{column, container, image as image_widget, row, space};
use iced::{Alignment, Background, Border, Element, Length};
use scl_gui_widgets::icons;
use scl_gui_widgets::theme::Theme;

use crate::{
    app::Message,
    app_state::{AppState, LocalMod},
    ui::{TextField, UiMessage, common},
};

/// 每行图标尺寸，官方为 40×40。
const ICON_SIZE: f32 = 40.0;

fn mod_icon<'a>(theme: &Theme, icon: Option<&iced::widget::image::Handle>) -> Element<'a, Message> {
    let theme = *theme;
    if let Some(handle) = icon {
        return image_widget(handle.clone())
            .width(Length::Fixed(ICON_SIZE))
            .height(Length::Fixed(ICON_SIZE))
            .filter_method(iced::widget::image::FilterMethod::Linear)
            .into();
    }

    container(icons::MODS.view_default(&theme, ICON_SIZE * 0.7))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .center_x(Length::Fixed(ICON_SIZE))
        .center_y(Length::Fixed(ICON_SIZE))
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(theme.base.low)),
            border: Border {
                radius: 4.0.into(),
                ..Border::default()
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

/// 一行模组：图标 + 名称/版本 + 启用开关。
fn mod_row<'a>(theme: &Theme, item: &LocalMod) -> Element<'a, Message> {
    let theme = *theme;
    let path = item.path.clone();

    let text = column![
        iced::widget::text(item.name.clone())
            .size(theme.typography.caption.size)
            .font(theme.typography.caption.font)
            .color(theme.base.high),
        iced::widget::text(item.version.clone())
            .size(theme.typography.caption.size)
            .font(theme.typography.caption.font)
            .color(theme.base.medium),
    ]
    .spacing(1)
    .width(Length::Fill);

    let toggle = common::toggle_switch(&theme, item.enabled, move |enabled| {
        Message::Ui(UiMessage::ToggleMod {
            path: path.clone(),
            enabled,
        })
    });

    row![
        mod_icon(&theme, item.icon.as_ref()),
        space().width(Length::Fixed(7.0)),
        text,
        toggle,
    ]
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fixed(44.0))
    .padding([0, 8])
    .into()
}

/// 创建版本模组管理页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let search = container(common::text_input(
        &theme,
        "搜索模组……",
        &state.version_mods_search_filter,
        move |value| Message::Ui(UiMessage::Text(TextField::VersionModsFilter, value)),
    ))
    .padding([10, 10]);

    let mut list = column![].spacing(0);
    if state.version_mods_filtered.is_empty() {
        list = list.push(
            container(common::caption(&theme, "没有找到任何模组"))
                .padding(16)
                .width(Length::Fill)
                .center_x(Length::Fill),
        );
    } else {
        for item in state.version_mods_filtered.iter() {
            list = list.push(mod_row(&theme, item));
        }
    }

    column![
        search,
        container(common::scroll(&theme, list)).height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
