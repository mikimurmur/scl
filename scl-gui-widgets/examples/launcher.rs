//! 用 Iced + tiny-skia 重写的启动器外壳示例。
//!
//! 这个示例不是完整的启动器，而是把旧 `scl-gui` 的主页、版本列表、
//! 下载页和设置页用新的 Iced 组件库重新搭了一遍，用来验证迁移路线。
//!
//! 运行：
//!
//! ```sh
//! cargo run -p scl-gui-widgets --example launcher
//! ```

use iced::widget::{column, container, row, space, text};
use iced::{Element, Length, Size, Subscription, Task};
use scl_gui_widgets::icons::{self, Icon};
use scl_gui_widgets::theme::Theme;
use scl_gui_widgets::widgets::{
    self, ButtonKind, ListItem, NavigationItem, PageSwitcher, WindowMessage,
};

fn main() -> iced::Result {
    iced::application(State::boot, update, view)
        .title("Sharp Craft Launcher · Iced")
        .theme(|state: &State| state.theme.iced_theme())
        .window(iced::window::Settings {
            size: Size::new(420.0, 620.0),
            min_size: Some(Size::new(360.0, 480.0)),
            decorations: false,
            transparent: false,
            resizable: true,
            ..iced::window::Settings::default()
        })
        .subscription(subscription)
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Versions,
    Downloads,
    Settings,
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Self::Home => "主页",
            Self::Versions => "版本",
            Self::Downloads => "下载",
            Self::Settings => "设置",
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Window(WindowMessage),
    WindowReady(Option<iced::window::Id>),
    Navigate(Page),
    ToggleTheme,
    Launch,
    Tick,
    SearchChanged(String),
    SelectVersion(usize),
    AutoLogin(bool),
    AutoUpdate(bool),
    MemoryChanged(f32),
    GameDirChanged(String),
    ClearCache,
    CheckUpdates,
}

#[derive(Debug, Clone)]
struct Version {
    name: String,
    kind: String,
    icon: Icon,
}

struct State {
    theme: Theme,
    window: Option<iced::window::Id>,
    pages: PageSwitcher<Page>,
    account: String,
    account_type: String,
    versions: Vec<Version>,
    selected_version: usize,
    search: String,
    launching: bool,
    progress: f32,
    phase: f32,
    status: String,
    auto_login: bool,
    auto_update: bool,
    memory: f32,
    game_dir: String,
}

impl State {
    fn boot() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme::dark(),
                window: None,
                pages: PageSwitcher::new(Page::Home),
                account: "SteveXMH".into(),
                account_type: "Microsoft 正版".into(),
                versions: vec![
                    Version {
                        name: "1.20.1-forge-47.2.0".into(),
                        kind: "Forge · 最近启动".into(),
                        icon: icons::GAME_FORGE,
                    },
                    Version {
                        name: "1.21.1-neoforge-21.1.80".into(),
                        kind: "NeoForge".into(),
                        icon: icons::GAME_NEOFORGE,
                    },
                    Version {
                        name: "1.21.4-fabric-0.16.9".into(),
                        kind: "Fabric".into(),
                        icon: icons::GAME_FABRIC,
                    },
                    Version {
                        name: "1.20.4".into(),
                        kind: "原版".into(),
                        icon: icons::GAME_VANILLA,
                    },
                    Version {
                        name: "1.12.2-forge-14.23.5.2860".into(),
                        kind: "Forge".into(),
                        icon: icons::GAME_FORGE,
                    },
                ],
                selected_version: 0,
                search: String::new(),
                launching: false,
                progress: 0.0,
                phase: 0.0,
                status: "准备就绪".into(),
                auto_login: true,
                auto_update: true,
                memory: 128.0,
                game_dir: "~/.minecraft".into(),
            },
            iced::window::latest().map(Message::WindowReady),
        )
    }

    fn selected_version(&self) -> &Version {
        &self.versions[self.selected_version.min(self.versions.len() - 1)]
    }
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Window(WindowMessage::Back) => {
            state.pages.pop();
            Task::none()
        }
        Message::Window(message) => match state.window {
            Some(window) => widgets::update_window(message, window),
            None => Task::none(),
        },
        Message::WindowReady(window) => {
            state.window = window;
            Task::none()
        }
        Message::Navigate(page) => {
            state.pages.push(page);
            Task::none()
        }
        Message::ToggleTheme => {
            state.theme = state.theme.toggled();
            Task::none()
        }
        Message::Launch => {
            state.launching = true;
            state.progress = 0.0;
            state.status = format!("正在启动 {}...", state.selected_version().name);
            Task::none()
        }
        Message::Tick => {
            state.phase = (state.phase + 0.015) % 1.0;
            if state.launching {
                state.progress = (state.progress + 0.012).min(1.0);
                if state.progress >= 1.0 {
                    state.launching = false;
                    state.status = "游戏已启动（示例）".into();
                }
            }
            Task::none()
        }
        Message::SearchChanged(value) => {
            state.search = value;
            Task::none()
        }
        Message::SelectVersion(index) => {
            state.selected_version = index;
            state.status = format!("已选择 {}", state.selected_version().name);
            Task::none()
        }
        Message::AutoLogin(value) => {
            state.auto_login = value;
            Task::none()
        }
        Message::AutoUpdate(value) => {
            state.auto_update = value;
            Task::none()
        }
        Message::MemoryChanged(value) => {
            state.memory = value;
            Task::none()
        }
        Message::GameDirChanged(value) => {
            state.game_dir = value;
            Task::none()
        }
        Message::ClearCache => {
            state.status = "缓存已清理（示例）".into();
            Task::none()
        }
        Message::CheckUpdates => {
            state.status = "已是最新版本（示例）".into();
            Task::none()
        }
    }
}

