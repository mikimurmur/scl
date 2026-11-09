//! 启动器的应用逻辑。
//!
//! 所有界面之间共享的逻辑以及后台任务都放在这里。

pub mod download;
pub mod java;
pub mod login_method_microsoft;
pub mod mod_download_dest;
pub mod mods_download_curseforge;
pub mod mods_download_modrinth;
pub mod settings;
pub mod starting;
pub mod version_setting;

use std::path::PathBuf;

use druid::{
    commands,
    piet::{PaintBrush, Text, TextLayout, TextLayoutBuilder},
    Command, Event, EventCtx, LifeCycle, LifeCycleCtx, RenderContext, Selector, Target, Widget,
    WidgetPod,
};
use im::Vector;
use scl_core::{auth::generate_offline_uuid, download::AuthlibDownloadExt};

use crate::{
    app_state::{
        AppState, DownloadState, LocalMod, LocalVersionInfo, ModFileState, ModState, NetworkData,
    },
    config,
    core::{AuthMethod, CoreTransformer, SCLLaunchConfig, VersionInfo},
    ui::{
        PAGE_ACCOUNTS, PAGE_DOWNLOAD, PAGE_LAUNCH_ERROR, PAGE_LOGIN_AUTHLIB,
        PAGE_LOGIN_AUTHLIB_SELECT, PAGE_LOGIN_MICROSOFT, PAGE_LOGIN_OFFLINE, PAGE_MAIN,
        PAGE_MODS_DOWNLOAD_CURSEFORGE, PAGE_MODS_DOWNLOAD_MODRINTH, PAGE_MOD_DOWNLOAD_DEST,
        PAGE_STARTING, PAGE_VERSION_SETTING, PAGE_VERSION_SETTING_MODS,
    },
    utils::{thread::thread_main, Progress},
};

/// 从界面上选择了一个游戏版本
pub const SELECT_VERSION: Selector<String> = Selector::new("net.stevexmh.scl.select-version");
/// 打开一个游戏版本的设置页面
pub const SELECT_VERSION_SETTING: Selector<String> =
    Selector::new("net.stevexmh.scl.select-version-setting");
/// 保存游戏版本设置
pub const SAVE_VERSION_SETTING: Selector<()> =
    Selector::new("net.stevexmh.scl.save-version-setting");
/// 删除游戏版本
pub const REMOVE_VERSION: Selector<()> = Selector::new("net.stevexmh.scl.remove-version");
/// 切换账户
pub const SWITCH_AUTH: Selector<usize> = Selector::new("net.stevexmh.scl.switch-auth");
/// 删除账户
pub const DELETE_AUTH: Selector<usize> = Selector::new("net.stevexmh.scl.delete-auth");
/// 选择 Java 运行时（官方携带整条 JavaRuntimeInfo）
pub const SELECT_JAVA: Selector<crate::config::JavaRuntimeInfo> =
    Selector::new("net.stevexmh.scl.select-java");
/// 删除 Java 运行时（官方携带整条 JavaRuntimeInfo）
pub const DELETE_JAVA: Selector<crate::config::JavaRuntimeInfo> =
    Selector::new("net.stevexmh.scl.delete-java");
/// 手动添加 Java 运行时（作为文件选择器的 accept command，携带 FileInfo）
pub const MANUALLY_ADD_JAVA: Selector<druid::FileInfo> =
    Selector::new("net.stevexmh.scl.manually-add-java");
/// 设置内存大小
pub const SET_MEMORY_STRING: Selector<String> = Selector::new("net.stevexmh.scl.set-memory-str");
/// 添加 Minecraft 文件夹
pub const ADD_MC_FOLDER: Selector<druid::FileInfo> =
    Selector::new("net.stevexmh.scl.add-mc-folder");
/// 删除 Minecraft 文件夹
pub const DEL_MC_FOLDER: Selector<usize> = Selector::new("net.stevexmh.scl.del-mc-folder");
/// 选择 Minecraft 文件夹
pub const SEL_MC_FOLDER: Selector<usize> = Selector::new("net.stevexmh.scl.sel-mc-folder");
/// 原版版本选择
pub const VANILLA_DOWNLOAD_SELECT: Selector<String> =
    Selector::new("net.stevexmh.scl.vanilla-download-select");
/// Fabric 版本选择
pub const FABRIC_DOWNLOAD_SELECT: Selector<String> =
    Selector::new("net.stevexmh.scl.fabric-download-select");
/// QuiltMC 版本选择
pub const QUILTMC_DOWNLOAD_SELECT: Selector<String> =
    Selector::new("net.stevexmh.scl.quiltmc-download-select");
/// Forge 版本选择
pub const FORGE_DOWNLOAD_SELECT: Selector<String> =
    Selector::new("net.stevexmh.scl.forge-download-select");
/// NeoForge 版本选择
pub const NEOFORGE_DOWNLOAD_SELECT: Selector<String> =
    Selector::new("net.stevexmh.scl.neoforge-download-select");
/// Optifine 版本选择
pub const OPTIFINE_DOWNLOAD_SELECT: Selector<String> =
    Selector::new("net.stevexmh.scl.optifine-download-select");
/// 切换下载页面的加载器分页
pub const SET_DOWNLOAD_TAB: Selector<usize> = Selector::new("net.stevexmh.scl.set-download-tab");
/// 下载游戏
pub const DOWNLOAD_GAME: Selector<DownloadRequest> =
    Selector::new("net.stevexmh.scl.download-game");
/// 启动游戏
pub const START_GAME: Selector<()> = Selector::new("net.stevexmh.scl.start-game");
/// 新增版本完成后通知版本列表刷新
pub const ADD_VERSION: Selector<()> = Selector::new("net.stevexmh.scl.add-version");
/// 离线登录
pub const LOGIN_OFFLINE: Selector<()> = Selector::new("net.stevexmh.scl.login-offline");
/// Authlib 登录
pub const LOGIN_AUTHLIB: Selector<()> = Selector::new("net.stevexmh.scl.login-authlib");
/// 选择 Authlib 角色
pub const SELECT_AUTHLIB_CHARACTER: Selector<usize> =
    Selector::new("net.stevexmh.scl.select-authlib-character");
/// 微软登录
pub const LOGIN_MICROSOFT: Selector<()> = Selector::new("net.stevexmh.scl.login-microsoft");
/// 检查更新
pub const CHECK_UPDATE: Selector<()> = Selector::new("net.stevexmh.scl.check-update");
/// 立即更新
pub const UPDATE_NOW: Selector<()> = Selector::new("net.stevexmh.scl.update-now");
/// 修复版本文件（全局扫描）
pub const FIX_MINECRAFT: Selector<()> = Selector::new("net.stevexmh.scl.fix-minecraft");
/// 修复当前打开版本的版本文件（版本级）
pub const FIX_VERSION: Selector<()> = Selector::new("net.stevexmh.scl.fix-version");
/// 打开链接
pub const OPEN_URL: Selector<String> = Selector::new("net.stevexmh.scl.open-url");
/// 保存配置
pub const SAVE_CONFIG: Selector<()> = Selector::new("net.stevexmh.scl.save-config");
/// 启用/禁用模组
pub const TOGGLE_MOD: Selector<String> = Selector::new("net.stevexmh.scl.toggle-mod");
/// 删除模组
pub const DELETE_MOD: Selector<String> = Selector::new("net.stevexmh.scl.delete-mod");
/// 从版本设置页面下载模组
pub const INSTALL_MOD: Selector<String> = Selector::new("net.stevexmh.scl.install-mod");
/// 保存模组到指定路径
pub const SAVE_MOD: Selector<druid::FileInfo> = Selector::new("net.stevexmh.scl.save-mod");
/// 设置模组文件列表
pub const SET_MOD_FILES: Selector<()> = Selector::new("net.stevexmh.scl.set-mod-files");
/// 模组下载完成
pub const MOD_DOWNLOADED: Selector<String> = Selector::new("net.stevexmh.scl.mod-downloaded");
/// CurseForge 搜索
pub const CF_APPLY_SEARCH: Selector<()> = Selector::new("net.stevexmh.scl.cf.apply-search");
/// CurseForge 加载下一页
pub const CF_LOAD_MORE: Selector<()> = Selector::new("net.stevexmh.scl.cf.load-more-page");
/// CurseForge 选择模组
pub const CF_SELECT_MOD: Selector<ModState> = Selector::new("net.stevexmh.scl.cf.select-mod");
/// Modrinth 搜索
pub const MR_APPLY_SEARCH: Selector<()> = Selector::new("net.stevexmh.scl.mr.apply-search");
/// Modrinth 加载下一页
pub const MR_LOAD_MORE: Selector<()> = Selector::new("net.stevexmh.scl.mr.load-more-page");
/// Modrinth 选择模组
pub const MR_SELECT_MOD: Selector<ModState> = Selector::new("net.stevexmh.scl.mr.select-mod");
/// 从模组文件列表中选择一个文件并进入安装目标页面
pub const CF_DOWNLOAD_MOD: Selector<ModFileState> =
    Selector::new("net.stevexmh.scl.cf.download-mod");
/// 选择模组安装目标版本
pub const SELECT_MOD_INSTALL_VERSION: Selector<String> =
    Selector::new("net.stevexmh.scl.select-mod-install-version");

