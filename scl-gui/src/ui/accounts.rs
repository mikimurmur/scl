//! 账户管理页面。

use iced::widget::{button as iced_button, column, container, row, space};
use iced::{Alignment, Background, Border, Element, Length};
use scl_gui_widgets::icons;
use scl_gui_widgets::theme::Theme;
use scl_gui_widgets::widgets::ButtonKind;

use crate::{
    app::{DELETE_AUTH, Message, SWITCH_AUTH},
    app_state::AppState,
    core::AuthMethod,
    ui::{PAGE_LOGIN_METHODS, TextField, UiMessage, common},
};

fn selection_dot<'a>(theme: &Theme, selected: bool) -> Element<'a, Message> {
    let theme = *theme;
    container(space())
        .width(Length::Fixed(20.0))
        .height(Length::Fixed(20.0))
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(if selected {
                theme.alt.high
            } else {
                theme.base.low
            })),
            border: Border {
                radius: 10.0.into(),
                width: if selected { 5.0 } else { 1.0 },
                color: if selected {
                    theme.main.primary
                } else {
                    theme.base.medium
                },
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

fn account_row<'a>(
    theme: &Theme,
    index: usize,
    selected: bool,
    account: &AuthMethod,
) -> Element<'a, Message> {
    let theme = *theme;
    let head = common::head_with_hat(&theme, account.head_skin(), account.hat_skin(), 40.0);

    let labels = column![
        iced::widget::text(account.display_name())
            .size(13.0)
            .font(theme.typography.base.font)
            .color(theme.font_color),
        iced::widget::text(account.type_name())
            .size(11.0)
            .font(theme.typography.caption.font)
            .color(theme.font_color),
    ]
    .spacing(2);

    let main_content = row![
        selection_dot(&theme, selected),
        space().width(Length::Fixed(8.0)),
        head,
        space().width(Length::Fixed(8.0)),
        labels,
        space().width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .padding([0, 8]);

    let main = iced_button(main_content)
        .width(Length::Fill)
        .height(Length::Fixed(50.0))
        .padding(0)
        .style(move |_, status| {
            let background = match status {
                iced::widget::button::Status::Hovered => theme.list.low,
                iced::widget::button::Status::Pressed => theme.base.low,
                _ if selected => theme.list.accent_high,
                _ => iced::Color::TRANSPARENT,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(background)),
                text_color: theme.font_color,
                border: Border::default(),
                shadow: iced::Shadow::default(),
                snap: true,
            }
        })
        .on_press(common::command(SWITCH_AUTH, index));

    let delete = scl_gui_widgets::widgets::icon_button(
        &theme,
        icons::DELETE,
        16.0,
        ButtonKind::Flat,
        common::command(DELETE_AUTH, index),
    );

    row![main, delete]
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

fn create_account_row<'a>(theme: &Theme) -> Element<'a, Message> {
    let theme = *theme;
    let content = row![
        icons::ADD.view_default(&theme, 22.0),
        iced::widget::text("创建新账户")
            .size(13.0)
            .font(theme.typography.base.font)
            .color(theme.font_color),
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
                background: Some(Background::Color(background)),
                text_color: theme.font_color,
                border: Border::default(),
                shadow: iced::Shadow::default(),
                snap: true,
            }
        })
        .on_press(common::command(crate::app::PUSH_PAGE, PAGE_LOGIN_METHODS))
        .into()
}

/// 创建账户管理页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let filter = state.accounts_filter.trim().to_lowercase();
    let selected = state.config.accounts_data.selected;

    let search = common::text_input(
        &theme,
        "搜索账户……",
        &state.accounts_filter,
        move |value| Message::Ui(UiMessage::Text(TextField::AccountsFilter, value)),
    );

    let mut list = column![].spacing(0);
    for (index, account) in state.config.accounts_data.accounts.iter().enumerate() {
        if !filter.is_empty()
            && !account.player_name().to_lowercase().contains(&filter)
            && !account.type_name().to_lowercase().contains(&filter)
        {
            continue;
        }
        list = list.push(account_row(&theme, index, index == selected, account));
    }
    list = list.push(create_account_row(&theme));

    column![
        container(search).padding([8, 8]),
        space().height(Length::Fixed(8.0)),
        container(common::scroll(&theme, list)).height(Length::Fill),
    ]
    .padding([0, 8])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
