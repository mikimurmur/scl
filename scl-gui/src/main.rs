//! Sharp Craft Launcher
//!
//! 一个轻量、快速、干净的 Minecraft 启动器。

mod app;
mod app_state;
mod config;
mod core;
mod panic;
mod theme;
mod ui;
mod utils;
mod widgets;

use druid::{AppLauncher, Color, Env, Insets, Size, WindowDesc};
use scl_gui_widgets::theme::color::{self, accent, main, Theme};

use crate::{app_state::AppState, config::Config, theme::icons::set_icon_to_env};

/// 把 tracing 的根 target 从可执行目标名改成原版 crate 名。
fn rewrite_log_target(target: &str) -> std::borrow::Cow<'_, str> {
    if let Some(rest) = target.strip_prefix("SharpCraftLauncher") {
        std::borrow::Cow::Owned(format!("sharp_craft_launcher{rest}"))
    } else {
        std::borrow::Cow::Borrowed(target)
    }
}

/// 与 tracing-subscriber 默认 `Full` 格式一致，仅重写 target 前缀。
struct SharpLogFormat;

impl<S, N> tracing_subscriber::fmt::FormatEvent<S, N> for SharpLogFormat
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        let meta = event.metadata();
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.6fZ");
        if writer.has_ansi_escapes() {
            write!(writer, "\x1b[2m{}\x1b[0m ", timestamp)?;
            let color = match *meta.level() {
                tracing::Level::ERROR => "\x1b[31m",
                tracing::Level::WARN => "\x1b[33m",
                tracing::Level::INFO => "\x1b[32m",
                tracing::Level::DEBUG => "\x1b[34m",
                tracing::Level::TRACE => "\x1b[35m",
            };
            write!(writer, "{}{:>5}\x1b[0m ", color, meta.level())?;
            write!(
                writer,
                "\x1b[2m{}\x1b[0m\x1b[2m:\x1b[0m ",
                rewrite_log_target(meta.target())
            )?;
        } else {
            write!(
                writer,
                "{} {:>5} {}: ",
                timestamp,
                meta.level(),
                rewrite_log_target(meta.target())
            )?;
        }
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        // 以更新器身份运行：等待并覆盖目标程序后重启新版本
        Some("--update") => {
            if let Err(err) = utils::update::run_updater(&args[1..]) {
                eprintln!("更新失败：{err:?}");
                std::process::exit(1);
            }
            return;
        }
        // 新版本启动时清理更新残留的临时文件，然后重新以正常模式启动
        Some("--post-update") => {
            if let Err(err) = utils::update::run_post_updater(&args[1..]) {
                eprintln!("清理更新临时文件失败：{err:?}");
            }
            if let Ok(exe) = std::env::current_exe() {
                if std::process::Command::new(exe).spawn().is_ok() {
                    return;
                }
            }
        }
        _ => {}
    }

    if let Err(err) = run() {
        eprintln!("启动器运行失败：{err:?}");
        std::process::exit(1);
    }
}