fn subscription(state: &State) -> Subscription<Message> {
    if state.launching || *state.pages.current() == Page::Downloads {
        iced::window::frames().map(|_| Message::Tick)
    } else {
        Subscription::none()
    }
}

fn view(state: &State) -> Element<'_, Message> {
    let theme = state.theme;
    let current = *state.pages.current();
    let active = match current {
        Page::Home => 0,
        Page::Versions => 1,
        Page::Downloads => 2,
        Page::Settings => 3,
    };

    let nav = widgets::navigation_control(
        &theme,
        [
            NavigationItem::new(Page::Home.title(), Message::Navigate(Page::Home)),
            NavigationItem::new(Page::Versions.title(), Message::Navigate(Page::Versions)),
            NavigationItem::new(Page::Downloads.title(), Message::Navigate(Page::Downloads)),
            NavigationItem::new(Page::Settings.title(), Message::Navigate(Page::Settings)),
        ],
        active,
    );

    let page: Element<'_, Message> = match current {
        Page::Home => home(state),
        Page::Versions => versions(state),
        Page::Downloads => downloads(state),
        Page::Settings => settings(state),
    };
    let page = widgets::scrollable(&theme, page).height(Length::Fill);

    let content = column![nav, page].spacing(8).padding([0, 12]);

    widgets::window_shell(
        &theme,
        "Sharp Craft Launcher · Iced 迁移预览",
        state.pages.can_go_back(),
        content,
        Message::Window,
    )
}

