//! 启动器主页。

use iced::widget::{button as iced_button, column, container, row, space};
use iced::{Alignment, Element, Length};

use scl_gui_widgets::icons;
use scl_gui_widgets::theme::Theme;
use scl_gui_widgets::widgets::{self, ButtonKind};

use crate::{
    app::{Message, START_GAME},
    app_state::AppState,
    ui::{PAGE_ACCOUNTS, PAGE_DOWNLOAD, PAGE_LOGIN_METHODS, PAGE_SETTINGS, PAGE_VERSION, common},
};

const NAV_BUTTON_SIZE: f32 = 30.0;
const BOTTOM_BAR_WIDTH: f32 = 220.0;

fn nav_button<'a>(theme: &Theme, icon: icons::Icon, message: Message) -> Element<'a, Message> {
    widgets::icon_button(theme, icon, 18.0, ButtonKind::Accent, message)
}

fn nav_column<'a>(_theme: &Theme, items: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut column = column![].spacing(2).align_x(Alignment::Center);
    for item in items {
        column = column.push(item);
    }
    container(column)
        .width(Length::Fixed(NAV_BUTTON_SIZE))
        .height(Length::Fixed(62.0))
        .center_x(Length::Fixed(NAV_BUTTON_SIZE))
        .center_y(Length::Fixed(62.0))
        .style(move |_| iced::widget::container::Style {
            background: None,
            ..iced::widget::container::Style::default()
        })
        .into()
}

/// 创建主页
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let account = state.current_account();

    let account_name = account
        .as_ref()
        .map(|account| account.display_name())
        .unwrap_or_else(|| "请点击头像选择登陆方式".to_owned());

    let (skin, hat) = account
        .as_ref()
        .map(|account| (account.head_skin().to_vec(), account.hat_skin().to_vec()))
        .unwrap_or_default();

    let avatar = common::head_with_hat(&theme, &skin, &hat, 72.0);
    let avatar = iced_button(avatar)
        .padding(0)
        .style(move |_, status| iced::widget::button::Style {
            background: None,
            border: iced::border::Border {
                radius: 8.0.into(),
                ..iced::border::Border::default()
            },
            text_color: theme.font_color,
            shadow: match status {
                iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => {
                    iced::Shadow {
                        color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.2),
                        offset: iced::Vector::new(2.0, 2.0),
                        blur_radius: 8.0,
                    }
                }
                _ => iced::Shadow::default(),
            },
            snap: true,
        })
        .on_press(if account.is_some() {
            common::command(crate::app::PUSH_PAGE, PAGE_ACCOUNTS)
        } else {
            common::command(crate::app::PUSH_PAGE, PAGE_LOGIN_METHODS)
        });

    let mut account_info = column![
        iced::widget::text(account_name)
            .size(theme.typography.subtitle_alt.size)
            .font(theme.typography.subtitle_alt.font)
            .color(theme.font_color)
    ]
    .spacing(2)
    .align_x(Alignment::Center);

    if let Some(account) = &account {
        account_info = account_info.push(
            iced::widget::text(account.type_name())
                .size(11.0)
                .font(theme.typography.caption.font)
                .color(theme.font_color),
        );

        if let crate::core::AuthMethod::AuthlibInjector {
            api_location,
            server_name,
            ..
        } = account
        {
            let server = if server_name.is_empty() {
                api_location.as_str()
            } else {
                server_name.as_str()
            };
            let provider = if server.is_empty() {
                "奇怪的 BUG 出现了！".to_owned()
            } else {
                format!("由 {server} 提供验证")
            };
            account_info = account_info.push(
                iced::widget::text(provider)
                    .size(11.0)
                    .font(theme.typography.caption.font)
                    .color(theme.font_color),
            );
        }
    }

    let version_name = if state.config.selected_version.is_empty() {
        "请先选择游戏版本！".to_owned()
    } else {
        state.config.selected_version.clone()
    };

    let start = widgets::start_button(
        &theme,
        version_name,
        if state.launching {
            "正在启动游戏……".to_owned()
        } else {
            "点击启动游戏".to_owned()
        },
        !state.launching,
        Message::Command(START_GAME.with(()).to(crate::app::Target::Global)),
    );

    let left_buttons = nav_column(
        &theme,
        vec![
            nav_button(
                &theme,
                icons::VERSIONS,
                common::command(crate::app::PUSH_PAGE, PAGE_VERSION),
            ),
            nav_button(
                &theme,
                icons::DOWNLOAD,
                common::command(crate::app::PUSH_PAGE, PAGE_DOWNLOAD),
            ),
        ],
    );

    let mods_message = Message::Ui(crate::ui::UiMessage::OpenModsPage);
    let right_buttons = nav_column(
        &theme,
        vec![
            nav_button(&theme, icons::MODS, mods_message),
            nav_button(
                &theme,
                icons::SETTINGS,
                common::command(crate::app::PUSH_PAGE, PAGE_SETTINGS),
            ),
        ],
    );

    let start = container(start)
        .width(Length::Fill)
        .height(Length::Fixed(62.0));

    let bottom_bar = row![
        left_buttons,
        space().width(Length::Fixed(2.0)),
        start,
        space().width(Length::Fixed(2.0)),
        right_buttons,
    ]
    .align_y(Alignment::Center);

    let bottom_bar = container(bottom_bar)
        .width(Length::Fixed(BOTTOM_BAR_WIDTH))
        .height(Length::Fixed(62.0));

    column![
        space().height(Length::Fill),
        avatar,
        space().height(Length::Fixed(16.0)),
        account_info,
        space().height(Length::Fill),
        bottom_bar,
        space().height(Length::Fixed(40.0)),
    ]
    .spacing(0)
    .align_x(Alignment::Center)
    .padding(4)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