/// 端到端测试钩子的硬性 HOME 隔离防护。
///
/// 只要环境里存在任一测试驱动变量（前缀 `SCL_MOCK`、`SCL_AUTO_`，或
/// `SCL_AUTOLAUNCH` / `SCL_START_PAGE`），就要求 `HOME` 指向 `/tmp` 下的
/// 隔离目录，否则直接报错退出。背景：e2e 曾两次因未隔离 HOME 把真实
/// `~/.scl/auth_data.json` 覆盖/删除（该文件由账户双轨保存链写入
/// `$HOME/.scl/`，且主配置在 CWD，家目录没有 .scl.json 不代表没有账户）。
/// 不依赖人工纪律，在进程入口（读配置之前）强制拦截。
fn ensure_e2e_home_isolated() {
    let is_test_driver = |key: &str| {
        key.starts_with("SCL_MOCK")
            || key.starts_with("SCL_AUTO_")
            || key == "SCL_AUTOLAUNCH"
            || key == "SCL_START_PAGE"
    };
    let triggered =
        std::env::vars_os().any(|(key, _)| key.to_str().map(is_test_driver).unwrap_or(false));
    if !triggered {
        return; // 正常用户运行不受影响
    }
    let home = std::env::var_os("HOME").unwrap_or_default();
    let home = std::path::PathBuf::from(&home);
    if home.starts_with("/tmp") {
        return; // 已隔离
    }
    eprintln!(
        "拒绝运行：检测到测试驱动环境变量（SCL_MOCK*/SCL_AUTO_*/SCL_AUTOLAUNCH/SCL_START_PAGE），\
         但 HOME={home:?} 不在 /tmp 下。端到端测试必须把 HOME 指向临时沙箱\
         （例如 HOME=/tmp/scl-e2e/xxx），否则测试会覆盖真实 ~/.scl/auth_data.json。"
    );
    std::process::exit(1);
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 必须在读配置之前执行：测试钩子驱动的运行会在启动早期触发保存链
    //（登录钩子 → update_or_create_auth → save → $HOME/.scl/auth_data.json）。
    ensure_e2e_home_isolated();

    // Windows 下从命令行启动时，把日志输出接到父控制台
    #[cfg(target_os = "windows")]
    let console = utils::console::attach_parent_console();

    #[cfg(target_os = "windows")]
    let ansi = console
        .as_ref()
        .map(|info| info.mode & utils::console::ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0)
        .unwrap_or(false);
    #[cfg(not(target_os = "windows"))]
    let ansi = true;

    let fmt = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_ansi(ansi)
        // 原版 crate 名为 `sharp_craft_launcher`，而本恢复版的可执行目标名是
        // `SharpCraftLauncher`，默认 target 会带上前者的首字母大写形式。
        // 这里只重写日志前缀，日志文案与字段保持不变。
        .event_format(SharpLogFormat);
    fmt.init();
    #[cfg(target_os = "windows")]
    if let Some(info) = console.as_ref() {
        tracing::debug!(
            "已连接父控制台：模式 {:#x}，输出代码页 {}",
            info.mode,
            info.output_cp
        );
    }

    // 与原版一致的启动横幅与流程日志
    tracing::warn!("============================================================");
    tracing::warn!("警告：你正在查阅启动器的日志输出信息！仅供调试使用！");
    tracing::warn!("      其中有可能输出包含你帐户信息的个人隐私内容！");
    tracing::warn!("      除非你清楚你在做什么，否则绝对不可以将日志分享给他人！");
    tracing::warn!("============================================================");
    tracing::info!(
        "Sharp Craft Launcher {} 正在运行！",
        env!("CARGO_PKG_VERSION")
    );
    tracing::info!("构建日期： {}", env!("SCL_BUILD_DATE"));
    tracing::info!("正在初始化");

    // GTK 的辅助功能桥在 at-spi bus 不存在时会为每个进程打印一条
    // `dbind-WARNING: Couldn't connect/register with accessibility bus`。
    // 这里只在检测不到 bus 时关闭辅助功能桥，其它 GTK 警告仍会正常输出。
    disable_accessibility_bridge_if_unavailable();

    tracing::info!("正在初始化Panic回调");
    panic::install();
    tracing::info!("正在创建窗口");

    let mut config = match config::get_config_sync() {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("读取配置文件失败，将使用默认配置：{err:?}");
            config::Config::default()
        }
    };
    // 启动顺序：先读 ./.scl.json，再用 ~/.scl/auth_data.json 覆盖账户数据；
    // auth 文件存在即以它为准。
    config::apply_auth_override(&mut config, config::load_auth_data());
    tracing::info!("正在初始化数据状态");
    let first_run = !config::config_path().exists();
    let mut state = AppState::new(config);
    state.first_run = first_run;
    if std::env::var_os("SCL_MOCK").is_some() {
        app_state::apply_mock(&mut state);
    }

    tracing::info!("后台任务将按需创建线程");
    utils::thread::ensure_background_thread();
    tracing::info!("创建窗口中");

    AppLauncher::with_window(gen_win())
        .configure_env(|env, state| config_env(&state.config, env))
        .launch(state)?;

    tracing::info!("退出中");
    Ok(())
}

/// 生成主窗口
fn gen_win() -> WindowDesc<AppState> {
    let height = if std::env::var_os("SCL_MOCK_TALL").is_some() {
        1000.
    } else {
        420.
    };
    WindowDesc::new(ui::root())
        .title("Sharp Craft Launcher")
        .window_size(Size::new(300., height))
        .with_min_size(Size::new(300., 300.))
        // 固定窗口且不显示标题栏，因此 KWin/GTK 标题栏不提供最大化按钮。
        .resizable(false)
        .show_titlebar(false)
}

/// 在 at-spi 辅助功能总线不可用时关闭 GTK 的辅助功能桥
///
/// GTK 在启动时会尝试向 `$XDG_RUNTIME_DIR/at-spi/bus_0` 注册，失败时通过
/// `dbind` 打印 `Gtk-WARNING`/`dbind-WARNING`。这里只在该 socket 确实不存在时
/// 设置 `NO_AT_BRIDGE=1`，不会屏蔽其它 GTK 警告。
#[cfg(unix)]
fn disable_accessibility_bridge_if_unavailable() {
    if std::env::var_os("NO_AT_BRIDGE").is_some() || std::env::var_os("GTK_A11Y").is_some() {
        return;
    }
    // 即使 socket 文件存在，at-spi 也可能没有响应并让 GTK 打印同样的
    // dbind-WARNING；启动器不需要辅助功能树，因此直接关闭该桥。
    std::env::set_var("NO_AT_BRIDGE", "1");
}