/// 后台任务返回的事件
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// 扫描到所有本地版本
    VersionsLoaded(Vec<LocalVersionInfo>),
    /// 扫描到所有 Java 运行时（含无法解析的候选数量）
    JavaLoaded {
        /// 成功识别的运行时
        javas: Vec<crate::config::JavaRuntimeInfo>,
        /// 无法解析的候选数量
        failed: usize,
    },
    /// 添加了一个 Java 运行时
    JavaAdded(crate::config::JavaRuntimeInfo),
    /// 原版版本列表加载完成
    VanillaLoaded(Result<Vec<VersionInfo>, String>),
    /// 模组加载器版本列表加载完成
    LoaderLoaded {
        /// 加载器类型
        kind: String,
        /// 版本列表以及该版本是否为测试版本
        result: Result<Vec<(String, bool)>, String>,
    },
    /// 添加了一个账户
    AuthAdded(AuthMethod),
    /// Authlib 服务器返回的角色列表
    AuthCharacters(Vec<AuthMethod>),
    /// 登录失败
    AuthFailed(String),
    /// 微软登录的设备码
    MicrosoftCode {
        /// 用户需要输入的代码
        code: String,
        /// 验证链接
        url: String,
    },
    /// 微软登录的设备码验证成功
    MicrosoftSuccess,
    /// 更新检查完成
    UpdateChecked(Result<(String, String), String>),
    /// 自更新流程结束，成功则应退出启动器
    UpdateFinished(Result<(), String>),
    /// 版本设置加载完成（仅含版本配置；模组与计数由页面推送后的
    /// `on_app_push_page` 后台填充，与官方时序一致）
    VersionConfigLoaded {
        /// 版本名称
        name: String,
        /// 版本设置
        config: SCLLaunchConfig,
        /// 版本类型
        version_type: u8,
        /// 地图菜单行描述（推页前已计数完成）
        maps_desc: String,
        /// 资源包菜单行描述（推页前已计数完成）
        resource_packs_desc: String,
    },
    /// 版本设置在页面弹出时保存完成
    VersionSettingSaved(String),
    /// 版本级修复完成
    VersionFixFinished(Result<usize, String>),
    /// 版本模组列表加载完成（页面推送后由 `on_app_push_page` 发送）
    VersionModsLoaded {
        /// 发起请求时的版本名，用于丢弃迟到回调
        version_name: String,
        /// 发起请求时的加载 token，用于丢弃迟到回调
        token: u64,
        /// 发起请求时的失效纪元；写入缓存后据此判断是否已过期
        cache_epoch: u64,
        /// 版本模组
        mods: Vec<LocalMod>,
        /// 模组菜单行描述
        mods_desc: String,
    },
    /// 游戏下载完成
    DownloadFinished(Result<String, String>),
    /// 游戏启动失败
    LaunchFailed(String),
    /// 游戏进程输出的一行日志
    LogLine(String),
    /// 首次启动初始化完成，携带更新后的配置
    ///
    /// 配置体积远大于其它事件载荷，装箱以避免整个枚举被撑大。
    InitFinished(Box<crate::config::Config>),
    /// 玩家皮肤获取完成
    AccountSkinLoaded {
        uuid: String,
        head: Vec<u8>,
        hat: Vec<u8>,
    },
    /// 首次启动失败后文件完整性检查完成，可以重新启动
    RecheckRetry,
    /// 游戏进程已启动（spawn 成功）
    GameStarted(u32),
    /// 游戏窗口已出现（或进程以 0 退出），可以隐藏/关闭启动器或回主页
    GameWindowReady,
    /// 游戏进程以非零返回值退出（启动后崩溃）
    GameCrashed(Option<i32>),
    /// Modrinth 搜索结果
    ///
    /// 官方分发器只有成功载荷通路：搜索任务失败（网络/JSON 解析）时
    /// 直接走后台任务错误回调，弹「执行后台任务发生错误」窗，不向界面提交命令。
    /// `generation` 用于丢弃迟到回调（官方以 SEARCH_TIMER 时刻比较）。
    ModrinthLoaded {
        /// 搜索结果
        mods: Vec<ModState>,
        /// 发起请求时的页码
        page: u64,
        /// 发起请求时的搜索代次
        generation: u64,
    },
    /// CurseForge 搜索结果（同上，失败走后台错误弹窗）
    CurseforgeLoaded {
        /// 搜索结果
        mods: Vec<ModState>,
        /// 发起请求时的页码
        page: u64,
        /// 发起请求时的搜索代次
        generation: u64,
    },
    /// 模组图标加载完成
    ModIconLoaded {
        /// 模组来源
        source: u8,
        /// 模组 ID
        id: String,
        /// 图标（ASCII85 编码字符串）
        icon: String,
    },
    /// 模组中文名加载完成
    ModCnameLoaded {
        /// 模组来源
        source: u8,
        /// 匹配键（模组 slug）
        key: String,
        /// 中文名（gitee scl-data 的 MCMOD 译名）
        cname: String,
    },
    /// 模组文件列表加载完成（失败走后台错误弹窗，不提交本事件）
    ModFilesLoaded {
        /// 文件列表
        files: Vec<ModFileState>,
        /// 发起请求时的代次（用于丢弃迟到回调）
        generation: u64,
    },
    /// 模组下载完成
    ModDownloaded(Result<String, String>),
}

/// 后台任务向界面发送事件时使用的选择器
pub const APP_EVENT: Selector<AppEvent> = Selector::new("net.stevexmh.scl.app-event");

/// 下载一个游戏版本的请求
#[derive(Debug, Clone)]
pub struct DownloadRequest {
    /// 安装后的版本名称
    pub version_name: String,
    /// 原版版本
    pub vanilla: VersionInfo,
    /// Fabric 加载器版本
    pub fabric: String,
    /// QuiltMC 加载器版本
    pub quiltmc: String,
    /// Forge 加载器版本
    pub forge: String,
    /// NeoForge 加载器版本
    pub neoforge: String,
    /// Optifine 版本，格式为 `类型 补丁号`
    pub optifine: String,
}

impl DownloadRequest {
    /// 创建一个仅包含原版的下载请求
    pub fn vanilla(version: VersionInfo) -> Self {
        Self {
            version_name: version.name.to_owned(),
            vanilla: version,
            fabric: String::new(),
            quiltmc: String::new(),
            forge: String::new(),
            neoforge: String::new(),
            optifine: String::new(),
        }
    }
}

/// 设置项修改后的保存防抖代数
static CONFIG_SAVE_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 仅调试用：记录自动（非用户点击）页面切换命令的来源。
/// 排查「无输入自发换页」时配合 `RUST_LOG=sharp_craft_launcher=debug` 使用。
fn trace_auto_page(action: &str, page: &str, origin: &'static str) {
    tracing::debug!("auto page {action}: page={page:?}, origin={origin}");
}

/// Esc 触发返回的条件：返回按钮可用、按的是 Esc、且不是按住不放产生的自动重复。
///
/// 忽略 `repeat` 保证按住 Esc 不会连续退掉多层页面。
fn should_back_on_key(back_page_enabled: bool, code: druid::Code, repeat: bool) -> bool {
    back_page_enabled && code == druid::Code::Escape && !repeat
}

/// 启动失败后是否安排自动修复 + 重试。
///
/// 只看三个条件：配置开关、当前启动周期是否已重试过、游戏窗口是否曾经出现。
fn should_schedule_auto_repair(
    enabled: bool,
    retry_latch: bool,
    launch_window_ready: bool,
) -> bool {
    enabled && !retry_latch && !launch_window_ready
}

/// 将配置保存到磁盘
pub fn save_config(data: &AppState) {
    let cfg = data.config.to_owned();
    thread_main(move || config::save_config(&cfg));
}

/// 设置项发生变化后延迟保存：连续拖动滑块/输入时只落盘最后一次状态。
fn save_config_debounced(data: &AppState) {
    use std::sync::atomic::Ordering;
    let cfg = data.config.to_owned();
    let generation = CONFIG_SAVE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    thread_main(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if CONFIG_SAVE_GENERATION.load(Ordering::SeqCst) == generation {
            if let Err(err) = config::save_config(&cfg) {
                tracing::error!("保存配置文件失败：{err:?}");
            }
        }
        Ok(())
    });
}

/// 将账户添加到配置中
pub fn update_or_create_auth(ctx: &EventCtx, data: &mut AppState, auth: AuthMethod) {
    let uuid = auth.uuid().to_owned();
    let need_skin = !matches!(auth, crate::core::AuthMethod::Offline { .. })
        && auth.head_skin().is_empty()
        && !uuid.is_empty();
    if let Some(index) = data
        .config
        .accounts_data
        .accounts
        .iter()
        .position(|x| x.is_same_account(&auth))
    {
        data.config.accounts_data.accounts[index] = auth;
        data.config.accounts_data.selected = index;
    } else {
        data.config.accounts_data.accounts.push_back(auth);
        data.config.accounts_data.selected = data.config.accounts_data.accounts.len() - 1;
    }
    save_config(data);

    if need_skin {
        let sink = ctx.get_external_handle();
        thread_main(move || {
            match fetch_head_skin(&uuid) {
                Ok((head, hat)) => {
                    let _ = sink.submit_command(
                        APP_EVENT,
                        AppEvent::AccountSkinLoaded { uuid, head, hat },
                        Target::Global,
                    );
                }
                Err(err) => {
                    tracing::debug!("获取玩家皮肤失败：{err:?}");
                }
            }
            Ok(())
        });
    }
}

