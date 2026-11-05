//! 启动器主页。

use druid::{
    widget::{CrossAxisAlignment, Flex, SizedBox, ViewSwitcher},
    Env, Insets, LensExt, Target, Widget, WidgetExt,
};
use scl_gui_widgets::widgets::{IconButton, StartGameButton};

use crate::{
    app::START_GAME,
    app_state::AppState,
    config::{AccountData, Config},
    ui::{
        PAGE_ACCOUNTS, PAGE_DOWNLOAD, PAGE_LOGIN_METHODS, PAGE_MODS_DOWNLOAD, PAGE_SETTINGS,
        PAGE_VERSION, PUSH_PAGE,
    },
    widgets::HeadImage,
};

const BOTTOM_BAR_WIDTH: f64 = 220.;
const BOTTOM_BAR_HEIGHT: f64 = 62.;
/// 导航图标按钮的尺寸
const NAV_BUTTON_SIZE: f64 = 30.;

fn nav_button(
    icon: scl_gui_widgets::theme::icons::IconKeyPair,
    page: &'static str,
) -> impl Widget<AppState> {
    IconButton::new(icon)
        .with_accent(true)
        .on_click(move |ctx, _data, _env| {
            ctx.submit_command(PUSH_PAGE.with(page).to(Target::Global));
        })
        .fix_width(NAV_BUTTON_SIZE)
        .fix_height(NAV_BUTTON_SIZE)
}

/// 创建主页
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    let head = HeadImage::<AccountData>::new()
        .lens(AppState::config.then(Config::accounts_data))
        .on_click(|ctx: &mut druid::EventCtx, data: &mut AppState, _env| {
            if data.current_account().is_some() {
                ctx.submit_command(PUSH_PAGE.with(PAGE_ACCOUNTS).to(Target::Global));
            } else {
                ctx.submit_command(PUSH_PAGE.with(PAGE_LOGIN_METHODS).to(Target::Global));
            }
        });

    let account_label = scl_gui_widgets::widgets::label::dynamic(|data: &AppState, _: &Env| {
        match data.current_account() {
            Some(account) => account.display_name(),
            None => "请点击头像选择登陆方式".to_owned(),
        }
    });

    // 只有登录后才显示账户类型。未登录时不能留下一个空标签，
    // 否则会把头像和提示文字整体向上顶。
    // 原版主页把账户名与账户类型放在同一个居中的列里；类型文本使用
    // caption-alt 字体键，并把字号覆盖为 11，颜色保持 label::new 的默认
    // typography::FONT_COLOR。
    let account_type_label = ViewSwitcher::new(
        |data: &AppState, _: &Env| {
            data.current_account().map(|account| {
                let type_name = match &account {
                    crate::core::AuthMethod::Offline { .. } => "离线账户",
                    crate::core::AuthMethod::Mojang { .. } => "Mojang 正版",
                    crate::core::AuthMethod::Microsoft { .. } => "Microsoft 正版",
                    crate::core::AuthMethod::AuthlibInjector { .. } => {
                        "Authlib-Injector 第三方正版"
                    }
                }
                .to_owned();
                let provider = match &account {
                    crate::core::AuthMethod::AuthlibInjector {
                        api_location,
                        server_name,
                        ..
                    } => {
                        let server = if server_name.is_empty() {
                            api_location.as_str()
                        } else {
                            server_name.as_str()
                        };
                        if server.is_empty() {
                            Some("奇怪的 BUG 出现了！".to_owned())
                        } else {
                            Some(format!("由 {server} 提供验证"))
                        }
                    }
                    _ => None,
                };
                (type_name, provider)
            })
        },
        |value: &Option<(String, Option<String>)>,
         _data: &AppState,
         env: &Env|
         -> Box<dyn Widget<AppState>> {
            match value {
                Some((type_name, provider)) => {
                    let font = env
                        .get(scl_gui_widgets::theme::color::typography::CAPTION_ALT)
                        .with_size(11.);
                    let mut column = Flex::column()
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(
                            scl_gui_widgets::widgets::label::new(type_name.clone())
                                .with_font(font.clone()),
                        );
                    if let Some(provider) = provider {
                        column = column.with_child(
                            scl_gui_widgets::widgets::label::new(provider.clone()).with_font(font),
                        );
                    }
                    Box::new(column)
                }
                None => Box::new(SizedBox::empty()),
            }
        },
    );

    let account_info = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(account_label)
        .with_child(account_type_label);

    let start_button = StartGameButton::dynamic(|data: &AppState, _: &Env| {
        if data.config.selected_version.is_empty() {
            "请先选择游戏版本！".to_owned()
        } else {
            data.config.selected_version.clone()
        }
    })
    .with_accent(true)
    .on_click(|ctx, _data, _env| {
        ctx.submit_command(START_GAME.with(()).to(Target::Global));
    });
    let start_button = SizedBox::new(start_button)
        .width(BOTTOM_BAR_WIDTH)
        .height(BOTTOM_BAR_HEIGHT);

    let left_buttons = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(nav_button(crate::theme::icons::VERSIONS, PAGE_VERSION))
        .with_spacer(2.)
        .with_child(nav_button(crate::theme::icons::DOWNLOAD, PAGE_DOWNLOAD));

    let right_buttons = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(
            IconButton::new(crate::theme::icons::MODS)
                .with_accent(true)
                .on_click(|ctx, data: &mut AppState, _env| {
                    if !data.config.selected_version.is_empty() {
                        data.mod_download_install_version = data.config.selected_version.clone();
                    }
                    ctx.submit_command(PUSH_PAGE.with(PAGE_MODS_DOWNLOAD).to(Target::Global));
                })
                .fix_width(NAV_BUTTON_SIZE)
                .fix_height(NAV_BUTTON_SIZE),
        )
        .with_spacer(2.)
        .with_child(nav_button(crate::theme::icons::SETTINGS, PAGE_SETTINGS));

    let bottom_bar = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(left_buttons)
        .with_spacer(2.)
        .with_flex_child(start_button, 1.0)
        .with_spacer(2.)
        .with_child(right_buttons);

    let bottom_bar = SizedBox::new(bottom_bar)
        .width(BOTTOM_BAR_WIDTH)
        .height(BOTTOM_BAR_HEIGHT);

    Box::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .with_flex_spacer(1.0)
            .with_child(head)
            .with_spacer(16.)
            .with_child(account_info)
            .with_flex_spacer(1.0)
            .with_child(bottom_bar)
            .with_spacer(40.)
            .padding(Insets::uniform(4.))
            .expand(),
    )
}