#[cfg(not(unix))]
fn disable_accessibility_bridge_if_unavailable() {}

/// 根据配置设置 Druid 的环境
pub(crate) fn config_env(config: &Config, env: &mut Env) {
    let theme = if config.theme.dark_theme {
        Theme::Dark
    } else {
        Theme::Light
    };
    color::set_color_to_env(env, theme);
    set_icon_to_env(env, &config.theme.icons_theme);
    set_env_color(config, env);
}

/// 应用自定义的主题颜色并补齐 Druid 自带控件使用的主题变量
fn set_env_color(config: &Config, env: &mut Env) {
    // v1.2.0 的默认主题以品牌橙色作为强调色。`scl-gui-widgets` 中保留的
    // 蓝色 Fluent 强调色只作为库的默认值，启动器需要在没有自定义颜色时覆盖掉。
    let default_accent = Color::Rgba32(0xF74C00FF);
    let default_accent_light = Color::Rgba32(0xFF7233FF);
    let default_accent_dark = Color::Rgba32(0xD24100FF);
    env.set(accent::ACCENT, default_accent);
    env.set(accent::ACCENT_1, default_accent_light);
    env.set(accent::ACCENT_DARK_1, default_accent_dark);
    env.set(accent::ACCENT_LIGHT_1, default_accent_light);

    if let Some(color) = config.theme.theme_color_secondary {
        env.set(main::SECONDARY, color);
    }
    if let Some(color) = config.theme.theme_color_primary {
        env.set(main::PRIMARY, color);
    }
    if let Some(color) = config.theme.theme_color_accent {
        env.set(accent::ACCENT, color);
    }
    if let Some(color) = config.theme.theme_color_accent_1 {
        env.set(accent::ACCENT_1, color);
    }
    if let Some(color) = config.theme.theme_color_accent_dark_1 {
        env.set(accent::ACCENT_DARK_1, color);
    }
    if let Some(color) = config.theme.theme_color_accent_light_1 {
        env.set(accent::ACCENT_LIGHT_1, color);
    }
    if let Some(color) = config.theme.theme_color_title_bar {
        env.set(main::TITLE_BAR, color);
    }
    if let Some(color) = config.theme.theme_color_background {
        env.set(druid::theme::WINDOW_BACKGROUND_COLOR, color);
    }

    env.set(druid::theme::SCROLLBAR_WIDTH, 2.0);
    env.set(druid::theme::SCROLLBAR_COLOR, Color::Rgba32(0x7A7A7AFF));
    env.set(
        druid::theme::SCROLLBAR_BORDER_COLOR,
        Color::Rgba32(0x7A7A7AFF),
    );
    env.set(druid::theme::BUTTON_BORDER_RADIUS, 2.0);
    env.set(druid::theme::BUTTON_BORDER_WIDTH, 0.0);
    env.set(druid::theme::BUTTON_DARK, Color::Rgba32(0xC6C6C6FF));
    env.set(druid::theme::BUTTON_LIGHT, Color::Rgba32(0xE0E0E0FF));
    env.set(
        druid::theme::TEXT_COLOR,
        env.get(scl_gui_widgets::theme::color::base::HIGH),
    );
    env.set(druid::theme::TEXTBOX_INSETS, Insets::new(12., 6., 12., 6.));
    env.set(druid::theme::PRIMARY_LIGHT, default_accent);
    env.set(druid::theme::PRIMARY_DARK, default_accent);
    env.set(druid::theme::FOREGROUND_LIGHT, default_accent);
    env.set(druid::theme::FOREGROUND_DARK, default_accent);
    env.set(
        druid::theme::BACKGROUND_LIGHT,
        env.get(scl_gui_widgets::theme::color::alt::HIGH),
    );
    env.set(
        druid::theme::BACKGROUND_DARK,
        env.get(scl_gui_widgets::theme::color::alt::HIGH),
    );
    env.set(
        druid::theme::PLACEHOLDER_COLOR,
        druid::Color::rgb8(0x80, 0x80, 0x80),
    );
    env.set(
        druid::theme::SELECTED_TEXT_BACKGROUND_COLOR,
        env.get(accent::ACCENT),
    );
    env.set(
        druid::theme::CURSOR_COLOR,
        env.get(scl_gui_widgets::theme::color::base::HIGH),
    );
    env.set(druid::theme::TEXTBOX_BORDER_WIDTH, 1.0);
    env.set(druid::theme::TEXTBOX_BORDER_RADIUS, 4.0);
}
