//! NeoForge 加载器版本选择子页。

use iced::Element;

use crate::{app::Message, app_state::AppState, ui::download};

/// 创建 NeoForge 版本选择页
pub fn view(state: &AppState) -> Element<'_, Message> {
    download::string_loader_view(state, download::neoforge_page())
}
