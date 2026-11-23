//! 启动器设置页面。

pub mod about;
pub mod java;
pub mod launch;
pub mod update;

use iced::widget::{column, container, space};
use iced::{Element, Length};
use scl_gui_widgets::theme::Theme;
use scl_gui_widgets::widgets::{self, NavigationItem};

use crate::{
    app::Message,
    app_state::AppState,
    ui::{BoolField, UiMessage, common},
};

/// 个性化/关于页的说明标签。
pub(crate) fn caption<'a>(theme: &Theme, text: impl Into<String>) -> Element<'a, Message> {
    common::caption(theme, text)
}

/// 创建个性化页面（不含外层 padding / scroll）。
fn personalize_page(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let content = column![
        common::label(&theme, "黑暗模式"),
        common::toggle_switch(&theme, state.config.theme.dark_theme, move |value| {
            Message::Ui(UiMessage::Bool(BoolField::DarkTheme, value))
        }),
        common::label(&theme, "使用滑动页面切换动画"),
        common::toggle_switch(
            &theme,
            state.config.theme.slide_page_animation,
            move |value| Message::Ui(UiMessage::Bool(BoolField::SlideAnimation, value))
        ),
        space().height(Length::Fixed(8.0)),
        caption(
            &theme,
            "虽然看上去少，其实配置文件里有更多修改选项~\n\
             欲知配置文件的结构及如何修改，请咨询作者小萧哦~"
        ),
    ]
    .spacing(4);

    common::scroll(&theme, container(content).padding(12))
}

/// 创建设置页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let navigation = widgets::navigation_control(
        &theme,
        [
            NavigationItem::new("启动", Message::Ui(UiMessage::SetSettingPage(0))),
            NavigationItem::new("个性化", Message::Ui(UiMessage::SetSettingPage(1))),
            NavigationItem::new("关于", Message::Ui(UiMessage::SetSettingPage(2))),
        ],
        state.setting_page_index,
    );

    let page = match state.setting_page_index {
        1 => personalize_page(state),
        2 => common::scroll(&theme, container(about::view(state)).padding(12)),
        _ => launch::view(state),
    };

    column![navigation, container(page).height(Length::Fill)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