fn home(state: &State) -> Element<'_, Message> {
    let theme = state.theme;
    let avatar = container(icons::USER.view(&theme, 36.0, theme.chrome.white_high))
        .center(76)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(theme.main.primary)),
            border: iced::border::Border {
                radius: 38.0.into(),
                width: 3.0,
                color: theme.chrome.white_high,
            },
            ..container::Style::default()
        });

    let account = column![
        text(state.account.clone())
            .size(theme.typography.subtitle_alt.size)
            .font(theme.typography.base_alt.font)
            .color(theme.font_color),
        text(state.account_type.clone())
            .size(theme.typography.caption.size)
            .font(theme.typography.caption.font)
            .color(theme.base.medium),
    ]
    .spacing(2)
    .align_x(iced::Center);

    let hero = widgets::card(
        &theme,
        column![
            avatar,
            account,
            space().height(Length::Fixed(4.0)),
            widgets::start_button(
                &theme,
                state.selected_version().name.clone(),
                if state.launching {
                    format!("正在启动 · {:.0}%", state.progress * 100.0)
                } else {
                    "点击启动游戏".into()
                },
                !state.launching,
                Message::Launch,
            ),
            widgets::progress_bar(&theme, state.progress, 4.0),
        ]
        .spacing(8)
        .align_x(iced::Center),
    );

    let actions = row![
        action_button(
            &theme,
            icons::VERSIONS,
            "版本",
            Message::Navigate(Page::Versions)
        ),
        action_button(
            &theme,
            icons::DOWNLOAD,
            "下载",
            Message::Navigate(Page::Downloads)
        ),
        action_button(
            &theme,
            icons::MODS,
            "模组",
            Message::Navigate(Page::Versions)
        ),
        action_button(
            &theme,
            icons::SETTINGS,
            "设置",
            Message::Navigate(Page::Settings)
        ),
    ]
    .spacing(6);

    let mut recent = column![].spacing(2);
    for (index, version) in state.versions.iter().take(3).enumerate() {
        recent = recent.push(
            ListItem::new(&theme, version.name.clone())
                .icon(version.icon)
                .subtitle(version.kind.clone())
                .selected(index == state.selected_version)
                .on_press(Message::SelectVersion(index))
                .view(),
        );
    }

    column![
        hero,
        actions,
        widgets::card(
            &theme,
            column![
                text("最近启动")
                    .size(theme.typography.base_alt.size)
                    .font(theme.typography.base_alt.font)
                    .color(theme.font_color),
                recent,
            ]
            .spacing(6),
        ),
        status_bar(&theme, &state.status),
        space().height(Length::Fixed(12.0)),
    ]
    .spacing(10)
    .into()
}

fn versions(state: &State) -> Element<'_, Message> {
    let theme = state.theme;
    let filter = state.search.to_lowercase();
    let mut list = column![].spacing(2);
    let mut visible = 0;

    for (index, version) in state.versions.iter().enumerate() {
        if !filter.is_empty()
            && !version.name.to_lowercase().contains(&filter)
            && !version.kind.to_lowercase().contains(&filter)
        {
            continue;
        }

        visible += 1;
        list = list.push(
            ListItem::new(&theme, version.name.clone())
                .icon(version.icon)
                .subtitle(version.kind.clone())
                .selected(index == state.selected_version)
                .on_press(Message::SelectVersion(index))
                .view(),
        );
    }

    if visible == 0 {
        list = list.push(
            text("没有匹配的版本")
                .size(theme.typography.body.size)
                .font(theme.typography.body.font)
                .color(theme.base.medium),
        );
    }

    column![
        widgets::text_input(&theme, "搜索版本...", &state.search, Message::SearchChanged),
        row![
            widgets::button(&theme, "启动选中版本", ButtonKind::Accent, Message::Launch),
            widgets::button(&theme, "刷新", ButtonKind::Standard, Message::CheckUpdates),
        ]
        .spacing(8),
        widgets::card(
            &theme,
            widgets::scrollable(&theme, list).height(Length::Fixed(360.0)),
        ),
        status_bar(&theme, &state.status),
    ]
    .spacing(10)
    .into()
}

fn downloads(state: &State) -> Element<'_, Message> {
    let theme = state.theme;
    let entries = [
        ("1.21.4 客户端", 0.72, "12.4 MB/s"),
        ("Fabric Loader 0.16.9", 0.45, "8.1 MB/s"),
        ("资源索引 1.21", 0.18, "等待中"),
    ];

    let mut list = column![].spacing(10);
    for (name, progress, speed) in entries {
        list = list.push(
            column![
                row![
                    text(name)
                        .size(theme.typography.body.size)
                        .font(theme.typography.body.font)
                        .color(theme.font_color),
                    space().width(Length::Fill),
                    text(speed)
                        .size(theme.typography.caption.size)
                        .font(theme.typography.caption.font)
                        .color(theme.base.medium),
                ],
                widgets::progress_bar(&theme, progress, 6.0),
            ]
            .spacing(4),
        );
    }

    column![
        widgets::card(
            &theme,
            column![
                text("正在下载")
                    .size(theme.typography.base_alt.size)
                    .font(theme.typography.base_alt.font)
                    .color(theme.font_color),
                list,
                widgets::indeterminate_progress(&theme, state.phase, 6.0),
            ]
            .spacing(12),
        ),
        row![
            widgets::button(&theme, "暂停全部", ButtonKind::Standard, Message::Tick),
            widgets::button(&theme, "清除已完成", ButtonKind::Flat, Message::ClearCache),
        ]
        .spacing(8),
        status_bar(&theme, &state.status),
    ]
    .spacing(10)
    .into()
}

