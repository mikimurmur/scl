//! 列表项目组件。

use iced::widget::{button as iced_button, column, row};
use iced::{Element, Length};

use super::common::{ButtonKind, styled_text};
use crate::icons::{self, Icon};
use crate::theme::Theme;

/// 一个仿 WinUI 3 的列表项目。
pub struct ListItem<'a, Message> {
    theme: Theme,
    title: String,
    subtitle: Option<String>,
    icon: Option<Icon>,
    trailing: Option<Element<'a, Message>>,
    selected: bool,
    on_press: Option<Message>,
}

impl<'a, Message: Clone + 'a> ListItem<'a, Message> {
    /// 创建一个列表项目。
    pub fn new(theme: &Theme, title: impl Into<String>) -> Self {
        Self {
            theme: *theme,
            title: title.into(),
            subtitle: None,
            icon: None,
            trailing: None,
            selected: false,
            on_press: None,
        }
    }

    /// 设置左侧图标。
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// 设置副标题。
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// 设置右侧组件。
    pub fn trailing(mut self, trailing: impl Into<Element<'a, Message>>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    /// 设置选中状态。
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// 设置点击消息。
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// 构建组件。
    pub fn view(self) -> Element<'a, Message> {
        let theme = self.theme;
        let mut labels = column![styled_text(
            self.title,
            theme.typography.body,
            theme.font_color
        )]
        .spacing(1);

        if let Some(subtitle) = self.subtitle {
            labels = labels.push(styled_text(
                subtitle,
                theme.typography.caption,
                theme.base.medium,
            ));
        }

        let mut content = row![].spacing(10).align_y(iced::Center);

        if let Some(icon) = self.icon {
            content = content.push(icon.view(&theme, 18.0, theme.font_color));
        }

        content = content.push(labels.width(Length::Fill));

        if let Some(trailing) = self.trailing {
            content = content.push(trailing);
        } else {
            content = content.push(icons::CHEVRON_RIGHT.view(&theme, 14.0, theme.base.medium));
        }

        let mut widget = iced_button(content)
            .width(Length::Fill)
            .padding([8, 12])
            .style(ButtonKind::Flat.style(theme));

        if self.selected {
            widget = widget.style(move |iced_theme, status| {
                let mut style = ButtonKind::Flat.style(theme)(iced_theme, status);
                style.background = Some(iced::Background::Color(theme.list.low));
                style
            });
        }

        if let Some(message) = self.on_press {
            widget = widget.on_press(message);
        }

        widget.into()
    }
}