/// 从 Mojang 会话服务获取玩家头部与帽子皮肤像素
fn fetch_head_skin(uuid: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    use anyhow::Context as _;
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let profile: serde_json::Value = scl_core::http::retry_get_json(format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{uuid}"
    ))
    .context("获取玩家信息失败")?;
    let value = profile
        .get("properties")
        .and_then(|properties| properties.as_array())
        .and_then(|properties| {
            properties
                .iter()
                .find(|item| item.get("name").and_then(|name| name.as_str()) == Some("textures"))
        })
        .and_then(|item| item.get("value"))
        .and_then(|value| value.as_str())
        .context("玩家信息中没有皮肤数据")?;
    let decoded = STANDARD.decode(value).context("皮肤数据 base64 解析失败")?;
    let textures: serde_json::Value =
        serde_json::from_slice(&decoded).context("皮肤数据解析失败")?;
    let skin_url = textures
        .get("textures")
        .and_then(|textures| textures.get("skin"))
        .and_then(|skin| skin.get("url"))
        .and_then(|url| url.as_str())
        .context("皮肤数据中没有 skin url")?
        .to_owned();
    let png = scl_core::http::retry_get_bytes(&skin_url).context("下载皮肤失败")?;
    scl_core::auth::parse_head_skin(png)
}

/// 获取一个新的离线账户
pub fn offline_account(name: &str, uuid: &str) -> AuthMethod {
    let uuid = if uuid.trim().is_empty() {
        format!("{:x}", generate_offline_uuid(name))
    } else {
        uuid.trim().replace('-', "")
    };
    AuthMethod::Offline {
        player_name: name.to_owned(),
        uuid,
    }
}

/// 启动器的根组件，用于处理全局命令并启动后台任务。
pub struct AppWrapper {
    inner: WidgetPod<AppState, Box<dyn Widget<AppState>>>,
    started: bool,
    /// 上一次 on_page 通知的页面 ID，用于区分「首次进入主页」与
    /// 「从启动中页面返回主页」，避免误清刚设置的启动贴士。
    last_page: String,
    /// 紧随其后的 ON_PAGE 是否由 POP 触发。
    ///
    /// PageSwitcher 在弹栈动画帧内先发 POP_PAGE 再发 ON_PAGE（与 push 路径
    /// 共用同一个 ON_PAGE 选择器）；官方模组搜索页只在 **push** 时清空并
    /// 重载默认列表，返回（pop）时保留搜索词与结果。这里用该标记复刻。
    next_on_page_is_pop: bool,
    /// 左上角返回按钮当前是否可用。
    ///
    /// 与 [`scl_gui_widgets::widgets::ENABLE_BACK_PAGE`] 保持同步：主页时返回按钮
    /// 隐藏，Esc 也必须无操作。快捷键不直接看页面链，避免出现第二套导航判断。
    back_page_enabled: bool,
    /// 主页启动按钮悬停时显示的完整版本名（仅版本名被省略时需要）。
    version_tooltip: Option<String>,
}

impl AppWrapper {
    /// 创建一个新的根组件
    pub fn new(inner: impl Widget<AppState> + 'static) -> Self {
        Self {
            inner: WidgetPod::new(Box::new(inner)),
            started: false,
            last_page: String::new(),
            next_on_page_is_pop: false,
            back_page_enabled: false,
            version_tooltip: None,
        }
    }

    /// 返回上一页：Esc 与左上角返回按钮共用这一个入口，
    /// 具体返回到哪一页由 `PageSwitcher` 的页面链决定。
    fn back_page(ctx: &mut EventCtx) {
        ctx.submit_command(crate::ui::QUERY_POP_PAGE.with("").to(Target::Global));
    }

