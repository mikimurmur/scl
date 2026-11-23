//! 启动器的所有界面。

pub mod accounts;
pub mod common;
pub mod download;
pub mod game_dir;
pub mod launch_error;
pub mod login;
pub mod login_methods;
pub mod mod_download_dest;
pub mod mod_files_download;
pub mod mods_download;
pub mod mods_download_curseforge;
pub mod mods_download_modrinth;
pub mod settings;
pub mod starting;
pub mod version;
pub mod version_setting;
pub mod version_setting_advance;
pub mod version_setting_mods;
pub mod version_setting_remove;

use iced::widget::{column, container, row, space};
use iced::{Alignment, Background, Border, Element, Length, Shadow};

use crate::app::Message;
pub use crate::app::{PUSH_PAGE, QUERY_POP_PAGE, RESET_PAGE_TO};
use crate::app_state::AppState;

/// 主页面 ID
pub const PAGE_MAIN: &str = "main";
/// 版本列表页面 ID
pub const PAGE_VERSION: &str = "version";
/// 下载页面 ID
pub const PAGE_DOWNLOAD: &str = "download";
/// 原版版本选择子页 ID
pub const PAGE_DOWNLOAD_VANILLA: &str = "download_vanilla";
/// Fabric 版本选择子页 ID
pub const PAGE_DOWNLOAD_FABRIC: &str = "download_fabric";
/// QuiltMC 版本选择子页 ID
pub const PAGE_DOWNLOAD_QUILTMC: &str = "download_quiltmc";
/// Forge 版本选择子页 ID
pub const PAGE_DOWNLOAD_FORGE: &str = "download_forge";
/// NeoForge 版本选择子页 ID
pub const PAGE_DOWNLOAD_NEOFORGE: &str = "download_neoforge";
/// Optifine 版本选择子页 ID
pub const PAGE_DOWNLOAD_OPTIFINE: &str = "download_optifine";
/// 账户页面 ID
pub const PAGE_ACCOUNTS: &str = "accounts";
/// 设置页面 ID
pub const PAGE_SETTINGS: &str = "settings";
/// Java 运行时管理页面 ID
pub const PAGE_SETTINGS_JAVA: &str = "settings_java";
/// 启动器更新页面 ID
pub const PAGE_UPDATE_LOG: &str = "update_log";
/// 游戏文件夹页面 ID
pub const PAGE_GAME_DIR: &str = "game_dir";
/// 登录方式选择页面 ID
pub const PAGE_LOGIN_METHODS: &str = "login_methods";
/// 离线登录页面 ID
pub const PAGE_LOGIN_OFFLINE: &str = "login_offline";
/// 微软登录页面 ID
pub const PAGE_LOGIN_MICROSOFT: &str = "login_microsoft";
/// Authlib 登录页面 ID
pub const PAGE_LOGIN_AUTHLIB: &str = "login_authlib";
/// Authlib 多角色选择页面 ID
pub const PAGE_LOGIN_AUTHLIB_SELECT: &str = "login_authlib_select";
/// 版本设置页面 ID
pub const PAGE_VERSION_SETTING: &str = "version_setting";
/// 版本模组页面 ID
pub const PAGE_VERSION_SETTING_MODS: &str = "version_setting_mods";
/// 版本高级设置页面 ID
pub const PAGE_VERSION_SETTING_ADVANCE: &str = "version_setting_advance";
/// 版本删除页面 ID
pub const PAGE_VERSION_SETTING_REMOVE: &str = "version_setting_remove";
/// 模组下载页面 ID
pub const PAGE_MODS_DOWNLOAD: &str = "mods_download";
/// CurseForge 搜索页面 ID
pub const PAGE_MODS_DOWNLOAD_CURSEFORGE: &str = "mods_download_curseforge";
/// Modrinth 搜索页面 ID
pub const PAGE_MODS_DOWNLOAD_MODRINTH: &str = "mods_download_modrinth";
/// 模组文件选择页面 ID
pub const PAGE_MOD_FILE_DOWNLOAD: &str = "mod_files_download";
/// 模组安装目标页面 ID
pub const PAGE_MOD_DOWNLOAD_DEST: &str = "mod_download_dest";
/// 启动错误页面 ID
pub const PAGE_LAUNCH_ERROR: &str = "launch_error";
/// 游戏启动中页面 ID
pub const PAGE_STARTING: &str = "starting";

