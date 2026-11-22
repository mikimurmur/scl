//! Iced 迁移组件测试窗口。
//!
//! 运行：
//!
//! ```sh
//! cargo run -p scl-gui-iced --example widget_test
//! ```

use iced::widget::{column, row, space};
use iced::{Element, Length, Size, Subscription, Task};
use scl_gui_iced::icons;
use scl_gui_iced::theme::Theme;
use scl_gui_iced::widgets::{self, ButtonKind, ListItem, WindowMessage};

fn main() -> iced::Result {
    iced::application(State::boot, update, view)
        .title("SCL Iced Widget Test")
        .theme(|state: &State| state.theme.iced_theme())
        .window(iced::window::Settings {
            size: Size::new(640.0, 640.0),
            min_size: Some(Size::new(420.0, 360.0)),
            decorations: false,
            resizable: true,
            ..iced::window::Settings::default()
        })
        .subscription(subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Window(WindowMessage),
    WindowReady(Option<iced::window::Id>),
    ToggleTheme,
    TextChanged(String),
    PasswordChanged(String),
    Remember(bool),
    AutoUpdate(bool),
    VolumeChanged(f32),
    SelectItem(usize),
    Tick,
}

struct State {
    theme: Theme,
    window: Option<iced::window::Id>,
    text: String,
    password: String,
    remember: bool,
    auto_update: bool,
    volume: f32,
    selected: usize,
    progress: f32,
    phase: f32,
    animate: bool,
}

impl State {
    fn boot() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme::dark(),
                window: None,
                text: "SteveXMH".into(),
                password: "password!1234".into(),
                remember: true,
                auto_update: false,
                volume: 64.0,
                selected: 0,
                progress: 0.42,
                phase: 0.0,
                animate: true,
            },
            iced::window::latest().map(Message::WindowReady),
        )
    }
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Window(WindowMessage::Back) => Task::none(),
        Message::Window(message) => match state.window {
            Some(window) => widgets::update_window(message, window),
            None => Task::none(),
        },
        Message::WindowReady(window) => {
            state.window = window;
            Task::none()
        }
        Message::ToggleTheme => {
            state.theme = state.theme.toggled();
            Task::none()
        }
        Message::TextChanged(value) => {
            state.text = value;
            Task::none()
        }
        Message::PasswordChanged(value) => {
            state.password = value;
            Task::none()
        }
        Message::Remember(value) => {
            state.remember = value;
            Task::none()
        }
        Message::AutoUpdate(value) => {
            state.auto_update = value;
            Task::none()
        }
        Message::VolumeChanged(value) => {
            state.volume = value;
            Task::none()
        }
        Message::SelectItem(index) => {
            state.selected = index;
            Task::none()
        }
        Message::Tick => {
            if state.animate {
                state.phase = (state.phase + 0.012) % 1.0;
                state.progress = (state.progress + 0.004) % 1.0;
            }
            Task::none()
        }
    }
}

fn subscription(state: &State) -> Subscription<Message> {
    if state.animate {
        iced::window::frames().map(|_| Message::Tick)
    } else {
        Subscription::none()
    }
}

