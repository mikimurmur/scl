//! 删除版本确认页面。

use druid::{
    widget::{CrossAxisAlignment, Flex},
    Color, Env, Target, Widget, WidgetExt,
};
use scl_gui_widgets::{
    theme::color::typography,
    widgets::{label, Button},
};

use crate::{app::REMOVE_VERSION, app_state::AppState};

/// 创建删除版本确认页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let title = label::new("你认真的？").with_font(typography::TITLE);

    let message = label::dynamic(|data: &AppState, _: &Env| {
        format!("确定要删除版本 {} 吗？", data.version_setting_version_name)
    });

    let warning = label::new("这个操作不可撤销！").with_text_color(Color::rgb8(255, 0, 0));

    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(title)
            .with_spacer(10.)
            .with_child(message)
            .with_spacer(10.)
            .with_child(warning)
            .with_flex_spacer(1.)
            .with_child(
                Button::new("删除版本")
                    .expand_width()
                    .on_click(|ctx, _data, _env| {
                        ctx.submit_command(REMOVE_VERSION.with(()).to(Target::Global));
                    }),
            )
            .padding(10.),
    )
}
