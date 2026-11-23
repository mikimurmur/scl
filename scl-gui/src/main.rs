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

use iced::Size;

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

    tracing::info!("正在初始化恐慌回调");
    panic::install();
    tracing::info!("正在创建窗口");

    let height = if std::env::var_os("SCL_MOCK_TALL").is_some() {
        1000.0
    } else {
        420.0
    };

    tracing::info!("正在启动后台线程");
    utils::thread::ensure_background_thread();

    iced::application(
        app::AppWrapper::boot,
        app::AppWrapper::update,
        app::AppWrapper::view,
    )
    .title("Sharp Craft Launcher")
    .theme(app::AppWrapper::theme)
    .subscription(app::AppWrapper::subscription)
    .window(iced::window::Settings {
        size: Size::new(300.0, height),
        min_size: Some(Size::new(300.0, 300.0)),
        resizable: false,
        decorations: false,
        transparent: false,
        exit_on_close_request: false,
        ..iced::window::Settings::default()
    })
    .exit_on_close_request(false)
    .run()?;

    tracing::info!("退出中");
    Ok(())
}