fn view(state: &State) -> Element<'_, Message> {
    let theme = state.theme;
    let title = format!(
        "SCL Iced 组件测试 · {}",
        if theme.is_dark() { "暗色" } else { "亮色" }
    );

    let content = widgets::scrollable(
        &theme,
        column![
            widgets::card(
                &theme,
                column![
                    iced::widget::text("Iced + tiny-skia 组件迁移")
                        .size(theme.typography.title.size)
                        .font(theme.typography.title.font)
                        .color(theme.font_color),
                    iced::widget::text("这一页同时展示输入框、开关、滑块、按钮、进度条和列表项。")
                        .size(theme.typography.body.size)
                        .font(theme.typography.body.font)
                        .color(theme.base.medium),
                ]
                .spacing(4),
            ),
            widgets::card(
                &theme,
                column![
                    iced::widget::text("输入框")
                        .size(theme.typography.base_alt.size)
                        .font(theme.typography.base_alt.font)
                        .color(theme.font_color),
                    widgets::text_input(&theme, "用户名", &state.text, Message::TextChanged),
                    widgets::password_input(
                        &theme,
                        "密码",
                        &state.password,
                        Message::PasswordChanged
                    ),
                ]
                .spacing(10),
            ),
            widgets::card(
                &theme,
                column![
                    iced::widget::text("开关与滑块")
                        .size(theme.typography.base_alt.size)
                        .font(theme.typography.base_alt.font)
                        .color(theme.font_color),
                    widgets::setting_row(
                        &theme,
                        "记住登录",
                        "下次启动时自动恢复账户",
                        widgets::toggle_switch(&theme, state.remember, Message::Remember),
                    ),
                    widgets::setting_row(
                        &theme,
                        "自动更新",
                        "启动时检查启动器更新",
                        widgets::toggle_switch(&theme, state.auto_update, Message::AutoUpdate),
                    ),
                    widgets::setting_row(
                        &theme,
                        "内存分配",
                        format!("{} MB", state.volume as u32 * 32),
                        widgets::slider(&theme, 0.0..=256.0, state.volume, Message::VolumeChanged),
                    ),
                ]
                .spacing(12),
            ),
            widgets::card(
                &theme,
                column![
                    iced::widget::text("按钮")
                        .size(theme.typography.base_alt.size)
                        .font(theme.typography.base_alt.font)
                        .color(theme.font_color),
                    row![
                        widgets::button(
                            &theme,
                            "普通按钮",
                            ButtonKind::Standard,
                            Message::ToggleTheme
                        ),
                        widgets::button(
                            &theme,
                            "强调按钮",
                            ButtonKind::Accent,
                            Message::ToggleTheme
                        ),
                        widgets::button(
                            &theme,
                            "危险按钮",
                            ButtonKind::Danger,
                            Message::ToggleTheme
                        ),
                        widgets::flat_button(&theme, "扁平按钮", Message::ToggleTheme),
                    ]
                    .spacing(8),
                    widgets::start_button(
                        &theme,
                        "1.20.1-forge-47.2.0",
                        "点击启动游戏",
                        true,
                        Message::ToggleTheme
                    ),
                ]
                .spacing(10),
            ),
            widgets::card(
                &theme,
                column![
                    iced::widget::text("进度")
                        .size(theme.typography.base_alt.size)
                        .font(theme.typography.base_alt.font)
                        .color(theme.font_color),
                    widgets::progress_bar(&theme, state.progress, 6.0),
                    widgets::indeterminate_progress(&theme, state.phase, 6.0),
                    iced::widget::text(format!("下载进度：{:.0}%", state.progress * 100.0))
                        .size(theme.typography.caption.size)
                        .font(theme.typography.caption.font)
                        .color(theme.base.medium),
                ]
                .spacing(10),
            ),
            widgets::card(
                &theme,
                column![
                    iced::widget::text("列表项")
                        .size(theme.typography.base_alt.size)
                        .font(theme.typography.base_alt.font)
                        .color(theme.font_color),
                    ListItem::new(&theme, "Minecraft 1.20.1")
                        .icon(icons::GAME_VANILLA)
                        .subtitle("原版 · 最近启动")
                        .selected(state.selected == 0)
                        .on_press(Message::SelectItem(0))
                        .view(),
                    ListItem::new(&theme, "Forge 47.2.0")
                        .icon(icons::GAME_FORGE)
                        .subtitle("模组加载器")
                        .selected(state.selected == 1)
                        .on_press(Message::SelectItem(1))
                        .view(),
                    ListItem::new(&theme, "Fabric 0.15.7")
                        .icon(icons::GAME_FABRIC)
                        .subtitle("模组加载器")
                        .selected(state.selected == 2)
                        .on_press(Message::SelectItem(2))
                        .view(),
                ]
                .spacing(2),
            ),
            space().height(Length::Fixed(8.0)),
        ]
        .spacing(12)
        .padding(16),
    );

    widgets::window_shell(&theme, title, false, content, Message::Window)
}