    fn handle_command(&mut self, ctx: &mut EventCtx, cmd: &Command, data: &mut AppState) -> bool {
        if let Some(tooltip) = cmd.get(scl_gui_widgets::widgets::VERSION_NAME_TOOLTIP) {
            if self.version_tooltip != *tooltip {
                self.version_tooltip = tooltip.to_owned();
                ctx.request_paint();
            }
            return true;
        } else if let Some(enable) = cmd.get(scl_gui_widgets::widgets::ENABLE_BACK_PAGE) {
            self.back_page_enabled = *enable;
            return true;
        } else if let Some(name) = cmd.get(SELECT_VERSION) {
            data.config.selected_version = name.to_owned();
            data.update_selected_version();
            save_config(data);
            ctx.submit_command(crate::ui::PUSH_PAGE.with(PAGE_MAIN).to(Target::Global));
            return true;
        } else if let Some(name) = cmd.get(SELECT_VERSION_SETTING) {
            version_setting::on_app_event_command(ctx, data, name);
            return true;
        } else if cmd.get(SAVE_VERSION_SETTING).is_some() {
            version_setting::save_version(ctx, data);
            return true;
        } else if cmd.get(REMOVE_VERSION).is_some() {
            version_setting::remove_version(ctx, data);
            return true;
        } else if let Some(index) = cmd.get(SWITCH_AUTH) {
            if *index < data.config.accounts_data.accounts.len() {
                data.config.accounts_data.selected = *index;
                save_config(data);
                ctx.submit_command(crate::ui::PUSH_PAGE.with(PAGE_MAIN).to(Target::Global));
            }
            return true;
        } else if let Some(index) = cmd.get(DELETE_AUTH) {
            if *index < data.config.accounts_data.accounts.len() {
                data.config.accounts_data.accounts.remove(*index);
                if data.config.accounts_data.selected >= data.config.accounts_data.accounts.len() {
                    data.config.accounts_data.selected =
                        data.config.accounts_data.accounts.len().saturating_sub(1);
                }
                save_config(data);
            }
            return true;
        } else if let Some(info) = cmd.get(SELECT_JAVA) {
            java::select_java(data, info);
            return true;
        } else if let Some(info) = cmd.get(DELETE_JAVA) {
            java::delete_java(data, info);
            return true;
        } else if let Some(file) = cmd.get(MANUALLY_ADD_JAVA) {
            let path = file.path().to_string_lossy().to_string();
            java::manually_add_java(ctx, data, &path);
            return true;
        } else if let Some(text) = cmd.get(SET_MEMORY_STRING) {
            if let Ok(value) = text.trim().parse::<usize>() {
                data.config.mem_size = value;
                data.config.auto_mem_size = false;
                save_config(data);
            }
            return true;
        } else if let Some(file_info) = cmd.get(ADD_MC_FOLDER) {
            let folder = crate::config::MinecraftFolder::Other(
                file_info.path().to_string_lossy().to_string(),
            );
            add_minecraft_folder(ctx, data, folder);
            return true;
        } else if let Some(index) = cmd.get(DEL_MC_FOLDER) {
            if *index >= 2 {
                let custom_index = *index - 2;
                if custom_index < data.config.minecraft_folders.len() {
                    let removed = data.config.minecraft_folders.remove(custom_index);
                    if data.config.minecraft_folder
                        == crate::config::MinecraftFolder::Other(removed)
                    {
                        data.config.minecraft_folder =
                            crate::config::MinecraftFolder::CurrentDirectory;
                    }
                    save_config(data);
                    starting::reload_versions(ctx, data);
                }
            }
            return true;
        } else if let Some(index) = cmd.get(SEL_MC_FOLDER) {
            let folder = match *index {
                0 => Some(crate::config::MinecraftFolder::CurrentDirectory),
                1 => Some(crate::config::MinecraftFolder::OfficalDirectory),
                index => data
                    .config
                    .minecraft_folders
                    .get(index - 2)
                    .map(|path| crate::config::MinecraftFolder::Other(path.clone())),
            };
            if let Some(folder) = folder {
                data.config.minecraft_folder = folder;
                save_config(data);
                starting::reload_versions(ctx, data);
            }
            return true;
        } else if let Some(name) = cmd.get(VANILLA_DOWNLOAD_SELECT) {
            let selected = data
                .download_state_vanilla
                .get()
                .and_then(|versions| versions.all_versions().find(|x| &x.name == name).cloned());
            if let Some(version) = selected {
                data.selected_download_version = Some(version.clone());
                data.clear_loader_versions();
                download::update_download_version_name(data);
                // 选中的原版版本变化后，所有加载器的可用列表都要重新查询
                download::reload_current_loader(ctx, data);
            }
            return true;
        } else if let Some(version) = cmd.get(FABRIC_DOWNLOAD_SELECT) {
            download::select_loader_version(data, "fabric", version);
            return true;
        } else if let Some(version) = cmd.get(QUILTMC_DOWNLOAD_SELECT) {
            download::select_loader_version(data, "quiltmc", version);
            return true;
        } else if let Some(version) = cmd.get(FORGE_DOWNLOAD_SELECT) {
            download::select_loader_version(data, "forge", version);
            return true;
        } else if let Some(version) = cmd.get(NEOFORGE_DOWNLOAD_SELECT) {
            download::select_loader_version(data, "neoforge", version);
            return true;
        } else if let Some(version) = cmd.get(OPTIFINE_DOWNLOAD_SELECT) {
            download::select_loader_version(data, "optifine", version);
            return true;
        } else if let Some(tab) = cmd.get(SET_DOWNLOAD_TAB) {
            if data.download_tab != *tab {
                data.download_tab = *tab;
                download::reload_current_loader(ctx, data);
            }
            return true;
        } else if let Some(request) = cmd.get(DOWNLOAD_GAME) {
            download::start_download(ctx, data, request.to_owned());
            return true;
        } else if cmd.get(START_GAME).is_some() {
            starting::start_game(ctx, data);
            if data.launching {
                // 自动重试轮不重新压入启动页：失败时错误页已在最上层，
                // 后台静默重试，成功由 GameWindowReady 统一弹回主页。
                if !data.auto_recheck_retry {
                    trace_auto_page("push", PAGE_STARTING, "START_GAME");
                    ctx.submit_command(crate::ui::PUSH_PAGE.with(PAGE_STARTING).to(Target::Global));
                } else {
                    trace_auto_page("no-push", PAGE_STARTING, "START_GAME auto-recheck retry");
                }
            } else if data.launch_error.is_some() && self.last_page != PAGE_LAUNCH_ERROR {
                trace_auto_page("push", PAGE_LAUNCH_ERROR, "START_GAME failed synchronously");
                ctx.submit_command(
                    crate::ui::PUSH_PAGE
                        .with(PAGE_LAUNCH_ERROR)
                        .to(Target::Global),
                );
            }
            return true;
        } else if cmd.get(ADD_VERSION).is_some() {
            starting::reload_versions(ctx, data);
            return true;
        } else if cmd.get(LOGIN_OFFLINE).is_some() {
            let name = data.login_offline_user_name.trim();
            if name.is_empty() {
            } else {
                let pop_target = if data.config.accounts_data.accounts.is_empty() {
                    PAGE_MAIN
                } else {
                    PAGE_ACCOUNTS
                };
                let auth = offline_account(name, &data.login_offline_uuid);
                update_or_create_auth(ctx, data, auth);
                data.login_error = None;
                ctx.submit_command(
                    crate::ui::QUERY_POP_PAGE
                        .with(pop_target)
                        .to(Target::Global),
                );
            }
            return true;
        } else if cmd.get(LOGIN_AUTHLIB).is_some() {
            login_authlib(ctx, data);
            return true;
        } else if let Some(index) = cmd.get(SELECT_AUTHLIB_CHARACTER) {
            if let Some(auth) = data.login_authlib_available_characters.get(*index).cloned() {
                let client_token = data.config.client_token.to_owned();
                let sink = ctx.get_external_handle();
                data.login_busy = true;
                data.login_error = None;
                thread_main(move || {
                    let progress = Progress::new(sink.clone());
                    scl_core::progress::Reporter::send(
                        &progress,
                        scl_core::progress::ReportState::SetMessage("正在获取正式访问令牌".into()),
                    );
                    let result = scl_core::auth::authlib::refresh_token(
                        auth.into_core(),
                        &client_token,
                        true,
                    );
                    progress.remove();
                    match result {
                        Ok(auth) => {
                            let _ = sink.submit_command(
                                APP_EVENT,
                                AppEvent::AuthAdded(AuthMethod::from_core(auth)),
                                Target::Global,
                            );
                        }
                        Err(err) => {
                            let err = err.context("获取可用访问令牌失败");
                            let _ = sink.submit_command(
                                APP_EVENT,
                                AppEvent::AuthFailed(format!("{err:?}")),
                                Target::Global,
                            );
                        }
                    }
                    Ok(())
                });
            }
            return true;
        } else if cmd.get(LOGIN_MICROSOFT).is_some() {
            login_method_microsoft::login(ctx, data);
            return true;
        } else if cmd.get(CHECK_UPDATE).is_some() {
            settings::check_update(ctx, data);
            return true;
        } else if cmd.get(UPDATE_NOW).is_some() {
            settings::update_now(ctx, data);
            return true;
        } else if cmd.get(FIX_VERSION).is_some() {
            version_setting::fix_version(ctx, data);
            return true;
        } else if cmd.get(FIX_MINECRAFT).is_some() {
            let folder = data.config.minecraft_folder.to_path_buf();
            let source = data.config.download_source.clone();
            let parallel_amount = data.config.download_parallel_amount;
            let sink = ctx.get_external_handle();
            thread_main(move || {
                let progress = Progress::new(sink.clone());
                scl_core::progress::Reporter::send(
                    &progress,
                    scl_core::progress::ReportState::SetMessage("正在修复版本文件".into()),
                );
                match crate::utils::fix_minecraft::fix_minecraft(
                    &folder,
                    source,
                    parallel_amount,
                    progress.clone(),
                ) {
                    Ok(fixed) => {
                        let _ = sink.submit_command(
                            APP_EVENT,
                            AppEvent::LogLine(format!("修复版本文件完成，修复了 {fixed} 个文件")),
                            Target::Global,
                        );
                    }
                    Err(err) => {
                        tracing::error!("修复版本文件失败：{err:?}");
                    }
                }
                progress.remove();
                Ok(())
            });
            return true;
        } else if let Some(file_info) = cmd.get(commands::OPEN_FILE) {
            let path = file_info.path();
            if path.is_file() {
                if let Some(path) = path.to_str() {
                    java::manually_add_java(ctx, data, path);
                }
            } else {
                let path = path.to_string_lossy().to_string();
                if !path.is_empty() {
                    add_minecraft_folder(ctx, data, crate::config::MinecraftFolder::Other(path));
                }
            }
            return true;
        } else if let Some(url) = cmd.get(OPEN_URL) {
            let _ = crate::utils::open_url::open_url(url);
            return true;
        } else if cmd.get(SAVE_CONFIG).is_some() {
            save_config(data);
            return true;
        } else if let Some(path) = cmd.get(TOGGLE_MOD) {
            version_setting::toggle_mod(ctx, data, path);
            return true;
        } else if let Some(path) = cmd.get(DELETE_MOD) {
            version_setting::delete_mod(ctx, data, path);
            return true;
        } else if let Some(name) = cmd.get(INSTALL_MOD) {
            mod_download_dest::install(ctx, data, name);
            return true;
        } else if let Some(file_info) = cmd.get(SAVE_MOD) {
            mod_download_dest::save_mod(ctx, data, file_info);
            return true;
        } else if cmd.get(SET_MOD_FILES).is_some() {
            version_setting::load_mods(ctx, data);
            return true;
        } else if let Some(id) = cmd.get(MOD_DOWNLOADED) {
            tracing::info!("模组下载完成：{id}");
            return true;
        } else if cmd.get(CF_APPLY_SEARCH).is_some() {
            mods_download_curseforge::apply_search(ctx, data);
            return true;
        } else if cmd.get(CF_LOAD_MORE).is_some() {
            mods_download_curseforge::load_more(ctx, data);
            return true;
        } else if let Some(mod_state) = cmd.get(CF_SELECT_MOD) {
            mods_download_curseforge::select_mod(ctx, data, mod_state);
            return true;
        } else if cmd.get(MR_APPLY_SEARCH).is_some() {
            mods_download_modrinth::apply_search(ctx, data);
            return true;
        } else if cmd.get(MR_LOAD_MORE).is_some() {
            mods_download_modrinth::load_more(ctx, data);
            return true;
        } else if let Some(mod_state) = cmd.get(MR_SELECT_MOD) {
            mods_download_modrinth::select_mod(ctx, data, mod_state);
            return true;
        } else if let Some(version) = cmd.get(SELECT_MOD_INSTALL_VERSION) {
            mod_download_dest::select_version(data, version);
            return true;
        } else if let Some(file) = cmd.get(CF_DOWNLOAD_MOD) {
            data.mod_file_downloads_selected = Some(file.clone());
            ctx.submit_command(
                crate::ui::PUSH_PAGE
                    .with(PAGE_MOD_DOWNLOAD_DEST)
                    .to(Target::Global),
            );
            return true;
        }
        false
    }