fn settings(state: &State) -> Element<'_, Message> {
    let theme = state.theme;

    column![
        widgets::card(
            &theme,
            column![
                text("账户")
                    .size(theme.typography.base_alt.size)
                    .font(theme.typography.base_alt.font)
                    .color(theme.font_color),
                widgets::setting_row(
                    &theme,
                    "自动登录",
                    "启动时自动恢复上次使用的账户",
                    widgets::toggle_switch(&theme, state.auto_login, Message::AutoLogin),
                ),
                widgets::setting_row(
                    &theme,
                    "自动检查更新",
                    "启动时检查启动器新版本",
                    widgets::toggle_switch(&theme, state.auto_update, Message::AutoUpdate),
                ),
            ]
            .spacing(12),
        ),
        widgets::card(
            &theme,
            column![
                text("游戏")
                    .size(theme.typography.base_alt.size)
                    .font(theme.typography.base_alt.font)
                    .color(theme.font_color),
                text("游戏目录")
                    .size(theme.typography.caption.size)
                    .font(theme.typography.caption.font)
                    .color(theme.base.medium),
                widgets::text_input(
                    &theme,
                    "例如 ~/.minecraft",
                    &state.game_dir,
                    Message::GameDirChanged
                ),
                widgets::setting_row(
                    &theme,
                    "内存分配",
                    format!("{} MB", (state.memory * 32.0) as u32),
                    widgets::slider(&theme, 0.0..=256.0, state.memory, Message::MemoryChanged),
                ),
            ]
            .spacing(10),
        ),
        widgets::card(
            &theme,
            column![
                text("外观")
                    .size(theme.typography.base_alt.size)
                    .font(theme.typography.base_alt.font)
                    .color(theme.font_color),
                widgets::setting_row(
                    &theme,
                    "暗色模式",
                    "在亮色和暗色主题之间切换",
                    widgets::toggle_switch(&theme, state.theme.is_dark(), move |_| {
                        Message::ToggleTheme
                    }),
                ),
            ]
            .spacing(12),
        ),
        row![
            widgets::button(
                &theme,
                "检查更新",
                ButtonKind::Accent,
                Message::CheckUpdates
            ),
            widgets::button(
                &theme,
                "清理缓存",
                ButtonKind::Standard,
                Message::ClearCache
            ),
        ]
        .spacing(8),
        status_bar(&theme, &state.status),
        space().height(Length::Fixed(12.0)),
    ]
    .spacing(10)
    .into()
}

fn action_button<'a>(
    theme: &Theme,
    icon: Icon,
    label: &'static str,
    message: Message,
) -> Element<'a, Message> {
    let theme = *theme;

    column![
        widgets::icon_button(&theme, icon, 18.0, ButtonKind::Accent, message),
        text(label)
            .size(theme.typography.caption.size)
            .font(theme.typography.caption.font)
            .color(theme.base.medium),
    ]
    .spacing(2)
    .align_x(iced::Center)
    .into()
}

fn status_bar<'a>(theme: &Theme, status: &str) -> Element<'a, Message> {
    let theme = *theme;

    container(
        row![
            icons::CHECK.view(&theme, 12.0, theme.base.medium),
            text(status.to_owned())
                .size(theme.typography.caption.size)
                .font(theme.typography.caption.font)
                .color(theme.base.medium),
        ]
        .spacing(6)
        .align_y(iced::Center),
    )
    .width(Length::Fill)
    .padding([4, 6])
    .into()
}