/// 页面中的文本输入框字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextField {
    /// 版本搜索。
    VersionsFilter,
    /// 下载页的版本名称。
    DownloadVersionName,
    /// 版本设置页的重命名输入框。
    VersionRename,
    /// 版本模组搜索。
    VersionModsFilter,
    /// 账户搜索。
    AccountsFilter,
    /// 离线登录用户名。
    OfflineName,
    /// 离线登录 UUID。
    OfflineUuid,
    /// Authlib 服务器地址。
    AuthlibServer,
    /// Authlib 用户名。
    AuthlibUser,
    /// Authlib 密码。
    AuthlibPassword,
    /// CurseForge 搜索。
    CurseforgeFilter,
    /// Modrinth 搜索。
    ModrinthFilter,
    /// 全局内存。
    GlobalMemory,
    /// 全局 JVM 参数。
    JvmArgs,
    /// 全局游戏参数。
    GameArgs,
    /// 包装程序路径。
    WrapperPath,
    /// 包装程序参数。
    WrapperArgs,
    /// 版本 JVM 参数。
    VersionJvmArgs,
    /// 版本游戏参数。
    VersionGameArgs,
    /// 版本 Java 路径。
    VersionJavaPath,
    /// 版本窗口标题。
    VersionWindowTitle,
    /// 版本包装程序路径。
    VersionWrapperPath,
    /// 版本包装程序参数。
    VersionWrapperArgs,
    /// 版本内存文本。
    VersionMaxMemory,
}

/// 页面中的开关字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolField {
    /// 黑暗模式。
    DarkTheme,
    /// 页面切换动画。
    SlideAnimation,
    /// 自动调节内存。
    AutoMemory,
    /// 等待游戏窗口出现。
    WaitUntilLaunched,
    /// 启动后不关闭启动器。
    DontCloseWindow,
    /// 启动后隐藏启动器。
    HideWindow,
    /// 自动检查文件完整性。
    AutoRecheck,
    /// 使用独立版本设置。
    VersionEnable,
    /// 使用版本隔离。
    VersionIndependent,
    /// 版本手动内存。
    VersionOverrideMemory,
}

/// 页面中的单选字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceField {
    /// 版本隔离方式。
    VersionIndependentMethod,
    /// 下载源。
    DownloadSource,
}

/// 页面控件发出的消息。
#[derive(Debug, Clone)]
pub enum UiMessage {
    /// 文本输入。
    Text(TextField, String),
    /// 开关切换。
    Bool(BoolField, bool),
    /// 单选选项。
    Choice(ChoiceField, usize),
    /// 切换版本排序方式。
    ToggleSort,
    /// 折叠/展开一个版本分组。
    ToggleFolder(String),
    /// 启用/禁用一个模组。
    ToggleMod {
        /// 模组文件路径。
        path: String,
        /// 是否启用。
        enabled: bool,
    },
    /// 选择一个加载器版本并返回上一页。
    SelectLoaderVersion {
        /// 加载器类型：fabric / quiltmc / forge / neoforge / optifine。
        kind: &'static str,
        /// 版本名称。
        version: String,
    },
    /// 选择一个原版版本并返回上一页。
    SelectVanillaVersion(String),
    /// 切换设置页分页。
    SetSettingPage(usize),
    /// 设置下载并发数。
    SetParallel(usize),
    /// 复制文本到剪切板。
    CopyText(String),
    /// 打开模组下载页。
    OpenModsPage,
    /// 打开版本目录。
    OpenVersionDir(Option<&'static str>),
    /// 把当前模组安装到指定版本。
    InstallMod(String),
    /// 重新检查启动器更新。
    CheckUpdate,
}

/// 创建启动器的根视图。
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let page = match *state.pages.current() {
        PAGE_MAIN => self::main_view(state),
        PAGE_VERSION => version::view(state),
        PAGE_DOWNLOAD => download::view(state),
        PAGE_DOWNLOAD_VANILLA => download::vanilla::view(state),
        PAGE_DOWNLOAD_FABRIC => download::fabric::view(state),
        PAGE_DOWNLOAD_QUILTMC => download::quiltmc::view(state),
        PAGE_DOWNLOAD_FORGE => download::forge::view(state),
        PAGE_DOWNLOAD_NEOFORGE => download::neoforge::view(state),
        PAGE_DOWNLOAD_OPTIFINE => download::optifine::view(state),
        PAGE_ACCOUNTS => accounts::view(state),
        PAGE_SETTINGS => settings::view(state),
        PAGE_SETTINGS_JAVA => settings::java::view(state),
        PAGE_UPDATE_LOG => settings::update::view(state),
        PAGE_GAME_DIR => game_dir::view(state),
        PAGE_LOGIN_METHODS => login_methods::view(state),
        PAGE_LOGIN_OFFLINE => login::offline::view(state),
        PAGE_LOGIN_MICROSOFT => login::microsoft::view(state),
        PAGE_LOGIN_AUTHLIB => login::authlib::view(state),
        PAGE_LOGIN_AUTHLIB_SELECT => login::authlib_select::view(state),
        PAGE_VERSION_SETTING => version_setting::view(state),
        PAGE_VERSION_SETTING_MODS => version_setting_mods::view(state),
        PAGE_VERSION_SETTING_ADVANCE => version_setting_advance::view(state),
        PAGE_VERSION_SETTING_REMOVE => version_setting_remove::view(state),
        PAGE_MODS_DOWNLOAD => mods_download::view(state),
        PAGE_MODS_DOWNLOAD_CURSEFORGE => mods_download_curseforge::view(state),
        PAGE_MODS_DOWNLOAD_MODRINTH => mods_download_modrinth::view(state),
        PAGE_MOD_FILE_DOWNLOAD => mod_files_download::view(state),
        PAGE_MOD_DOWNLOAD_DEST => mod_download_dest::view(state),
        PAGE_LAUNCH_ERROR => launch_error::view(state),
        PAGE_STARTING => starting::view(state),
        _ => self::main_view(state),
    };