    fn handle_app_event(&mut self, ctx: &mut EventCtx, event: AppEvent, data: &mut AppState) {
        match event {
            AppEvent::VersionsLoaded(versions) => {
                data.versions = versions.into();
                // 版本列表刷新（保存/下载/修复后）不应清掉仍存在于列表中的
                // 版本设置页身份：否则当前打开页面的计数回调会被误判为迟到并丢弃。
                if !data.version_setting_version_name.is_empty()
                    && !data
                        .versions
                        .iter()
                        .any(|version| version.name == data.version_setting_version_name)
                {
                    data.version_setting_version_name.clear();
                }
                data.update_selected_version();
                if data.config.selected_version.is_empty() {
                    if let Some(first) = data.versions.front() {
                        data.config.selected_version = first.name.to_owned();
                        data.update_selected_version();
                    }
                }
                // 端到端测试钩子：版本列表加载完成后自动触发一次启动，
                // 用于在无 GUI 交互的环境下用假 Java 记录最终命令行。
                if std::env::var_os("SCL_AUTOLAUNCH").is_some() {
                    ctx.submit_command(START_GAME.with(()).to(Target::Global));
                }
            }
            AppEvent::JavaLoaded { javas, failed } => {
                let loaded = javas.len();
                java::merge_java_list(data, javas);
                // 问题 ⑤：解析失败的候选不能只写日志，页面上要给出可见反馈。
                data.java_scan_message = java::scan_notice(loaded, failed);
            }
            AppEvent::JavaAdded(info) => {
                java::add_java(data, info);
            }
            AppEvent::VanillaLoaded(result) => {
                data.download_state_vanilla = match result {
                    Ok(versions) => {
                        let mut state = DownloadState::default();
                        for version in versions {
                            match version.version_type.as_str() {
                                "release" => state.avaliable_versions.push_back(version),
                                "snapshot" => state.avaliable_snapshot_versions.push_back(version),
                                "old_beta" => state.avaliable_old_beta_versions.push_back(version),
                                "old_alpha" => {
                                    state.avaliable_old_alpha_versions.push_back(version)
                                }
                                _ => state.avaliable_snapshot_versions.push_back(version),
                            }
                        }
                        if data.download_version_name.is_empty() {
                            data.download_version_name = state
                                .avaliable_versions
                                .front()
                                .map(|x| x.name.to_owned())
                                .unwrap_or_default();
                        }
                        data.selected_download_version = state.avaliable_versions.front().cloned();
                        NetworkData::new(state)
                    }
                    Err(err) => NetworkData::new_error(err),
                };
                // 原版在版本列表加载完成后立即查询所有加载器的可用版本
                download::reload_current_loader(ctx, data);
            }
            AppEvent::LoaderLoaded { kind, result } => {
                let state = match result {
                    Ok(versions) => {
                        let mut state = DownloadState::default();
                        for (version, is_snapshot) in versions {
                            if is_snapshot {
                                state.avaliable_snapshot_versions.push_back(version);
                            } else {
                                state.avaliable_versions.push_back(version);
                            }
                        }
                        // 原版加载完成时不自动选中任何加载器版本：
                        // 根页面此时显示「可安装」，子页显示「不安装」圆点。
                        NetworkData::new(state)
                    }
                    Err(err) => NetworkData::new_error(err),
                };
                match kind.as_str() {
                    "fabric" => data.download_state_fabric = state,
                    "quiltmc" => data.download_state_quiltmc = state,
                    "forge" => data.download_state_forge = state,
                    "neoforge" => data.download_state_neoforge = state,
                    "optifine" => data.download_state_optifine = state,
                    _ => {}
                }
            }
            AppEvent::AuthAdded(auth) => {
                update_or_create_auth(ctx, data, auth);
                data.login_busy = false;
                data.login_error = None;
                data.login_microsoft_success = true;
                data.login_microsoft_code.clear();
                data.login_microsoft_url.clear();
                // 官方登录成功一律 query_pop_page("main") 弹回主页
                trace_auto_page("pop", PAGE_MAIN, "AuthAdded");
                ctx.submit_command(crate::ui::QUERY_POP_PAGE.with(PAGE_MAIN).to(Target::Global));
            }
            AppEvent::AuthCharacters(characters) => {
                data.login_busy = false;
                data.login_authlib_available_characters = characters.into();
                if data.login_authlib_available_characters.is_empty() {
                    data.login_error = Some("账户不存在任何角色！".into());
                } else {
                    // 官方：多角色登录成功后 push 独立选择页
                    trace_auto_page("push", PAGE_LOGIN_AUTHLIB_SELECT, "AuthCharacters");
                    ctx.submit_command(
                        crate::ui::PUSH_PAGE
                            .with(PAGE_LOGIN_AUTHLIB_SELECT)
                            .to(Target::Global),
                    );
                }
            }
            AppEvent::AuthFailed(err) => {
                data.login_busy = false;
                data.login_error = Some(err);
            }
            AppEvent::MicrosoftCode { code, url } => {
                data.login_microsoft_code = code;
                data.login_microsoft_url = url;
            }
            AppEvent::MicrosoftSuccess => {
                data.login_microsoft_success = true;
                data.login_error = None;
                ctx.submit_command(
                    scl_gui_widgets::widgets::ENABLE_BACK_PAGE
                        .with(false)
                        .to(Target::Global),
                );
            }
            AppEvent::UpdateChecked(result) => match result {
                Ok((version, logs)) => {
                    data.update_available = Some(version);
                    data.update_logs = Some(logs);
                    data.setting_about_update_logs = data.update_logs.to_owned();
                }
                Err(_) => {
                    // 官方在更新记录拉取失败时于更新页展示网络错误提示，而不是一直停留在加载中。
                    let notice =
                        "肥肠抱歉……因网络错误无法获取更新记录……但是新版本肯定有很多新东西的！所以快更新吧！";
                    data.update_logs = Some(notice.to_owned());
                    data.setting_about_update_logs = data.update_logs.to_owned();
                }
            },
            AppEvent::UpdateFinished(result) => match result {
                Ok(()) => {
                    data.setting_about_update_logs = Some("正在覆盖当前版本".into());
                    ctx.submit_command(druid::commands::CLOSE_WINDOW.to(ctx.window_id()));
                }
                Err(err) => {
                    data.setting_about_update_logs = Some(format!("无法更新 SCL：{err}"));
                }
            },
            AppEvent::VersionConfigLoaded {
                name,
                config,
                version_type,
                maps_desc,
                resource_packs_desc,
            } => {
                data.version_setting_version_name = name.clone();
                data.version_setting_version_name_rename = name;
                data.version_setting_config = config;
                data.version_setting_version_type = version_type;
                // 首帧就带官方占位态：模组行为「读取中」，另两行已是最终计数。
                data.version_setting_mods_desc = "读取中".to_owned();
                data.version_setting_maps_desc = maps_desc;
                data.version_setting_resource_packs_desc = resource_packs_desc;
                // 官方根容器的 DisabledIf 会在 starting_tip 非空时禁用整页；
                // 配置读取完成后先清掉加载提示，再推入页面。
                data.starting_tip = None;
                // 官方在页面推入时就开始加载三行数据；本地 ON_PAGE 要等 300ms
                // 推页动画结束才触发，这里提前启动，动画结束后由在途标记跳过重复请求。
                version_setting::on_app_push_page(ctx, data);
                trace_auto_page("push", PAGE_VERSION_SETTING, "VersionConfigLoaded");
                ctx.submit_command(
                    crate::ui::PUSH_PAGE
                        .with(PAGE_VERSION_SETTING)
                        .to(Target::Global),
                );
            }
            AppEvent::VersionModsLoaded {
                version_name,
                token,
                cache_epoch,
                mods,
                mods_desc,
            } => {
                if !version_setting::accepts_load(
                    &data.version_setting_version_name,
                    data.version_setting_load_token,
                    &version_name,
                    token,
                ) {
                    tracing::debug!(
                        "丢弃迟到的模组回调：目标 {version_name} token={token}，当前 {} token={}",
                        data.version_setting_version_name,
                        data.version_setting_load_token
                    );
                    return;
                }
                if data.version_setting_loading_for == version_name
                    && data.version_setting_loading_token == token
                {
                    data.version_setting_loading_for.clear();
                }
                data.version_mods = mods.into();
                data.rebuild_mods_filter();
                data.version_setting_mods_desc = mods_desc.clone();
                if cache_epoch == data.version_setting_cache_epoch {
                    data.version_setting_cache_put(crate::app_state::VersionSettingCacheEntry {
                        version_name,
                        mods: data.version_mods.clone(),
                        mods_desc,
                        maps_desc: data.version_setting_maps_desc.clone(),
                        resource_packs_desc: data.version_setting_resource_packs_desc.clone(),
                        epoch: cache_epoch,
                    });
                }
            }
            AppEvent::VersionSettingSaved(name) => {
                // 页面弹出时保存完成：重命名后同步选中版本并刷新版本列表，
                // 不改变当前页面（此时页面已经弹回上一页）。
                if name.is_empty() {
                    return;
                }
                // 不写 `version_setting_version_name`：此时用户可能已经打开了
                // 另一个版本的设置页，写入旧版本名会污染新页面的身份键。
                data.config.selected_version = name;
                starting::reload_versions(ctx, data);
            }
            AppEvent::VersionFixFinished(result) => {
                // 修复任务会改写版本目录，三行计数缓存作废。
                version_setting::invalidate_cache(data);
                // 修复任务结束：清空「修复中」提示，根容器 DisabledIf 随之解除。
                data.starting_tip = None;
                match result {
                    Ok(fixed) => tracing::info!("版本级修复完成，修复 {fixed} 个文件"),
                    Err(err) => tracing::error!("版本级修复失败：{err}"),
                }
            }
            AppEvent::DownloadFinished(result) => {
                data.starting_tip = None;
                match result {
                    Ok(name) => {
                        data.config.selected_version = name;
                        starting::reload_versions(ctx, data);
                        // 原版下载完成后是弹回已有的主页面，而不是再压入一个新的主页
                        // （实测：下载完成后主页面左上角没有返回箭头，页面链只剩 main）。
                        trace_auto_page("pop", PAGE_MAIN, "DownloadFinished ok");
                        ctx.submit_command(
                            crate::ui::QUERY_POP_PAGE.with(PAGE_MAIN).to(Target::Global),
                        );
                    }
                    Err(_err) => {
                        // 下载失败按原版处理：主窗口弹回主页，错误由后台任务错误弹窗显示，
                        // 不再进入 launch_error 页面。启动期失败仍走 launch_error。
                        trace_auto_page("pop", PAGE_MAIN, "DownloadFinished err");
                        ctx.submit_command(
                            crate::ui::QUERY_POP_PAGE.with(PAGE_MAIN).to(Target::Global),
                        );
                    }
                }
            }
            AppEvent::LaunchFailed(err) => {
                data.launching = false;
                data.starting_tip = None;
                data.launch_error = Some(err.clone());
                data.launch_error_logs.push_back(err);
                // 启动失败即离开启动中页面：不能把启动页留在栈上继续转圈。
                // 已在错误页时（自动重试轮再次失败）不重复弹栈，避免错误页闪烁。
                if self.last_page != PAGE_LAUNCH_ERROR {
                    show_launch_error_page(ctx);
                }
                Self::schedule_auto_repair(ctx, data);
            }
            AppEvent::RecheckRetry => {
                // 只有闩锁仍为真（重试已安排且未被终态复位）才重新启动，
                // 防止过期/重复的修复完成通知再次触发启动。
                if data.auto_recheck_retry {
                    tracing::debug!("auto start-game: origin=RecheckRetry");
                    ctx.submit_command(START_GAME.with(()).to(Target::Global));
                }
            }
            AppEvent::InitFinished(config) => {
                data.config = *config;
                data.first_run = false;
                data.starting_tip = None;
                save_config(data);
            }
            AppEvent::LogLine(line) => {
                data.launch_error_logs.push_back(line);
                while data.launch_error_logs.len() > 1000 {
                    data.launch_error_logs.pop_front();
                }
            }
            AppEvent::AccountSkinLoaded { uuid, head, hat } => {
                let mut changed = false;
                for account in data.config.accounts_data.accounts.iter_mut() {
                    if account.uuid() == uuid {
                        account.set_head_skin(head.clone(), hat.clone());
                        changed = true;
                    }
                }
                if changed {
                    save_config(data);
                }
            }
            AppEvent::GameStarted(pid) => {
                tracing::info!("游戏已启动，进程 ID：{pid}");
                data.launching = false;
                // 注意：不要在这里清 starting_tip。官方进程 spawn 后启动页仍显示
                // 「你知道吗」卡片；本地曾在 GameStarted 时清空，导致卡片变成空白框。
                // 这里也不能复位 auto_recheck_retry：自动重试轮尚未证明成功，
                // 若此时崩溃会被误判为新一轮失败而无限重试；成功在 GameWindowReady 复位。
                let sink = ctx.get_external_handle();
                let wait_until_launched = data.config.wait_until_launched;
                thread_main(move || {
                    if wait_until_launched {
                        if starting::wait_for_game_window(pid) {
                            let _ = sink.submit_command(
                                APP_EVENT,
                                AppEvent::GameWindowReady,
                                Target::Global,
                            );
                        }
                    } else {
                        // 官方不等待窗口时只给 1 秒观察窗口，随后直接回主页。
                        starting::wait_for_game_window_for(pid, std::time::Duration::from_secs(1));
                        let _ = sink.submit_command(
                            APP_EVENT,
                            AppEvent::GameWindowReady,
                            Target::Global,
                        );
                    }
                    Ok(())
                });
            }
            AppEvent::GameWindowReady => {
                if !data.launch_window_ready && data.launch_error.is_none() {
                    data.launch_window_ready = true;
                    // 启动成功（含自动重试成功）：复位重试闩锁，
                    // 下一次手动启动重新拥有一次自动修复机会。
                    data.auto_recheck_retry = false;
                    tracing::info!("游戏窗口已出现，执行启动器收尾");
                    if data.config.hide_window_when_launched {
                        #[cfg(target_os = "windows")]
                        starting::hide_launcher_window();
                        let window = ctx.window_id();
                        ctx.submit_command(commands::HIDE_WINDOW.to(Target::Window(window)));
                    }
                    if !data.config.dont_close_window_when_launched {
                        // 与关窗一致：退出前同步保存，避免后台任务被进程退出打断。
                        if let Err(err) = config::save_config_sync(&data.config) {
                            tracing::error!("启动游戏后保存配置文件失败：{err:?}");
                        }
                        ctx.submit_command(commands::QUIT_APP);
                    } else {
                        trace_auto_page("pop", PAGE_MAIN, "GameWindowReady");
                        ctx.submit_command(
                            crate::ui::QUERY_POP_PAGE.with(PAGE_MAIN).to(Target::Global),
                        );
                    }
                }
            }
            // 游戏进程以非零返回值退出：原版会显示「游戏启动了，但是出错了……」页面，
            // 带完整游戏输出与日志文件/剪切板操作，而不是静默回到主页。
            AppEvent::GameCrashed(code) => {
                tracing::warn!("游戏进程异常退出，返回值：{code:?}");
                data.launching = false;
                data.starting_tip = None;
                let message = format!(
                    "游戏进程异常退出，返回值：{}",
                    code.map(|c| c.to_string()).unwrap_or_else(|| "未知".into())
                );
                data.launch_error = Some(message.clone());
                data.launch_error_logs.push_back(message);
                // 已在错误页时（自动重试轮再次失败）不重复弹栈，避免错误页闪烁。
                if self.last_page != PAGE_LAUNCH_ERROR {
                    show_launch_error_page(ctx);
                }
                // 启动期崩溃（窗口尚未出现）与 LaunchFailed 一样走自动修复 + 重试；
                // 游戏窗口出现后运行中途崩溃不自动重启，只停在错误页。
                Self::schedule_auto_repair(ctx, data);
            }
            AppEvent::ModrinthLoaded {
                mods,
                page,
                generation,
            } => {
                // 迟到回调丢弃：官方 apply_search 任务比较 SEARCH_TIMER
                if data.mods_download_modrinth_search_generation != generation {
                    tracing::trace!(
                        "丢弃过期的 Modrinth 搜索结果：generation={generation}，当前={}",
                        data.mods_download_modrinth_search_generation
                    );
                } else {
                    let mods = Vector::from(mods);
                    if page <= 1 {
                        data.mods_download_modrinth_result_mods = Some(mods);
                    } else if let Some(list) = &mut data.mods_download_modrinth_result_mods {
                        list.append(mods);
                    } else {
                        data.mods_download_modrinth_result_mods = Some(mods);
                    }
                    data.mods_download_modrinth_searching = false;
                }
            }
            AppEvent::CurseforgeLoaded {
                mods,
                page,
                generation,
            } => {
                if data.mods_download_curseforge_search_generation != generation {
                    tracing::trace!(
                        "丢弃过期的 CurseForge 搜索结果：generation={generation}，当前={}",
                        data.mods_download_curseforge_search_generation
                    );
                } else {
                    let mods = Vector::from(mods);
                    if page <= 1 {
                        data.mods_download_curseforge_result_mods = Some(mods);
                    } else if let Some(list) = &mut data.mods_download_curseforge_result_mods {
                        list.append(mods);
                    } else {
                        data.mods_download_curseforge_result_mods = Some(mods);
                    }
                    data.mods_download_curseforge_searching = false;
                }
            }
            AppEvent::ModIconLoaded { source, id, icon } => {
                // 官方图标 handler 同时写回搜索结果列表与文件页持有的
                if let Some(info) = &mut data.mod_file_downloads_info {
                    if info.id == id {
                        info.icon = icon.clone();
                    }
                }
                let list = if source == 0 {
                    &mut data.mods_download_curseforge_result_mods
                } else {
                    &mut data.mods_download_modrinth_result_mods
                };
                if let Some(list) = list {
                    if let Some(item) = list.iter_mut().find(|x| x.id == id) {
                        item.icon = icon;
                    }
                }
            }
            AppEvent::ModCnameLoaded { source, key, cname } => {
                if let Some(info) = &mut data.mod_file_downloads_info {
                    if info.slug == key {
                        info.cname = cname.clone();
                    }
                }
                let list = if source == 0 {
                    &mut data.mods_download_curseforge_result_mods
                } else {
                    &mut data.mods_download_modrinth_result_mods
                };
                if let Some(list) = list {
                    if let Some(item) = list.iter_mut().find(|x| x.slug == key) {
                        item.cname = cname;
                    }
                }
            }
            AppEvent::ModFilesLoaded { files, generation } => {
                if data.mod_file_downloads_generation == generation {
                    data.mod_file_downloads_files = Some(files.into());
                } else {
                    tracing::trace!(
                        "丢弃过期的模组文件列表：generation={generation}，当前={}",
                        data.mod_file_downloads_generation
                    );
                }
            }
            AppEvent::ModDownloaded(result) => match result {
                // 模组下载完成后清空底部提示。
                Ok(_) => {
                    data.starting_tip = None;
                    // 安装完成会写入版本 mods 目录，三行计数缓存作废。
                    version_setting::invalidate_cache(data);
                }
                Err(err) => {
                    data.launch_error = Some(err);
                }
            },
        }
    }

