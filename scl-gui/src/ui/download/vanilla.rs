//! 原版游戏版本选择子页。

use iced::Element;

use crate::{app::Message, app_state::AppState, ui::download};

/// 创建原版版本选择页
pub fn view(state: &AppState) -> Element<'_, Message> {
    download::vanilla_view(state)
}
