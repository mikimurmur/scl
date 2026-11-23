//! Java 运行时管理页面。

use iced::widget::{button as iced_button, column, container, row, space};
use iced::{Alignment, Element, Length};
use scl_gui_widgets::icons;
use scl_gui_widgets::theme::Theme;
use scl_gui_widgets::widgets::ButtonKind;

use crate::{
    app::{
        DELETE_JAVA, Message, SELECT_JAVA,
        java::{is_default_path, runtime_display_name},
    },
    app_state::AppState,
    config::JavaRuntimeInfo,
    ui::common,
};

fn java_row<'a>(theme: &Theme, info: &JavaRuntimeInfo, selected: bool) -> Element<'a, Message> {
    let theme = *theme;
    let icon = if info.main_version >= 16 {
        icons::JAVA_16_FULLED
    } else {
        icons::JAVA_8_FULLED
    };

    let mut labels = column![
        iced::widget::text(runtime_display_name(info))
            .size(14.0)
            .font(theme.typography.base.font)
            .color(theme.base.high),
        iced::widget::text(info.path.clone())
            .size(theme.typography.caption_alt.size)
            .font(theme.typography.caption_alt.font)
            .color(theme.base.medium),
    ]
    .spacing(1);

    if selected {
        labels = labels.push(
            iced::widget::text("使用中")
                .size(11.0)
                .font(theme.typography.caption.font)
                .color(theme.main.primary),
        );
    }

    let content = row![
        icon.view_default(&theme, 20.0),
        space().width(Length::Fixed(10.0)),
        labels,
        space().width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .padding([6, 14]);

    let main = iced_button(content)
        .width(Length::Fill)
        .padding(0)
        .style(move |_, status| {
            let background = match status {
                iced::widget::button::Status::Hovered => theme.list.low,
                iced::widget::button::Status::Pressed => theme.base.low,
                _ => iced::Color::TRANSPARENT,
            };
            iced::widget::button::Style {
                background: Some(iced::Background::Color(background)),
                text_color: theme.font_color,
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                snap: true,
            }
        })
        .on_press(common::command(SELECT_JAVA, info.clone()));

    if is_default_path(&info.path) {
        main.into()
    } else {
        let delete = scl_gui_widgets::widgets::icon_button(
            &theme,
            icons::DELETE,
            16.0,
            ButtonKind::Flat,
            common::command(DELETE_JAVA, info.clone()),
        );

        row![main, delete]
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .into()
    }
}

fn action_row<'a>(theme: &Theme, title: &'static str, message: Message) -> Element<'a, Message> {
    let theme = *theme;

    let content = row![
        icons::ADD.view_default(&theme, 22.0),
        iced::widget::text(title)
            .size(theme.typography.body.size)
            .font(theme.typography.body.font)
            .color(theme.base.high),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .padding([0, 14]);

    iced_button(content)
        .width(Length::Fill)
        .height(Length::Fixed(50.0))
        .padding(0)
        .style(move |_, status| {
            let background = match status {
                iced::widget::button::Status::Hovered => theme.list.low,
                iced::widget::button::Status::Pressed => theme.base.low,
                _ => iced::Color::TRANSPARENT,
            };
            iced::widget::button::Style {
                background: Some(iced::Background::Color(background)),
                text_color: theme.font_color,
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                snap: true,
            }
        })
        .on_press(message)
        .into()
}

/// 创建 Java 运行时管理页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let mut list = column![].spacing(0);
    for info in state.config.javas.javapaths.iter() {
        let selected = info.path == state.config.javas.selected_modern
            || info.path == state.config.javas.selected_leagzy;
        list = list.push(java_row(&theme, info, selected));
    }

    list = list.push(action_row(
        &theme,
        "添加新的 Java 运行时",
        Message::PickJava,
    ));
    list = list.push(action_row(
        &theme,
        "尝试搜索可用 Java 运行时",
        Message::SearchJava,
    ));

    container(common::scroll(&theme, list))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