    fn handle_notification(
        &mut self,
        ctx: &mut EventCtx,
        notification: &druid::Notification,
        data: &mut AppState,
    ) {
        if notification.is(crate::ui::ON_PAGE) {
            if let Some(page) = notification.get(crate::ui::ON_PAGE) {
                self.on_page(ctx, data, page);
            }
        } else if notification.is(scl_gui_widgets::widgets::POP_PAGE) {
            if let Some(page) = notification.get(scl_gui_widgets::widgets::POP_PAGE) {
                // 同一动画帧内随后会发 ON_PAGE；标记它为 pop，供 on_page 区分。
                self.next_on_page_is_pop = true;
                if *page == PAGE_LOGIN_MICROSOFT {
                    data.login_microsoft_success = false;
                    data.login_microsoft_code.clear();
                    data.login_microsoft_url.clear();
                    data.login_error = None;
                    data.login_busy = false;
                } else if *page == PAGE_LOGIN_AUTHLIB {
                    // 问题 ④：错误状态下返回登录方式页再进来时，旧错误文案不应残留。
                    data.login_error = None;
                    data.login_busy = false;
                } else if *page == PAGE_VERSION_SETTING {
                    // 官方在该页弹出时后台保存版本设置（含重命名）。
                    version_setting::on_app_pop_page(ctx, data);
                }
            }
        } else if notification.is(scl_gui_widgets::widgets::BACK_PAGE_CLICKED) {
            Self::back_page(ctx);
        } else if notification.is(scl_gui_widgets::widgets::QUERY_CLOSE_WINDOW) {
            // DEV-10：官方关窗不保存版本级设置，会丢修改；这里在退出前补一次同步写盘。
            if crate::ui::PAGE_VERSION_SETTING == self.last_page
                || crate::ui::PAGE_VERSION_SETTING_MODS == self.last_page
                || crate::ui::PAGE_VERSION_SETTING_ADVANCE == self.last_page
                || crate::ui::PAGE_VERSION_SETTING_REMOVE == self.last_page
            {
                version_setting::save_on_close(data);
            }
            // 官方在关窗时同步落盘（save_config_sync）。本地原实现后台保存后
            // 立刻 QUIT_APP，进程先退出导致文件根本没写完，这是设置丢失的根因。
            if let Err(err) = config::save_config_sync(&data.config) {
                tracing::error!("关闭前保存配置文件失败：{err:?}");
            }
            ctx.submit_command(commands::QUIT_APP);
        }
    }