    let content = container(page)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(theme.window_background)),
            ..iced::widget::container::Style::default()
        });

    let show_back = state.pages.can_go_back() && state.back_enabled;
    let shell = scl_gui_widgets::widgets::window_shell(
        &theme,
        "Sharp Craft Launcher",
        show_back,
        content,
        Message::Window,
    );

    let mut root: Element<'_, Message> = shell;

    if !state.progress.is_empty() {
        let overlay = progress_overlay(state);
        root = iced::widget::stack![
            root,
            container(overlay)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Bottom)
                .align_x(iced::alignment::Horizontal::Center)
        ]
        .into();
    }

    if state.async_error.is_some() {
        let modal = async_error_modal(state);
        root = iced::widget::stack![
            root,
            container(modal)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| iced::widget::container::Style {
                    background: Some(Background::Color(iced::Color::from_rgba(
                        0.0, 0.0, 0.0, 0.45
                    ))),
                    ..iced::widget::container::Style::default()
                })
        ]
        .into();
    }

    root
}

fn main_view(state: &AppState) -> Element<'_, Message> {
    // 主页在 ui::main 中实现，这里转接以保持页面路由集中。
    crate::ui::main::view(state)
}

fn progress_overlay(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let mut entries = column![].spacing(8);

    for entry in state.progress.values() {
        let title = if entry.message.is_empty() {
            "正在处理……".to_owned()
        } else {
            entry.message.clone()
        };

        let right = if !entry.sub_message.is_empty() {
            entry.sub_message.clone()
        } else if let Some(max) = entry.max {
            format!("{}/{}", entry.current.round(), max.round())
        } else {
            String::new()
        };

        let ratio = entry
            .max
            .filter(|max| *max > 0.0)
            .map(|max| (entry.current / max).clamp(0.0, 1.0) as f32);

        let mut item = column![
            row![
                iced::widget::text(title)
                    .size(12.0)
                    .font(theme.typography.base.font)
                    .color(theme.base.high),
                space().width(Length::Fill),
                iced::widget::text(right)
                    .size(12.0)
                    .font(theme.typography.base.font)
                    .color(theme.base.high),
            ]
            .align_y(Alignment::Center)
        ]
        .spacing(4);

        if let Some(ratio) = ratio {
            item = item.push(common::progress(&theme, ratio, 4.0));
        } else {
            item = item.push(common::indeterminate(&theme, state.animation_phase, 4.0));
        }

        entries = entries.push(item);
    }

    container(entries)
        .width(Length::Fill)
        .padding([8, 12])
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(theme.chrome.medium_low)),
            border: Border {
                radius: 0.0.into(),
                width: 1.0,
                color: theme.border.transient,
            },
            shadow: Shadow::default(),
            ..iced::widget::container::Style::default()
        })
        .into()
}

fn async_error_modal(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;
    let (location, error) = state.async_error.clone().unwrap_or_default();

    let body = format!(
        "执行位于 {location} 的后台异步任务时发生错误：\n{error}\n\n\
         这个错误不会使程序崩溃，但是可能会重要，是否将此错误复制到剪切板？"
    );

    let buttons = row![
        common::button(
            &theme,
            "关闭",
            scl_gui_widgets::widgets::ButtonKind::Standard,
            Message::DismissAsyncError,
        ),
        common::button(
            &theme,
            "复制并关闭",
            scl_gui_widgets::widgets::ButtonKind::Accent,
            Message::CopyAsyncError,
        ),
    ]
    .spacing(8);

    container(
        column![
            iced::widget::text("执行异步任务发生错误")
                .size(theme.typography.subtitle_alt.size)
                .font(theme.typography.subtitle_alt.font)
                .color(theme.font_color),
            iced::widget::text(body)
                .size(theme.typography.base.size)
                .font(theme.typography.base.font)
                .color(theme.font_color),
            space().height(Length::Fixed(4.0)),
            buttons,
        ]
        .spacing(10),
    )
    .width(Length::Fixed(300.0))
    .padding(16)
    .style(move |_| iced::widget::container::Style {
        background: Some(Background::Color(theme.chrome.medium_low)),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: theme.border.transient,
        },
        shadow: Shadow {
            color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..iced::widget::container::Style::default()
    })
    .into()
}

pub mod main;
