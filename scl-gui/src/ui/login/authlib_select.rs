//! Authlib 登录的角色选择独立页面。

use druid::{
    widget::{Flex, List, Scroll},
    Env, Lens, Target, Widget, WidgetExt,
};
use im::Vector;
use scl_gui_widgets::widgets::{label, LargeListItem};

use crate::{app::SELECT_AUTHLIB_CHARACTER, app_state::AppState, core::AuthMethod};

/// 角色列表的数据项
pub struct CharacterLens;

impl Lens<AppState, Vector<(usize, AuthMethod)>> for CharacterLens {
    fn with<V, F: FnOnce(&Vector<(usize, AuthMethod)>) -> V>(&self, data: &AppState, f: F) -> V {
        let characters: Vector<(usize, AuthMethod)> = data
            .login_authlib_available_characters
            .iter()
            .enumerate()
            .map(|(index, auth)| (index, auth.clone()))
            .collect();
        f(&characters)
    }

    fn with_mut<V, F: FnOnce(&mut Vector<(usize, AuthMethod)>) -> V>(
        &self,
        data: &mut AppState,
        f: F,
    ) -> V {
        let mut characters: Vector<(usize, AuthMethod)> = data
            .login_authlib_available_characters
            .iter()
            .enumerate()
            .map(|(index, auth)| (index, auth.clone()))
            .collect();
        f(&mut characters)
    }
}

fn character_name(data: &(usize, AuthMethod), _: &Env) -> String {
    data.1.display_name()
}

fn character_uuid(data: &(usize, AuthMethod), _: &Env) -> String {
    data.1.uuid().to_owned()
}

/// 创建 Authlib 角色选择页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let list = List::new(|| {
        LargeListItem::new(
            crate::theme::icons::LOGIN_AUTHLIB,
            character_name,
            character_uuid,
        )
        .on_click(|ctx, data, _env| {
            ctx.submit_command(SELECT_AUTHLIB_CHARACTER.with(data.0).to(Target::Global));
        })
    })
    .lens(CharacterLens);

    Box::new(
        Flex::column()
            .with_child(label::new("此账户包含了多个角色，请选择一个来添加：").padding(5.))
            .with_flex_child(Scroll::new(list).vertical().expand(), 1.)
            .expand()
            .padding(10.),
    )
}