    fn on_page(&mut self, ctx: &mut EventCtx, data: &mut AppState, page: &str) {
        tracing::debug!("on_page: {page:?}");
        // 页面切换后旧页面的悬停提示不应残留。
        self.version_tooltip = None;
        // ON_PAGE 同时用于 push 与 pop；只有 push 才执行「进入页面」逻辑。
        let is_pop = std::mem::take(&mut self.next_on_page_is_pop);
        // 仅在「从启动中页面切到其它页面」时清空贴士。不能在任意
        // 非 starting 的 on_page 里清：SCL_AUTOLAUNCH 等场景下，主页的
        // 首次 on_page 可能晚于 start_game，会把刚设置好的贴士清成空白框。
        if self.last_page == PAGE_STARTING && page != PAGE_STARTING {
            data.starting_tip = None;
        }
        self.last_page = page.to_owned();
        let enable_back = page != PAGE_MAIN;
        ctx.submit_command(
            scl_gui_widgets::widgets::ENABLE_BACK_PAGE
                .with(enable_back)
                .to(Target::Global),
        );
        // druid 只把 KeyDown 路由给焦点链；页面切换后把焦点收回到根组件，
        // 这样即使页面里没有任何输入框，Esc 也能被根组件收到。
        ctx.request_focus();
        match page {
            PAGE_DOWNLOAD => download::on_app_push_page(ctx, data),
            PAGE_VERSION_SETTING => version_setting::on_app_push_page(ctx, data),
            PAGE_VERSION_SETTING_MODS => version_setting::load_mods(ctx, data),
            PAGE_ACCOUNTS => {
                if data.config.accounts_data.accounts.is_empty() {
                    trace_auto_page("push", PAGE_LOGIN_OFFLINE, "PAGE_ACCOUNTS empty");
                    ctx.submit_command(
                        crate::ui::PUSH_PAGE
                            .with(PAGE_LOGIN_OFFLINE)
                            .to(Target::Global),
                    );
                }
            }
            PAGE_LOGIN_MICROSOFT => {
                data.login_microsoft_success = false;
                data.login_error = None;
                if std::env::var_os("SCL_MOCK").is_none() {
                    data.login_microsoft_code.clear();
                    data.login_microsoft_url.clear();
                    login_method_microsoft::login(ctx, data);
                }
            }
            PAGE_LOGIN_OFFLINE => {
                // 端到端测试钩子（与 SCL_AUTOLAUNCH 同类的非交互驱动）：
                // SCL_AUTO_OFFLINE_LOGIN=<玩家名> 时自动填名并提交，
                // 用于无输入环境下验证登录成功后的页面栈行为。
                if let Some(name) = std::env::var_os("SCL_AUTO_OFFLINE_LOGIN") {
                    let name = name.to_string_lossy().to_string();
                    if !name.is_empty() {
                        data.login_offline_user_name = name;
                        ctx.submit_command(LOGIN_OFFLINE.with(()).to(Target::Global));
                    }
                }
            }
            PAGE_LOGIN_AUTHLIB => {
                // 每次重新进入页面都从干净的错误态开始（pop 回退时保留上一次
                // 成功/失败结果的做法会让旧错误文案跨导航残留，问题 ④）。
                if !is_pop {
                    data.login_error = None;
                }
                // 端到端测试钩子：SCL_AUTO_AUTHLIB_LOGIN=<服务器,用户名,密码>
                // 自动填表提交，配合本地 mock authlib 服务器验证多角色选择页流程。
                if let Some(spec) = std::env::var_os("SCL_AUTO_AUTHLIB_LOGIN") {
                    let spec = spec.to_string_lossy().to_string();
                    let mut parts = spec.split(',');
                    if let (Some(server), Some(user), Some(pass)) =
                        (parts.next(), parts.next(), parts.next())
                    {
                        data.login_authlib_server_url = server.to_owned();
                        data.login_authlib_username = user.to_owned();
                        data.login_authlib_password = pass.to_owned();
                        ctx.submit_command(LOGIN_AUTHLIB.with(()).to(Target::Global));
                    }
                }
            }
            PAGE_LOGIN_AUTHLIB_SELECT => {
                // 端到端测试钩子：SCL_AUTO_AUTHLIB_SELECT=<序号> 自动点选角色，
                // 验证选择后的令牌刷新与 pop 回主页流程。
                if let Some(index) = std::env::var_os("SCL_AUTO_AUTHLIB_SELECT") {
                    if let Ok(index) = index.to_string_lossy().parse::<usize>() {
                        ctx.submit_command(SELECT_AUTHLIB_CHARACTER.with(index).to(Target::Global));
                    }
                }
            }
            PAGE_MODS_DOWNLOAD_MODRINTH => {
                // 官方只在 push 时清空搜索框并重载默认列表；返回（pop）时
                if !is_pop {
                    mods_download_modrinth::on_app_push_page(ctx, data);
                }
            }
            PAGE_MODS_DOWNLOAD_CURSEFORGE if !is_pop => {
                mods_download_curseforge::on_app_push_page(ctx, data);
            }
            _ => {}
        }
    }
}

/// 启动失败/崩溃终态：立即回主界面并推入启动失败页
///
/// `RESET_PAGE_TO` 会撤销尚未完成的推入动画并截断页面链，因此不需要
/// 旧的「等待 500ms 再弹栈」定时器；启动中页会被立刻移除，转圈动画随之停止。
fn show_launch_error_page(ctx: &mut EventCtx) {
    trace_auto_page("reset", PAGE_MAIN, "launch error");
    ctx.submit_command(crate::ui::RESET_PAGE_TO.with(PAGE_MAIN).to(Target::Global));
    trace_auto_page("push", PAGE_LAUNCH_ERROR, "launch error");
    ctx.submit_command(
        crate::ui::PUSH_PAGE
            .with(PAGE_LAUNCH_ERROR)
            .to(Target::Global),
    );
}

impl Widget<AppState> for AppWrapper {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &druid::Env) {
        match event {
            Event::KeyDown(key)
                if should_back_on_key(self.back_page_enabled, key.code, key.repeat) =>
            {
                tracing::debug!("Esc 快捷键：返回上一页");
                Self::back_page(ctx);
                ctx.set_handled();
            }
            Event::Command(cmd) => {
                #[cfg(unix)]
                crate::utils::progress::update_taskbar_progress(cmd, ctx.window());
                if cmd.is(APP_EVENT) {
                    if let Some(event) = cmd.get(APP_EVENT) {
                        self.handle_app_event(ctx, event.to_owned(), data);
                    }
                } else {
                    self.handle_command(ctx, cmd, data);
                }
            }
            Event::Notification(notification) => {
                self.handle_notification(ctx, notification, data);
            }
            _ => {}
        }
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &AppState,
        env: &druid::Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            if !self.started {
                self.started = true;
                let sink = ctx.get_external_handle();
                if std::env::var_os("SCL_MOCK").is_none() {
                    starting::init(sink.clone(), data);
                    if data.first_run {
                        starting::init_settings(sink.clone(), data.config.to_owned());
                    }
                }
                if let Ok(page) = std::env::var("SCL_START_PAGE") {
                    let page: &'static str = Box::leak(page.into_boxed_str());
                    if !page.is_empty() {
                        tracing::debug!("test hook SCL_START_PAGE={page:?}: 2s 后自动 PUSH_PAGE");
                        let sink = sink.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_secs(2));
                            trace_auto_page("push", page, "SCL_START_PAGE hook");
                            let _ = sink.submit_command(crate::ui::PUSH_PAGE, page, Target::Global);
                        });
                    }
                }
                ctx.submit_command(
                    crate::ui::SET_SLIDE_PAGE_ANIMATION
                        .with(data.config.theme.slide_page_animation)
                        .to(Target::Global),
                );
                if !data.config.theme.theme_background_image_path.is_empty() {
                    let path = PathBuf::from(&data.config.theme.theme_background_image_path);
                    if let Ok(image) = image::open(path) {
                        if let Ok(buf) = crate::utils::thread::image_buf_from_dynamic(image) {
                            let _ = sink.submit_command(
                                scl_gui_widgets::widgets::SET_BACKGROUND_IMAGE,
                                buf,
                                Target::Global,
                            );
                        }
                    }
                }
            }
        }
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        old_data: &AppState,
        data: &AppState,
        env: &druid::Env,
    ) {
        // 设置页通过 Lens 直接修改 config，不会产生命令；这里统一监听 config
        // 变化并防抖落盘，保证修改后即使不关窗也不会丢设置。
        if old_data.config != data.config {
            save_config_debounced(data);
        }
        if old_data.config.theme.slide_page_animation != data.config.theme.slide_page_animation {
            ctx.submit_command(
                crate::ui::SET_SLIDE_PAGE_ANIMATION
                    .with(data.config.theme.slide_page_animation)
                    .to(Target::Global),
            );
        }
        self.inner.update(ctx, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &AppState,
        env: &druid::Env,
    ) -> druid::Size {
        let size = self.inner.layout(ctx, bc, data, env);
        self.inner.set_origin(ctx, druid::Point::ZERO);
        size
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &AppState, env: &druid::Env) {
        self.inner.paint(ctx, data, env);
        self.paint_version_tooltip(ctx, env);
    }
}

