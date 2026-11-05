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

use druid::{widget::EnvScope, Widget};
use scl_gui_widgets::widgets::{PageSwitcher, ProgressOverlay, WindowWidget};
pub use scl_gui_widgets::widgets::{
    ON_PAGE, PUSH_PAGE, QUERY_POP_PAGE, RESET_PAGE_TO, SET_SLIDE_PAGE_ANIMATION,
};

use crate::{app::AppWrapper, app_state::AppState};

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

/// 创建启动器的根组件
pub fn root() -> Box<dyn Widget<AppState>> {
    let pages = PageSwitcher::new()
        .with_page(PAGE_MAIN, Box::new(|| main::ui_builder()))
        .with_page(PAGE_VERSION, Box::new(|| version::ui_builder()))
        .with_page(PAGE_DOWNLOAD, Box::new(|| download::ui_builder()))
        .with_page(
            PAGE_DOWNLOAD_VANILLA,
            Box::new(|| download::vanilla::ui_builder()),
        )
        .with_page(
            PAGE_DOWNLOAD_FABRIC,
            Box::new(|| download::fabric::ui_builder()),
        )
        .with_page(
            PAGE_DOWNLOAD_QUILTMC,
            Box::new(|| download::quiltmc::ui_builder()),
        )
        .with_page(
            PAGE_DOWNLOAD_FORGE,
            Box::new(|| download::forge::ui_builder()),
        )
        .with_page(
            PAGE_DOWNLOAD_NEOFORGE,
            Box::new(|| download::neoforge::ui_builder()),
        )
        .with_page(
            PAGE_DOWNLOAD_OPTIFINE,
            Box::new(|| download::optifine::ui_builder()),
        )
        .with_page(PAGE_ACCOUNTS, Box::new(|| accounts::ui_builder()))
        .with_page(PAGE_SETTINGS, Box::new(|| settings::ui_builder()))
        .with_page(
            PAGE_SETTINGS_JAVA,
            Box::new(|| settings::java::ui_builder()),
        )
        .with_page(PAGE_UPDATE_LOG, Box::new(|| settings::update::ui_builder()))
        .with_page(PAGE_GAME_DIR, Box::new(|| game_dir::ui_builder()))
        .with_page(PAGE_LOGIN_METHODS, Box::new(|| login_methods::ui_builder()))
        .with_page(
            PAGE_LOGIN_OFFLINE,
            Box::new(|| login::offline::ui_builder()),
        )
        .with_page(
            PAGE_LOGIN_MICROSOFT,
            Box::new(|| login::microsoft::ui_builder()),
        )
        .with_page(
            PAGE_LOGIN_AUTHLIB,
            Box::new(|| login::authlib::ui_builder()),
        )
        .with_page(
            PAGE_LOGIN_AUTHLIB_SELECT,
            Box::new(|| login::authlib_select::ui_builder()),
        )
        .with_page(
            PAGE_VERSION_SETTING,
            Box::new(|| version_setting::ui_builder()),
        )
        .with_page(
            PAGE_VERSION_SETTING_MODS,
            Box::new(|| version_setting_mods::ui_builder()),
        )
        .with_page(
            PAGE_VERSION_SETTING_ADVANCE,
            Box::new(|| version_setting_advance::ui_builder()),
        )
        .with_page(
            PAGE_VERSION_SETTING_REMOVE,
            Box::new(|| version_setting_remove::ui_builder()),
        )
        .with_page(PAGE_MODS_DOWNLOAD, Box::new(|| mods_download::ui_builder()))
        .with_page(
            PAGE_MODS_DOWNLOAD_CURSEFORGE,
            Box::new(|| mods_download_curseforge::ui_builder()),
        )
        .with_page(
            PAGE_MODS_DOWNLOAD_MODRINTH,
            Box::new(|| mods_download_modrinth::ui_builder()),
        )
        .with_page(
            PAGE_MOD_FILE_DOWNLOAD,
            Box::new(|| mod_files_download::ui_builder()),
        )
        .with_page(
            PAGE_MOD_DOWNLOAD_DEST,
            Box::new(|| mod_download_dest::ui_builder()),
        )
        .with_page(PAGE_LAUNCH_ERROR, Box::new(|| launch_error::ui_builder()))
        .with_page(PAGE_STARTING, Box::new(|| starting::ui_builder()));

    let mock_pages: Vec<&'static str> = std::env::var("SCL_MOCK_PAGE")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|page| !page.is_empty())
        .map(|page| Box::leak(page.to_owned().into_boxed_str()) as &'static str)
        .collect();
    // SCL_MOCK_AUTOSTART：窗口就绪后自动提交启动命令（E2E 用，输入管道不可用
    // 时也能走完整启动流程），与其它 SCL_MOCK_* 测试挂钩同一模式。
    let mock_autostart = std::env::var_os("SCL_MOCK_AUTOSTART").is_some();

    let pages: Box<dyn Widget<AppState>> = if mock_pages.is_empty() && !mock_autostart {
        Box::new(pages)
    } else {
        Box::new(MockNavigator::new(pages, mock_pages, mock_autostart))
    };

    let window = EnvScope::new(
        |env, data: &AppState| crate::config_env(&data.config, env),
        WindowWidget::new("Sharp Craft Launcher", pages),
    );

    Box::new(AppWrapper::new(
        ProgressOverlay::new(window).show_progress_on_taskbar(),
    ))
}

/// 无交互逐页截图用的页面驱动器：按 [`SCL_MOCK_PAGE`] 给出的顺序依次推送页面。
struct MockNavigator {
    child: Box<dyn Widget<AppState>>,
    pages: Vec<&'static str>,
    next: usize,
    autostart: bool,
}

impl MockNavigator {
    fn new(
        child: impl Widget<AppState> + 'static,
        pages: Vec<&'static str>,
        autostart: bool,
    ) -> Self {
        Self {
            child: Box::new(child),
            pages,
            next: 0,
            autostart,
        }
    }
}

impl Widget<AppState> for MockNavigator {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut AppState,
        env: &druid::Env,
    ) {
        if self.autostart && matches!(event, druid::Event::WindowConnected) {
            self.autostart = false;
            ctx.submit_command(crate::app::START_GAME.to(druid::Target::Global));
        }
        if matches!(
            event,
            druid::Event::WindowConnected | druid::Event::Timer(_)
        ) && self.next < self.pages.len()
        {
            tracing::debug!(
                "test hook SCL_MOCK_PAGE: auto push {:?}",
                self.pages[self.next]
            );
            ctx.submit_command(
                PUSH_PAGE
                    .with(self.pages[self.next])
                    .to(druid::Target::Global),
            );
            self.next += 1;
            ctx.request_timer(std::time::Duration::from_millis(150));
        }
        self.child.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &AppState,
        env: &druid::Env,
    ) {
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        old_data: &AppState,
        data: &AppState,
        env: &druid::Env,
    ) {
        self.child.update(ctx, old_data, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &AppState,
        env: &druid::Env,
    ) -> druid::Size {
        self.child.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &AppState, env: &druid::Env) {
        self.child.paint(ctx, data, env);
    }

    fn id(&self) -> Option<druid::WidgetId> {
        self.child.id()
    }
}

pub mod main;
