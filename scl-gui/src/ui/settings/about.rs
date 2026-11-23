//! 关于页面。

use iced::widget::{column, container, row, space};
use iced::{Alignment, Element, Length};
use scl_gui_widgets::icons;
use scl_gui_widgets::theme::Theme;
use scl_gui_widgets::widgets::ButtonKind;

use crate::{
    app::{Message, OPEN_URL},
    app_state::AppState,
    ui::{PAGE_UPDATE_LOG, UiMessage, common},
};

fn link_button<'a>(theme: &Theme, text: &'static str, url: &'static str) -> Element<'a, Message> {
    container(common::button(
        theme,
        text,
        ButtonKind::Standard,
        common::command(OPEN_URL, url.to_owned()),
    ))
    .width(Length::FillPortion(1))
    .into()
}

fn dependency<'a>(
    theme: &Theme,
    name: &'static str,
    author: &'static str,
    license: &'static str,
) -> Element<'a, Message> {
    let mut content = column![common::label(theme, name)].spacing(1);
    for text in [author, license] {
        if !text.is_empty() {
            content = content.push(common::caption(theme, text));
        }
    }
    content.into()
}

/// 创建关于页面（不含外层 padding / scroll）
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let update_status = iced::widget::text(match &state.update_available {
        Some(version) if version == env!("CARGO_PKG_VERSION") => "已是最新版本".to_owned(),
        Some(version) => format!("有可用版本更新：{version}"),
        None => "检查更新中……".to_owned(),
    })
    .size(theme.typography.caption_alt.size)
    .font(theme.typography.caption_alt.font)
    .color(theme.font_color);

    let can_check = state
        .update_available
        .as_deref()
        .is_some_and(|version| !version.is_empty());
    let can_update = state
        .update_available
        .as_deref()
        .is_some_and(|version| !version.is_empty() && version != env!("CARGO_PKG_VERSION"));

    let update_row = row![
        container(common::button_maybe(
            &theme,
            "检查版本",
            ButtonKind::Standard,
            can_check.then_some(Message::Ui(UiMessage::CheckUpdate)),
        ))
        .width(Length::FillPortion(1)),
        space().width(Length::Fixed(5.0)),
        container(common::button_maybe(
            &theme,
            "立即更新",
            ButtonKind::Accent,
            can_update.then_some(common::command(crate::app::PUSH_PAGE, PAGE_UPDATE_LOG,)),
        ))
        .width(Length::FillPortion(1)),
    ]
    .align_y(Alignment::Center);

    let links_row = |left: (&'static str, &'static str), right: (&'static str, &'static str)| {
        row![
            link_button(&theme, left.0, left.1),
            space().width(Length::Fixed(5.0)),
            link_button(&theme, right.0, right.1),
        ]
        .align_y(Alignment::Center)
    };

    let dependencies = column![
        dependency(&theme, "surf", "By dignifiedquire", "MIT OR Apache-2.0"),
        dependency(
            &theme,
            "bangbang93's minecraft launcher api",
            "By bangbang93",
            "https://bmclapidoc.bangbang93.com/",
        ),
        dependency(&theme, "iced", "By Héctor Ramón", "Under MIT licence",),
        dependency(
            &theme,
            "tiny-skia",
            "By RazrFalcon",
            "Under BSD-3-Clause licence",
        ),
        dependency(&theme, "image", "", "Under MIT licence"),
        dependency(&theme, "Rust Programming Language", "By Mozilla", ""),
        dependency(
            &theme,
            "serde serde_json",
            "By David Tolnay",
            "Under MIT OR Apache-2.0 licence",
        ),
        dependency(&theme, "smol", "By smol-rs", "Under Apache-2.0 OR MIT"),
    ]
    .spacing(4);

    let icon = icons::VERSIONS.view_default(&theme, 48.0);

    column![
        row![
            icon,
            space().width(Length::Fixed(12.0)),
            column![
                common::label(&theme, "Sharp Craft Launcher 1.2.0"),
                common::label(&theme, "By SteveXMH"),
            ]
            .spacing(2),
        ]
        .align_y(Alignment::Center),
        common::label(
            &theme,
            concat!("编译日期：", env!("SCL_BUILD_DATE"))
        ),
        common::label(
            &theme,
            format!(
                "操作系统：{}",
                match std::env::consts::OS {
                    "windows" => "Windows",
                    "macos" => "macOS",
                    "linux" => "Linux",
                    other => other,
                }
            )
        ),
        common::label(&theme, format!("软件架构：{}", std::env::consts::ARCH)),
        space().height(Length::Fixed(8.0)),
        update_status,
        space().height(Length::Fixed(8.0)),
        update_row,
        space().height(Length::Fixed(5.0)),
        links_row(
            (
                "启动器主页",
                "https://steve-xmh.github.io/scl/index.html",
            ),
            ("Github 仓库", "https://github.com/Steve-xmh/scl"),
        ),
        space().height(Length::Fixed(5.0)),
        links_row(
            ("爱发电", "http://afdian.net/@SteveXMH"),
            ("赞助 BMCLAPI", "http://afdian.net/@bangbang93"),
        ),
        space().height(Length::Fixed(8.0)),
        common::title(&theme, "鸣谢"),
        space().height(Length::Fixed(8.0)),
        common::caption(
            &theme,
            "由于位置有限，只能展示主要使用的几个框架/库/服务，如果缺失了您的名字还请与作者私聊添加。感谢以下框架的开发者们！",
        ),
        space().height(Length::Fixed(8.0)),
        dependencies,
    ]
    .spacing(4)
    .into()
}