impl AppWrapper {
    /// 启动失败/启动期崩溃后安排一次自动文件修复与重试。
    ///
    /// 触发条件：
    /// - 配置开关 `auto_check_when_first_start_failed` 为真；
    /// - 当前启动周期尚未安排过自动重试（`auto_recheck_retry` 闩锁为假）；
    /// - 游戏尚未真正显示窗口（`launch_window_ready` 为假），避免游戏中途
    ///   崩溃时意外重启游戏。
    ///
    /// 修复只针对本次启动的版本（含继承链），完成后提交 `RecheckRetry`。
    /// 返回是否安排了自动重试。
    fn schedule_auto_repair(ctx: &mut EventCtx, data: &mut AppState) -> bool {
        if !should_schedule_auto_repair(
            data.config.auto_check_when_first_start_failed,
            data.auto_recheck_retry,
            data.launch_window_ready,
        ) {
            data.auto_recheck_retry = false;
            return false;
        }
        data.auto_recheck_retry = true;
        data.launch_error_logs
            .push_back("启动游戏失败，正在尝试检查文件完整性".into());
        let folder = data.config.minecraft_folder.to_path_buf();
        let version_base = data.versions_folder();
        let version_name = data.launching_version.clone();
        let source = data.config.download_source.clone();
        let parallel_amount = data.config.download_parallel_amount;
        let sink = ctx.get_external_handle();
        thread_main(move || {
            // 自动修复必须在界面上可见：官方同一路径使用
            let progress = Progress::new(sink.clone());
            scl_core::progress::Reporter::send(
                &progress,
                scl_core::progress::ReportState::SetMessage(format!(
                    "正在尝试检查版本 {version_name} 的文件完整性"
                )),
            );
            // 只修复本次要启动的那个版本（及其继承链），
            // 不再遍历 versions 目录，避免无关版本被补全。
            let version_info = scl_core::version::structs::VersionInfo {
                version_base: version_base.to_string_lossy().to_string(),
                version: version_name.clone(),
                ..Default::default()
            };
            match crate::utils::fix_minecraft::fix_minecraft_version(
                &folder,
                &version_info,
                source,
                parallel_amount,
                progress.clone(),
            ) {
                Ok(fixed) => {
                    tracing::info!("版本 {version_name} 文件完整性检查完成，修复了 {fixed} 个文件");
                }
                Err(err) => {
                    tracing::warn!("版本 {version_name} 文件完整性检查失败：{err:?}");
                }
            }
            progress.remove();
            // 无论修复成功、失败还是无可修，官方都进行恰好一次二次启动；
            // 修复自身已在下载层做了有界等待，不会永久挂起。
            let _ = sink.submit_command(APP_EVENT, AppEvent::RecheckRetry, Target::Global);
            Ok(())
        });
        true
    }

    /// 绘制主页启动按钮的完整版本名提示。
    ///
    /// 提示必须画在根组件上：按钮自身的 `WidgetPod` 会被裁到按钮矩形，
    /// 无法在按钮上方显示内容。
    fn paint_version_tooltip(&self, ctx: &mut druid::PaintCtx, env: &druid::Env) {
        let Some(text) = &self.version_tooltip else {
            return;
        };
        let max_width = (ctx.size().width - 24.).max(100.);
        let font = env.get(scl_gui_widgets::theme::color::typography::CAPTION_ALT);
        let Ok(layout) = ctx
            .text()
            .new_text_layout(text.clone())
            .font(font.family.clone(), font.size)
            .text_color(env.get(scl_gui_widgets::theme::color::typography::FONT_COLOR))
            .alignment(druid::TextAlignment::Start)
            .max_width(max_width)
            .build()
        else {
            return;
        };
        let padding = 6.;
        let width = layout.size().width + padding * 2.;
        let height = layout.size().height + padding * 2.;
        let x = (ctx.size().width - width) / 2.;
        // 主界面底部栏：窗口底 - 4 padding - 40 spacer - 62 栏高，提示再上移 8。
        let y = ctx.size().height - 106. - 8. - height;
        if y < 0. {
            return;
        }
        let rect = druid::Rect::from_origin_size((x, y), (width, height)).to_rounded_rect(4.);
        ctx.fill(
            &rect,
            &PaintBrush::Color(env.get(scl_gui_widgets::theme::color::chrome::MEDIUM_LOW)),
        );
        ctx.stroke(
            &rect,
            &PaintBrush::Color(env.get(scl_gui_widgets::theme::color::border::TRANSIENT)),
            1.,
        );
        ctx.draw_text(&layout, (x + padding, y + padding));
    }
}

fn same_path(a: &std::path::Path, b: &std::path::Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// 添加并切换到指定的 Minecraft 文件夹
fn add_minecraft_folder(
    ctx: &mut EventCtx,
    data: &mut AppState,
    folder: crate::config::MinecraftFolder,
) {
    if let crate::config::MinecraftFolder::Other(path) = &folder {
        if let Some(existing) =
            data.config.minecraft_folders.iter().find(|existing| {
                same_path(std::path::Path::new(existing), std::path::Path::new(path))
            })
        {
            data.config.minecraft_folder = crate::config::MinecraftFolder::Other(existing.clone());
            save_config(data);
            starting::reload_versions(ctx, data);
            return;
        }
        data.config.minecraft_folders.push_back(path.clone());
    }
    data.config.minecraft_folder = folder;
    save_config(data);
    starting::reload_versions(ctx, data);
}

fn login_authlib(ctx: &mut EventCtx, data: &mut AppState) {
    let server = data.login_authlib_server_url.trim().to_owned();
    let username = data.login_authlib_username.trim().to_owned();
    let password = data.login_authlib_password.to_owned();
    if server.is_empty() || username.is_empty() || password.is_empty() {
        data.login_error = Some("请填写服务器地址、用户名和密码".into());
        return;
    }
    data.login_busy = true;
    data.login_error = None;
    data.config.last_authlib_server = server.to_owned();
    save_config(data);
    let sink = ctx.get_external_handle();
    let client_token = data.config.client_token.to_owned();
    let minecraft_folder = data.config.minecraft_folder.to_path_buf();
    let download_source = data.config.download_source.to_owned();
    let parallel_amount = data.config.download_parallel_amount;
    thread_main(move || {
        let progress = Progress::new(sink.clone());
        scl_core::progress::Reporter::send(
            &progress,
            scl_core::progress::ReportState::SetMessage("正在尝试登录".into()),
        );
        let result = scl_core::auth::authlib::start_auth(
            Some(progress.clone()),
            &server,
            username,
            scl_core::password::Password::from(password),
            &client_token,
        );
        match result {
            Ok(characters) => {
                let downloader = scl_core::download::Downloader::<Progress>::default()
                    .with_source(download_source.to_core())
                    .with_minecraft_path(&minecraft_folder)
                    .with_parallel_amount(parallel_amount)
                    .with_reporter(progress.clone());
                scl_core::progress::Reporter::send(
                    &progress,
                    scl_core::progress::ReportState::SetMessage(
                        "检查 Authlib Injector 注入库".into(),
                    ),
                );
                if let Err(err) = downloader.install_authlib_injector() {
                    let _ = sink.submit_command(
                        APP_EVENT,
                        AppEvent::AuthFailed(format!("{err:?}")),
                        Target::Global,
                    );
                    return Ok(());
                }
                progress.remove();
                match characters.len() {
                    1 => {
                        let single = characters.into_iter().next().unwrap();
                        match scl_core::auth::authlib::refresh_token(single, &client_token, true) {
                            Ok(auth) => {
                                let _ = sink.submit_command(
                                    APP_EVENT,
                                    AppEvent::AuthAdded(AuthMethod::from_core(auth)),
                                    Target::Global,
                                );
                            }
                            Err(err) => {
                                let _ = sink.submit_command(
                                    APP_EVENT,
                                    AppEvent::AuthFailed(format!("{err:?}")),
                                    Target::Global,
                                );
                            }
                        }
                    }
                    // 多角色（>1）：不刷新令牌，push 独立角色选择页后再按选择刷新。
                    _ => {
                        let characters =
                            characters.into_iter().map(AuthMethod::from_core).collect();
                        let _ = sink.submit_command(
                            APP_EVENT,
                            AppEvent::AuthCharacters(characters),
                            Target::Global,
                        );
                    }
                }
            }
            Err(err) => {
                let _ = sink.submit_command(
                    APP_EVENT,
                    AppEvent::AuthFailed(format!("{err:?}")),
                    Target::Global,
                );
            }
        }
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use druid::Code;

    use super::{should_back_on_key, should_schedule_auto_repair};

    #[test]
    fn escape_backs_when_button_is_available() {
        assert!(should_back_on_key(true, Code::Escape, false));
    }

    #[test]
    fn escape_does_nothing_on_main_page() {
        // 主页没有返回目标，返回按钮隐藏，Esc 必须同样无操作。
        assert!(!should_back_on_key(false, Code::Escape, false));
    }

    #[test]
    fn escape_ignores_auto_repeat() {
        // 按住 Esc 不连续退掉多层页面。
        assert!(!should_back_on_key(true, Code::Escape, true));
    }

    #[test]
    fn other_keys_are_not_back() {
        assert!(!should_back_on_key(true, Code::Enter, false));
        assert!(!should_back_on_key(true, Code::KeyE, false));
    }

    #[test]
    fn auto_repair_only_for_first_startup_failure() {
        assert!(
            should_schedule_auto_repair(true, false, false),
            "首次启动失败且开关开启时应自动修复"
        );
        assert!(
            !should_schedule_auto_repair(false, false, false),
            "关闭自动检查时不应修复"
        );
        assert!(
            !should_schedule_auto_repair(true, true, false),
            "自动重试已经进行过一次时不应再次修复"
        );
        assert!(
            !should_schedule_auto_repair(true, false, true),
            "游戏窗口已出现后的中途崩溃不应自动重启"
        );
    }
}
