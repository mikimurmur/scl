//! 启动器的主题资源。

pub mod icons;

use scl_gui_widgets::theme::Theme;

use crate::config::Config;

/// 根据配置生成 SCL 的 Iced 主题。
pub fn theme_from_config(config: &Config) -> Theme {
    let mut theme = if config.theme.dark_theme {
        Theme::dark()
    } else {
        Theme::light()
    };

    if let Some(color) = config.theme.theme_color_secondary {
        theme.main.secondary = color.into();
    }
    if let Some(color) = config.theme.theme_color_primary {
        theme.main.primary = color.into();
    }
    if let Some(color) = config.theme.theme_color_accent {
        theme.accent.accent = color.into();
    }
    if let Some(color) = config.theme.theme_color_accent_1 {
        theme.accent.accent_1 = color.into();
    }
    if let Some(color) = config.theme.theme_color_accent_dark_1 {
        theme.accent.accent_dark_1 = color.into();
    }
    if let Some(color) = config.theme.theme_color_accent_light_1 {
        theme.accent.accent_light_1 = color.into();
    }
    if let Some(color) = config.theme.theme_color_title_bar {
        theme.main.title_bar = color.into();
    }
    if let Some(color) = config.theme.theme_color_background {
        theme.window_background = color.into();
    }

    scl_gui_widgets::icons::set_icon_theme(&config.theme.icons_theme);

    theme
}
