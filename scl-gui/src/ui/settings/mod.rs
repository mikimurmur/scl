//! 启动器设置页面。

pub mod about;
pub mod java;
pub mod launch;
pub mod update;

use druid::{
    widget::{CrossAxisAlignment, Flex, Label, ViewSwitcher},
    LensExt, Widget, WidgetExt,
};
use scl_gui_widgets::widgets::{label, NavigationControl, ToggleSwitch};

use crate::{
    app_state::AppState,
    config::{Config, ThemeConfig},
};

/// 个性化/关于页的说明标签
pub(crate) fn caption(text: &'static str) -> Label<AppState> {
    label::new(text)
}

/// 创建个性化页面（不含外层 padding / scroll）
fn personalize_page() -> Box<dyn Widget<AppState>> {
    let page = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(label::new("黑暗模式"))
        .with_child(
            ToggleSwitch::new().lens(
                AppState::config
                    .then(Config::theme)
                    .then(ThemeConfig::dark_theme),
            ),
        )
        .with_child(label::new("使用滑动页面切换动画"))
        .with_child(
            ToggleSwitch::new().lens(
                AppState::config
                    .then(Config::theme)
                    .then(ThemeConfig::slide_page_animation),
            ),
        )
        .with_spacer(8.)
        .with_child(caption(
            "虽然看上去少，其实配置文件里有更多修改选项~\n\
             欲知配置文件的结构及如何修改，请咨询作者小萧哦~",
        ));

    Box::new(page)
}

/// 创建设置页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let navigation = NavigationControl::new()
        .with_page("启动".to_owned())
        .with_page("个性化".to_owned())
        .with_page("关于".to_owned())
        .lens(AppState::setting_page_index);

    let views = ViewSwitcher::new(
        |data: &AppState, _env| data.setting_page_index,
        |index: &usize, _data: &AppState, _env| -> Box<dyn Widget<AppState>> {
            match *index {
                1 => Box::new(personalize_page().padding(12.).scroll().vertical().boxed()),
                2 => Box::new(about::ui_builder().padding(12.).scroll().vertical().boxed()),
                _ => launch::ui_builder(),
            }
        },
    );

    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(navigation)
            .with_flex_child(views, 1.0)
            .boxed(),
    )
}
