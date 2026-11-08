//! 模组文件选择页面。

use druid::{
    widget::{Align, CrossAxisAlignment, Flex, List, Scroll, SizedBox},
    Env, Target, UnitPoint, Widget, WidgetExt,
};
use scl_gui_widgets::widgets::{DownloadModuleItem, ProgressRing};

use crate::{
    app::CF_DOWNLOAD_MOD,
    app_state::{AppState, ModFileState},
    widgets::{ModListItem, OptionView},
};

fn file_name(data: &ModFileState, _: &Env) -> String {
    data.file_name.clone()
}

fn file_desc(_data: &ModFileState, _: &Env) -> String {
    String::new()
}

fn file_row() -> impl Widget<ModFileState> {
    DownloadModuleItem::dynamic(crate::theme::icons::MODS, file_name, file_desc).on_click(
        |ctx, data, _env| {
            ctx.submit_command(CF_DOWNLOAD_MOD.with(data.clone()).to(Target::Global));
        },
    )
}

/// 文件列表分支：可滚动的文件行列表
fn files_view() -> impl Widget<im::Vector<ModFileState>> {
    Scroll::new(List::new(file_row).expand_width())
        .vertical()
        .expand()
}

/// 文件列表加载中分支：内容区居中的转圈
fn loading_view() -> impl Widget<()> {
    SizedBox::new(Align::new(UnitPoint::CENTER, ProgressRing::new())).expand()
}

/// 创建模组文件选择页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    // 顶部模组信息行：静态模式（无 hover 高亮），与官方同一组件。
    // 用稳定的 OptionView 替代 Maybe，避免 info 迟到/清空时触发 druid
    // BuildFocusChain-before-WidgetAdded 断言（见 widgets/option_view.rs）。
    let info = OptionView::new(ModListItem::new_static(), SizedBox::empty())
        .lens(AppState::mod_file_downloads_info);

    let list =
        OptionView::new(files_view(), loading_view()).lens(AppState::mod_file_downloads_files);

    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(info)
            .with_flex_child(list, 1.0)
            .expand(),
    )
}
