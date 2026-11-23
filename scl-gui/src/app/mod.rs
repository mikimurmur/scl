//! 启动器的应用逻辑。
//!
//! 所有界面之间共享的逻辑以及后台任务都放在这里。
//!
//! 迁移到 Iced 后，后台任务仍然使用与旧版相同的“选择器 + 命令”模式与
//! 界面通信。这里的 [`Selector`] / [`Command`] / [`EventSink`] 是旧版
//! Druid API 的最小兼容层，避免为了换 UI 框架而重写全部业务逻辑。

pub mod download;
pub mod java;
pub mod login_method_microsoft;
pub mod mod_download_dest;
pub mod mods_download_curseforge;
pub mod mods_download_modrinth;
pub mod settings;
pub mod starting;
pub mod version_setting;

use std::any::Any;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use futures::channel::mpsc;
use iced::widget::image;
use iced::{Element, Subscription, Task, window};
use im::Vector;
use once_cell::sync::Lazy;
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
        PAGE_MOD_DOWNLOAD_DEST, PAGE_MODS_DOWNLOAD, PAGE_MODS_DOWNLOAD_CURSEFORGE,
        PAGE_MODS_DOWNLOAD_MODRINTH, PAGE_STARTING, PAGE_VERSION_SETTING,
        PAGE_VERSION_SETTING_MODS,
    },
    utils::{Progress, thread::thread_main},
};

/// 一个带类型的选择器。
///
/// 兼容层只使用选择器的名字做匹配，载荷保存在 [`Command`] 里；这样后台
/// 任务仍然可以像旧版一样先构造命令，再交给界面线程处理。
pub struct Selector<T> {
    name: &'static str,
    _marker: PhantomData<fn(T)>,
}

impl<T> Selector<T> {
    /// 创建一个新的选择器。
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _marker: PhantomData,
        }
    }

    /// 附带载荷，构造一个命令。
    pub fn with(self, payload: T) -> CommandBuilder<T> {
        CommandBuilder {
            selector: self,
            payload,
            target: Target::Global,
        }
    }
}

impl<T> Clone for Selector<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Selector<T> {}

/// 命令的目标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// 全局命令。
    Global,
    /// 指定窗口。
    Window(window::Id),
}

/// 尚未确定目标的命令构造器。
pub struct CommandBuilder<T> {
    selector: Selector<T>,
    payload: T,
    target: Target,
}

impl<T: Send + Sync + 'static> CommandBuilder<T> {
    /// 设置命令目标。
    pub fn to(mut self, target: Target) -> Command {
        self.target = target;
        Command {
            name: self.selector.name,
            payload: Arc::new(self.payload),
        }
    }
}

/// 一条在界面线程和后台任务之间传递的命令。
#[derive(Clone)]
pub struct Command {
    name: &'static str,
    payload: Arc<dyn Any + Send + Sync>,
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command").field("name", &self.name).finish()
    }
}

impl Command {
    /// 如果命令由指定选择器构造，返回载荷。
    pub fn get<T: 'static>(&self, selector: Selector<T>) -> Option<&T> {
        if self.name == selector.name {
            self.payload.downcast_ref::<T>()
        } else {
            None
        }
    }

    /// 命令是否由指定选择器构造。
    pub fn is<T>(&self, selector: Selector<T>) -> bool {
        self.name == selector.name
    }
}

/// 与 Druid 的 `FileInfo` 对应的文件选择结果。
#[derive(Debug, Clone)]
pub struct FileInfo {
    path: PathBuf,
}

impl FileInfo {
    /// 从路径创建。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 文件/文件夹路径。
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// 后台任务向界面发送消息的句柄。
#[derive(Clone)]
pub struct EventSink {
    sender: mpsc::UnboundedSender<Message>,
}

impl EventSink {
    /// 直接发送一条消息。
    pub fn send(&self, message: Message) {
        let _ = self.sender.unbounded_send(message);
    }

    /// 兼容旧版的三参数命令发送接口。
    pub fn submit_command<T: Send + Sync + 'static>(
        &self,
        selector: Selector<T>,
        payload: T,
        target: Target,
    ) -> Result<(), ()> {
        self.send(Message::Command(selector.with(payload).to(target)));
        Ok(())
    }
}

/// 旧版 Druid 的 `ExtEventSink` 在迁移后的等价类型。
pub type ExtEventSink = EventSink;

/// 界面线程中的命令上下文。
#[derive(Clone)]
pub struct EventCtx {
    sink: EventSink,
    window: window::Id,
}

impl EventCtx {
    fn new(sink: EventSink, window: window::Id) -> Self {
        Self { sink, window }
    }

    /// 获取向界面线程发送消息的句柄。
    pub fn get_external_handle(&self) -> EventSink {
        self.sink.clone()
    }

    /// 提交一条命令。
    pub fn submit_command(&self, command: Command) {
        // 窗口操作不是业务命令，直接转换为 Message，避免再去轮询目标窗口。
        if command.is(CLOSE_WINDOW) {
            self.sink.send(Message::CloseWindow);
        } else if command.is(HIDE_WINDOW) {
            self.sink.send(Message::HideWindow);
        } else if command.is(QUIT_APP) {
            self.sink.send(Message::Quit);
        } else {
            self.sink.send(Message::Command(command));
        }
    }

    /// 当前窗口 ID。
    pub fn window_id(&self) -> window::Id {
        self.window
    }
}

/// 进度上报消息。
#[derive(Debug, Clone)]
pub enum ProgressMessage {
    /// 新建一条进度。
    New(usize),
    /// 增加当前进度。
    Add(usize, f64),
    /// 增加最大进度。
    AddMax(usize, f64),
    /// 设置最大进度。
    SetMax(usize, f64),
    /// 设置当前进度。
    Set(usize, f64),
    /// 设置主消息。
    SetMessage(usize, String),
    /// 设置次级消息。
    SetSubMessage(usize, String),
    /// 隐藏进度（不确定进度）。
    Hide(usize),
    /// 移除进度。
    Remove(usize),
}

/// 界面消息。
#[derive(Debug, Clone)]
pub enum Message {
    /// 一条业务命令。
    Command(Command),
    /// 进度更新。
    Progress(ProgressMessage),
    /// 窗口标题栏操作。
    Window(scl_gui_widgets::widgets::WindowMessage),
    /// 窗口 ID 已就绪。
    WindowReady(Option<window::Id>),
    /// 关闭窗口。
    CloseWindow,
    /// 隐藏窗口。
    HideWindow,
    /// 退出程序。
    Quit,
    /// 后台任务发生非致命错误。
    AsyncError(String, String),
    /// 关闭异步错误提示。
    DismissAsyncError,
    /// 复制异步错误到剪切板。
    CopyAsyncError,
    /// 页面控件产生的界面消息。
    Ui(crate::ui::UiMessage),
    /// 动画帧。
    Tick,
    /// 选择游戏文件夹。
    PickFolder,
    /// 选择 Java 可执行文件。
    PickJava,
    /// 搜索可用的 Java 运行时。
    SearchJava,
    /// 选择模组保存位置。
    SaveMod,
    /// 文件夹选择完成。
    FolderPicked(Option<PathBuf>),
    /// Java 选择完成。
    JavaPicked(Option<PathBuf>),
    /// 模组保存位置选择完成。
    ModSavePicked(Option<PathBuf>),
}

/// 全局消息发送端。
static EVENT_TX: Lazy<mpsc::UnboundedSender<Message>> = Lazy::new(|| {
    let (tx, rx) = mpsc::unbounded();
    *EVENT_RX.lock().unwrap() = Some(rx);
    tx
});

/// 全局消息接收端，由 Iced 的订阅取走。
static EVENT_RX: Lazy<Mutex<Option<mpsc::UnboundedReceiver<Message>>>> =
    Lazy::new(|| Mutex::new(None));

/// 获取全局消息发送句柄。
pub fn event_sink() -> EventSink {
    Lazy::force(&EVENT_TX);
    EventSink {
        sender: EVENT_TX.clone(),
    }
}

/// 把后台任务错误发送到界面。
pub fn report_async_error(location: impl Into<String>, error: impl Into<String>) {
    let _ = EVENT_TX.unbounded_send(Message::AsyncError(location.into(), error.into()));
}

/// 取出全局消息接收端（只会成功一次）。
fn take_event_receiver() -> Option<mpsc::UnboundedReceiver<Message>> {
    Lazy::force(&EVENT_TX);
    EVENT_RX.lock().unwrap().take()
}

/// 页面入栈命令。
pub const PUSH_PAGE: Selector<&'static str> = Selector::new("net.stevexmh.scl.push-page");
/// 返回指定页面命令。
pub const QUERY_POP_PAGE: Selector<&'static str> = Selector::new("net.stevexmh.scl.query-pop-page");
/// 重置页面栈命令。
pub const RESET_PAGE_TO: Selector<&'static str> = Selector::new("net.stevexmh.scl.reset-page-to");
/// 启用/禁用标题栏返回按钮。
pub const ENABLE_BACK_PAGE: Selector<bool> = Selector::new("net.stevexmh.scl.enable-back-page");
/// 设置页面切换动画（Iced 版本保留字段，不再有动画）。
pub const SET_SLIDE_PAGE_ANIMATION: Selector<bool> =
    Selector::new("net.stevexmh.scl.set-slide-page-animation");
/// 设置窗口背景图。
pub const SET_BACKGROUND_IMAGE: Selector<image::Handle> =
    Selector::new("net.stevexmh.scl.set-background-image");
/// 标题栏返回按钮点击。
pub const BACK_PAGE_CLICKED: Selector<()> = Selector::new("net.stevexmh.scl.back-page-clicked");
/// 请求关闭窗口。
pub const QUERY_CLOSE_WINDOW: Selector<()> = Selector::new("net.stevexmh.scl.query-close-window");
/// 关闭窗口。
pub const CLOSE_WINDOW: Selector<()> = Selector::new("net.stevexmh.scl.close-window");
/// 隐藏窗口。
pub const HIDE_WINDOW: Selector<()> = Selector::new("net.stevexmh.scl.hide-window");
/// 退出程序。
pub const QUIT_APP: Selector<()> = Selector::new("net.stevexmh.scl.quit-app");
/// 文件选择器返回。
pub const OPEN_FILE: Selector<FileInfo> = Selector::new("net.stevexmh.scl.open-file");

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
pub const MANUALLY_ADD_JAVA: Selector<FileInfo> =
    Selector::new("net.stevexmh.scl.manually-add-java");
/// 设置内存大小
pub const SET_MEMORY_STRING: Selector<String> = Selector::new("net.stevexmh.scl.set-memory-str");
/// 添加 Minecraft 文件夹
pub const ADD_MC_FOLDER: Selector<FileInfo> = Selector::new("net.stevexmh.scl.add-mc-folder");
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
pub const SAVE_MOD: Selector<FileInfo> = Selector::new("net.stevexmh.scl.save-mod");
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
    /// 扫描到所有 Java 运行时
    JavaLoaded(Vec<crate::config::JavaRuntimeInfo>),
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
    /// `on_app_push_page` 异步填充，与官方时序一致）
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
    /// 官方分发器只有成功载荷通路：搜索 future 失败（网络/JSON 解析）时
    /// 直接走 do_async_ok 弹「执行异步任务发生错误」窗，不向界面提交命令。
    /// `generation` 用于丢弃迟到回调（官方以 SEARCH_TIMER 时刻比较）。
    ModrinthLoaded {
        /// 搜索结果
        mods: Vec<ModState>,
        /// 发起请求时的页码
        page: u64,
        /// 发起请求时的搜索代次
        generation: u64,
    },
    /// CurseForge 搜索结果（同上，失败走异步错误弹窗）
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
    /// 模组文件列表加载完成（失败走异步错误弹窗，不提交本事件）
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

/// 将配置异步保存到磁盘
pub fn save_config(data: &AppState) {
    let cfg = data.config.to_owned();
    thread_main(async move { config::save_config(&cfg).await });
}

/// 设置项发生变化后延迟保存：连续拖动滑块/输入时只落盘最后一次状态。
fn save_config_debounced(data: &AppState) {
    use std::sync::atomic::Ordering;
    let cfg = data.config.to_owned();
    let generation = CONFIG_SAVE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    thread_main(async move {
        smol::Timer::after(std::time::Duration::from_millis(250)).await;
        if CONFIG_SAVE_GENERATION.load(Ordering::SeqCst) == generation {
            if let Err(err) = config::save_config(&cfg).await {
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
        thread_main(async move {
            match fetch_head_skin(&uuid).await {
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
async fn fetch_head_skin(uuid: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    use anyhow::Context as _;
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let profile: serde_json::Value = scl_core::http::retry_get_json(format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{uuid}"
    ))
    .await
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
    let png = scl_core::http::retry_get_bytes(&skin_url)
        .await
        .context("下载皮肤失败")?;
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

/// 启动器根状态。Iced 的 `application` 直接以它作为应用状态。
pub struct AppWrapper {
    /// 全局业务状态。
    pub state: AppState,
    started: bool,
    /// 上一次进入的页面，用于区分「首次进入主页」与从启动页返回。
    last_page: String,
    /// 紧随其后的页面进入是否由返回触发。
    next_on_page_is_pop: bool,
}

impl AppWrapper {
    fn handle_command(&mut self, ctx: &mut EventCtx, cmd: &Command) -> bool {
        let data = &mut self.state;
        if let Some(name) = cmd.get(SELECT_VERSION) {
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
                // 重试轮（闩锁为真）时启动页已在栈顶，不重复压栈；
                // 官方是启动任务内联重试，全程只有一个 starting 页。
                if !data.auto_recheck_retry {
                    trace_auto_page("push", PAGE_STARTING, "START_GAME");
                    ctx.submit_command(crate::ui::PUSH_PAGE.with(PAGE_STARTING).to(Target::Global));
                } else {
                    trace_auto_page("no-push", PAGE_STARTING, "START_GAME auto-recheck retry");
                }
            } else if data.launch_error.is_some() {
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
                thread_main(async move {
                    let progress = Progress::new(sink.clone());
                    scl_core::progress::Reporter::send(
                        &progress,
                        scl_core::progress::ReportState::SetMessage("正在获取正式访问令牌".into()),
                    );
                    let result = scl_core::auth::authlib::refresh_token(
                        auth.into_core(),
                        &client_token,
                        true,
                    )
                    .await;
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
            thread_main(async move {
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
                )
                .await
                {
                    Ok(fixed) => {
                        let _ = sink.submit_command(
                            APP_EVENT,
                            AppEvent::LogLine(format!("修复版本文件完成，修复了 {fixed} 个版本")),
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
        } else if let Some(file_info) = cmd.get(OPEN_FILE) {
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

    fn handle_app_event(&mut self, ctx: &mut EventCtx, event: AppEvent) {
        let data = &mut self.state;
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
            AppEvent::JavaLoaded(javas) => {
                java::merge_java_list(data, javas);
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
                ctx.submit_command(ENABLE_BACK_PAGE.with(false).to(Target::Global));
            }
            AppEvent::UpdateChecked(result) => match result {
                Ok((version, logs)) => {
                    data.update_available = Some(version);
                    data.update_logs = Some(logs);
                    data.setting_about_update_logs = data.update_logs.to_owned();
                }
                Err(_) => {
                    // 官方在更新记录拉取失败时于更新页展示网络错误提示，而不是一直停留在加载中。
                    let notice = "肥肠抱歉……因网络错误无法获取更新记录……但是新版本肯定有很多新东西的！所以快更新吧！";
                    data.update_logs = Some(notice.to_owned());
                    data.setting_about_update_logs = data.update_logs.to_owned();
                }
            },
            AppEvent::UpdateFinished(result) => match result {
                Ok(()) => {
                    data.setting_about_update_logs = Some("正在覆盖当前版本".into());
                    ctx.submit_command(CLOSE_WINDOW.with(()).to(Target::Window(ctx.window_id())));
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
                    Ok(fixed) => tracing::info!("版本级修复完成，修正 {fixed} 项"),
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
                if data.config.auto_check_when_first_start_failed && !data.auto_recheck_retry {
                    data.auto_recheck_retry = true;
                    data.launch_error_logs
                        .push_back("启动游戏失败，正在尝试检查文件完整性".into());
                    let folder = data.config.minecraft_folder.to_path_buf();
                    let source = data.config.download_source.clone();
                    let parallel_amount = data.config.download_parallel_amount;
                    let sink = ctx.get_external_handle();
                    thread_main(async move {
                        // 自动修复必须在界面上可见：官方同一路径使用
                        let progress = Progress::new(sink.clone());
                        scl_core::progress::Reporter::send(
                            &progress,
                            scl_core::progress::ReportState::SetMessage(
                                "正在尝试检查文件完整性".into(),
                            ),
                        );
                        match crate::utils::fix_minecraft::fix_minecraft(
                            &folder,
                            source,
                            parallel_amount,
                            progress.clone(),
                        )
                        .await
                        {
                            Ok(fixed) => {
                                tracing::info!("文件完整性检查完成，修复了 {fixed} 个版本文件");
                            }
                            Err(err) => {
                                tracing::warn!("文件完整性检查失败：{err:?}");
                            }
                        }
                        progress.remove();
                        // 无论修复成功、失败还是无可修，官方都进行恰好一次
                        // 二次启动（无计数器、无退避）；修复自身已在下载层
                        // 做了有界等待，不会永久挂起。
                        let _ =
                            sink.submit_command(APP_EVENT, AppEvent::RecheckRetry, Target::Global);
                        Ok(())
                    });
                } else {
                    data.auto_recheck_retry = false;
                    show_launch_error_page(ctx);
                }
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
                // 启动成功即终态：复位重试闩锁，下一次手动启动重新拥有一次重试。
                data.auto_recheck_retry = false;
                let sink = ctx.get_external_handle();
                let wait_until_launched = data.config.wait_until_launched;
                thread_main(async move {
                    if wait_until_launched {
                        if starting::wait_for_game_window(pid).await {
                            let _ = sink.submit_command(
                                APP_EVENT,
                                AppEvent::GameWindowReady,
                                Target::Global,
                            );
                        }
                    } else {
                        smol::future::race(
                            async {
                                starting::wait_for_game_window(pid).await;
                            },
                            async {
                                smol::Timer::after(std::time::Duration::from_secs(1)).await;
                            },
                        )
                        .await;
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
                    tracing::info!("游戏窗口已出现，执行启动器收尾");
                    if data.config.hide_window_when_launched {
                        #[cfg(target_os = "windows")]
                        starting::hide_launcher_window();
                        let window = ctx.window_id();
                        ctx.submit_command(HIDE_WINDOW.with(()).to(Target::Window(window)));
                    }
                    if !data.config.dont_close_window_when_launched {
                        // 与关窗一致：退出前同步保存，避免异步任务被进程退出打断。
                        if let Err(err) = config::save_config_sync(&data.config) {
                            tracing::error!("启动游戏后保存配置文件失败：{err:?}");
                        }
                        ctx.submit_command(QUIT_APP.with(()).to(Target::Global));
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
                if data.launch_error.is_none() {
                    let message = format!(
                        "游戏进程异常退出，返回值：{}",
                        code.map(|c| c.to_string()).unwrap_or_else(|| "未知".into())
                    );
                    data.launch_error = Some(message);
                    // 与启动失败一样走官方终态序列：先把 `starting` 弹回
                    // 主页再推错误页，错误页返回才会落在主界面。
                    show_launch_error_page(ctx);
                }
            }
            AppEvent::ModrinthLoaded {
                mods,
                page,
                generation,
            } => {
                // 迟到回调丢弃：官方 apply_search future 比较 SEARCH_TIMER
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

    fn on_page(&mut self, ctx: &mut EventCtx, page: &str) {
        let data = &mut self.state;
        tracing::debug!("on_page: {page:?}");
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
        ctx.submit_command(ENABLE_BACK_PAGE.with(enable_back).to(Target::Global));
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

impl AppWrapper {
    /// 创建 Iced 应用，读取配置并准备后台任务。
    pub fn boot() -> (Self, Task<Message>) {
        let mut config = match config::get_config_sync() {
            Ok(config) => config,
            Err(err) => {
                tracing::error!("读取配置文件失败，将使用默认配置：{err:?}");
                config::Config::default()
            }
        };
        config::apply_auth_override(&mut config, config::load_auth_data());
        let first_run = !config::config_path().exists();
        let mut state = AppState::new(config);
        state.first_run = first_run;
        if std::env::var_os("SCL_MOCK").is_some() {
            crate::app_state::apply_mock(&mut state);
        }

        let launcher = Self {
            state,
            started: false,
            last_page: String::new(),
            next_on_page_is_pop: false,
        };

        (launcher, iced::window::latest().map(Message::WindowReady))
    }

    /// Iced 的消息处理入口。
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let old_config = self.state.config.clone();
        let task = match message {
            Message::WindowReady(id) => {
                self.state.window = id;
                self.start();
                Task::none()
            }
            Message::Command(command) => self.handle_command_message(command),
            Message::Progress(progress) => {
                self.apply_progress(progress);
                Task::none()
            }
            Message::Window(message) => {
                let ctx = self.event_ctx();
                match message {
                    scl_gui_widgets::widgets::WindowMessage::Back => {
                        self.query_pop_page(&ctx, "");
                        Task::none()
                    }
                    scl_gui_widgets::widgets::WindowMessage::Drag => {
                        iced::window::drag(self.window_id())
                    }
                    scl_gui_widgets::widgets::WindowMessage::Minimize => {
                        iced::window::minimize(self.window_id(), true)
                    }
                    scl_gui_widgets::widgets::WindowMessage::Close => self.close_window(),
                }
            }
            Message::CloseWindow => self.close_window(),
            Message::HideWindow => {
                if let Some(id) = self.state.window {
                    iced::window::minimize(id, true)
                } else {
                    Task::none()
                }
            }
            Message::Quit => {
                self.save_before_quit();
                iced::exit()
            }
            Message::AsyncError(location, error) => {
                self.state.async_error = Some((location, error));
                Task::none()
            }
            Message::DismissAsyncError => {
                self.state.async_error = None;
                Task::none()
            }
            Message::CopyAsyncError => {
                if let Some((_, error)) = &self.state.async_error {
                    crate::panic::copy_to_clipboard(error.clone());
                }
                Task::none()
            }
            Message::Ui(message) => self.handle_ui_message(message),
            Message::Tick => {
                self.state.animation_phase = (self.state.animation_phase + 0.02) % 1.0;
                Task::none()
            }
            Message::PickFolder => Task::perform(
                async {
                    smol::unblock(|| {
                        rfd::FileDialog::new()
                            .set_title("请选择一个 .minecraft 游戏文件夹")
                            .pick_folder()
                    })
                    .await
                },
                Message::FolderPicked,
            ),
            Message::PickJava => Task::perform(
                async {
                    smol::unblock(|| {
                        rfd::FileDialog::new()
                            .set_title("请选择名称为 java 的运行时")
                            .pick_file()
                    })
                    .await
                },
                Message::JavaPicked,
            ),
            Message::SearchJava => {
                let ctx = self.event_ctx();
                java::search_java(&ctx, &self.state);
                Task::none()
            }
            Message::SaveMod => {
                let file_name = self
                    .state
                    .mod_file_downloads_selected
                    .as_ref()
                    .or_else(|| {
                        self.state
                            .mod_file_downloads_files
                            .as_ref()
                            .and_then(|files| files.front())
                    })
                    .map(|file| file.file_name.clone())
                    .unwrap_or_default();
                let mod_name = self
                    .state
                    .mod_file_downloads_info
                    .as_ref()
                    .map(|info| info.name.clone())
                    .unwrap_or_default();
                Task::perform(
                    async move {
                        smol::unblock(move || {
                            rfd::FileDialog::new()
                                .set_file_name(file_name)
                                .set_title(format!("请选择保存模组 {mod_name} 的路径"))
                                .save_file()
                        })
                        .await
                    },
                    Message::ModSavePicked,
                )
            }
            Message::FolderPicked(path) => {
                let task = match path {
                    Some(path) => {
                        let mut ctx = self.event_ctx();
                        let folder = crate::config::MinecraftFolder::Other(
                            path.to_string_lossy().to_string(),
                        );
                        add_minecraft_folder(&mut ctx, &mut self.state, folder);
                        Task::none()
                    }
                    None => Task::none(),
                };
                task
            }
            Message::JavaPicked(path) => {
                if let Some(path) = path {
                    if let Some(path) = path.to_str() {
                        let ctx = self.event_ctx();
                        java::manually_add_java(&ctx, &mut self.state, path);
                    }
                }
                Task::none()
            }
            Message::ModSavePicked(path) => {
                if let Some(path) = path {
                    mod_download_dest::save_mod(
                        &self.event_ctx(),
                        &mut self.state,
                        &FileInfo::new(path),
                    );
                }
                Task::none()
            }
        };

        // 设置页会直接改 config；统一监听并防抖落盘。
        if old_config != self.state.config {
            self.state.theme = crate::theme::theme_from_config(&self.state.config);
            save_config_debounced(&self.state);
        }

        task
    }

    /// Iced 的视图入口。
    pub fn view(&self) -> Element<'_, Message> {
        crate::ui::view(&self.state)
    }

    /// Iced 订阅：把后台任务通过全局队列发来的消息转换到界面上。
    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            Subscription::run(event_stream),
            iced::window::close_requests().map(|_| Message::Quit),
        ];

        if self.is_animating() {
            subscriptions.push(iced::window::frames().map(|_| Message::Tick));
        }

        Subscription::batch(subscriptions)
    }

    fn is_animating(&self) -> bool {
        if !self.state.progress.is_empty()
            || self.state.launching
            || self.state.login_busy
            || self.state.mods_download_curseforge_searching
            || self.state.mods_download_modrinth_searching
            || self.state.launching
        {
            return true;
        }

        match *self.state.pages.current() {
            PAGE_STARTING => true,
            PAGE_LOGIN_MICROSOFT => {
                self.state.login_error.is_none() && self.state.login_microsoft_code.is_empty()
                    || self.state.login_microsoft_success
            }
            PAGE_MODS_DOWNLOAD_CURSEFORGE => {
                self.state.mods_download_curseforge_result_mods.is_none()
            }
            PAGE_MODS_DOWNLOAD_MODRINTH => self.state.mods_download_modrinth_result_mods.is_none(),
            _ => false,
        }
    }

    /// Iced 的基础主题，仅用于输入框等标准控件；组件自身颜色由 SCL 主题控制。
    pub fn theme(&self) -> iced::Theme {
        if self.state.theme.is_dark() {
            iced::Theme::Dark
        } else {
            iced::Theme::Light
        }
    }

    fn event_ctx(&self) -> EventCtx {
        EventCtx::new(
            event_sink(),
            self.state.window.unwrap_or_else(window::Id::unique),
        )
    }

    fn window_id(&self) -> window::Id {
        self.state.window.unwrap_or_else(window::Id::unique)
    }

    fn start(&mut self) {
        if self.started {
            return;
        }
        self.started = true;

        let ctx = self.event_ctx();
        let sink = ctx.get_external_handle();
        if std::env::var_os("SCL_MOCK").is_none() {
            starting::init(sink.clone(), &self.state);
            if self.state.first_run {
                starting::init_settings(sink.clone(), self.state.config.clone());
            }
        }

        if let Ok(page) = std::env::var("SCL_START_PAGE") {
            if !page.is_empty() {
                let page: &'static str = Box::leak(page.into_boxed_str());
                tracing::debug!("test hook SCL_START_PAGE={page:?}: 2s 后自动 PUSH_PAGE");
                let sink = sink.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    trace_auto_page("push", page, "SCL_START_PAGE hook");
                    let _ = sink.submit_command(PUSH_PAGE, page, Target::Global);
                });
            }
        }

        ctx.submit_command(
            SET_SLIDE_PAGE_ANIMATION
                .with(self.state.config.theme.slide_page_animation)
                .to(Target::Global),
        );

        if !self
            .state
            .config
            .theme
            .theme_background_image_path
            .is_empty()
        {
            let path = PathBuf::from(&self.state.config.theme.theme_background_image_path);
            if let Ok(image) = ::image::open(path) {
                if let Ok(handle) = crate::utils::thread::image_handle_from_dynamic(image) {
                    let _ = sink.submit_command(SET_BACKGROUND_IMAGE, handle, Target::Global);
                }
            }
        }
    }

    fn handle_command_message(&mut self, command: Command) -> Task<Message> {
        let mut ctx = self.event_ctx();

        if let Some(page) = command.get(PUSH_PAGE) {
            let page = *page;
            self.push_page(&ctx, page);
        } else if let Some(page) = command.get(QUERY_POP_PAGE) {
            let page = *page;
            self.query_pop_page(&ctx, page);
        } else if let Some(page) = command.get(RESET_PAGE_TO) {
            let page = *page;
            self.reset_page(&ctx, page);
        } else if let Some(enabled) = command.get(ENABLE_BACK_PAGE) {
            self.state.back_enabled = *enabled;
        } else if let Some(slide) = command.get(SET_SLIDE_PAGE_ANIMATION) {
            self.state.slide_page_animation = *slide;
        } else if let Some(handle) = command.get(SET_BACKGROUND_IMAGE) {
            self.state.background_image = Some(handle.clone());
        } else if command.is(BACK_PAGE_CLICKED) {
            self.query_pop_page(&ctx, "");
        } else if command.is(QUERY_CLOSE_WINDOW) {
            return self.close_window();
        } else if let Some(event) = command.get(APP_EVENT) {
            let event = event.clone();
            self.handle_app_event(&mut ctx, event);
        } else {
            self.handle_command(&mut ctx, &command);
        }

        Task::none()
    }

    fn push_page(&mut self, ctx: &EventCtx, page: &'static str) {
        if *self.state.pages.current() == page {
            let mut ctx = ctx.clone();
            self.on_page(&mut ctx, page);
            return;
        }
        self.state.pages.push(page);
        let mut ctx = ctx.clone();
        self.on_page(&mut ctx, page);
    }

    fn query_pop_page(&mut self, ctx: &EventCtx, target: &'static str) {
        let old = *self.state.pages.current();
        if !target.is_empty() && old == target {
            return;
        }
        let changed = if target.is_empty() {
            self.state.pages.pop().is_some()
        } else {
            self.state.pages.pop_to(&target)
        };
        if !changed {
            return;
        }
        self.handle_pop_page(ctx, old);
        self.next_on_page_is_pop = true;
        let current = *self.state.pages.current();
        let mut ctx = ctx.clone();
        self.on_page(&mut ctx, current);
    }

    fn reset_page(&mut self, ctx: &EventCtx, page: &'static str) {
        let old = *self.state.pages.current();
        self.state.pages.reset_to(page);
        if old != page {
            self.handle_pop_page(ctx, old);
        }
        self.next_on_page_is_pop = false;
        let mut ctx = ctx.clone();
        self.on_page(&mut ctx, page);
    }

    fn handle_pop_page(&mut self, ctx: &EventCtx, page: &str) {
        let data = &mut self.state;
        if page == PAGE_LOGIN_MICROSOFT {
            data.login_microsoft_success = false;
            data.login_microsoft_code.clear();
            data.login_microsoft_url.clear();
            data.login_error = None;
            data.login_busy = false;
        } else if page == PAGE_VERSION_SETTING {
            // 官方在该页弹出时异步保存版本设置（含重命名）。
            let mut ctx = ctx.clone();
            version_setting::on_app_pop_page(&mut ctx, data);
        }
    }

    fn close_window(&mut self) -> Task<Message> {
        let last_page = self.last_page.clone();
        let data = &mut self.state;
        if last_page == PAGE_VERSION_SETTING
            || last_page == crate::ui::PAGE_VERSION_SETTING_MODS
            || last_page == crate::ui::PAGE_VERSION_SETTING_ADVANCE
            || last_page == crate::ui::PAGE_VERSION_SETTING_REMOVE
        {
            version_setting::save_on_close(data);
        }
        if let Err(err) = config::save_config_sync(&data.config) {
            tracing::error!("关闭前保存配置文件失败：{err:?}");
        }
        iced::exit()
    }

    fn save_before_quit(&mut self) {
        if let Err(err) = config::save_config_sync(&self.state.config) {
            tracing::error!("退出前保存配置文件失败：{err:?}");
        }
    }

    fn apply_progress(&mut self, message: ProgressMessage) {
        match message {
            ProgressMessage::New(id) => {
                self.state.progress.entry(id).or_default();
            }
            ProgressMessage::Remove(id) => {
                self.state.progress.remove(&id);
            }
            ProgressMessage::Add(id, value) => {
                if let Some(entry) = self.state.progress.get_mut(&id) {
                    entry.current += value;
                }
            }
            ProgressMessage::AddMax(id, value) => {
                if let Some(entry) = self.state.progress.get_mut(&id) {
                    entry.max = Some(entry.max.unwrap_or(0.0) + value);
                }
            }
            ProgressMessage::SetMax(id, value) => {
                if let Some(entry) = self.state.progress.get_mut(&id) {
                    entry.max = Some(value);
                }
            }
            ProgressMessage::Set(id, value) => {
                if let Some(entry) = self.state.progress.get_mut(&id) {
                    entry.current = value;
                }
            }
            ProgressMessage::SetMessage(id, value) => {
                if let Some(entry) = self.state.progress.get_mut(&id) {
                    entry.message = value;
                }
            }
            ProgressMessage::SetSubMessage(id, value) => {
                if let Some(entry) = self.state.progress.get_mut(&id) {
                    entry.sub_message = value;
                }
            }
            ProgressMessage::Hide(id) => {
                if let Some(entry) = self.state.progress.get_mut(&id) {
                    entry.max = None;
                }
            }
        }
    }

    /// 具体页面的输入消息由 `ui` 模块定义；这里集中处理需要改全局状态的输入。
    fn handle_ui_message(&mut self, message: crate::ui::UiMessage) -> Task<Message> {
        use crate::ui::{BoolField, ChoiceField, TextField, UiMessage};

        let mut ctx = self.event_ctx();
        let data = &mut self.state;

        match message {
            UiMessage::Text(field, value) => match field {
                TextField::VersionsFilter => data.versions_search_filter = value,
                TextField::DownloadVersionName => data.download_version_name = value,
                TextField::VersionRename => data.version_setting_version_name_rename = value,
                TextField::VersionModsFilter => {
                    data.version_mods_search_filter = value;
                    data.rebuild_mods_filter();
                }
                TextField::AccountsFilter => data.accounts_filter = value,
                TextField::OfflineName => data.login_offline_user_name = value,
                TextField::OfflineUuid => data.login_offline_uuid = value,
                TextField::AuthlibServer => data.login_authlib_server_url = value,
                TextField::AuthlibUser => data.login_authlib_username = value,
                TextField::AuthlibPassword => data.login_authlib_password = value,
                TextField::CurseforgeFilter => {
                    data.mods_download_curseforge_search_filter = value;
                }
                TextField::ModrinthFilter => {
                    data.mods_download_modrinth_search_filter = value;
                    ctx.submit_command(MR_APPLY_SEARCH.with(()).to(Target::Global));
                }
                TextField::GlobalMemory => {
                    data.setting_launch_java_memory_size = value.clone();
                    ctx.submit_command(SET_MEMORY_STRING.with(value).to(Target::Global));
                }
                TextField::JvmArgs => data.config.jvm_args = value,
                TextField::GameArgs => data.config.game_args = value,
                TextField::WrapperPath => data.config.wrapper_path = value,
                TextField::WrapperArgs => data.config.wrapper_args = value,
                TextField::VersionJvmArgs => data.version_setting_config.jvm_args = value,
                TextField::VersionGameArgs => data.version_setting_config.game_args = value,
                TextField::VersionJavaPath => data.version_setting_config.java_path = value,
                TextField::VersionWindowTitle => data.version_setting_config.window_title = value,
                TextField::VersionWrapperPath => data.version_setting_config.wrapper_path = value,
                TextField::VersionWrapperArgs => data.version_setting_config.wrapper_args = value,
                TextField::VersionMaxMemory => {
                    data.version_setting_config.max_mem_text = value;
                    if data.version_setting_config.override_max_mem {
                        data.version_setting_config.max_mem =
                            data.version_setting_config.max_mem_text.trim().parse().ok();
                    }
                }
            },
            UiMessage::Bool(field, value) => match field {
                BoolField::DarkTheme => data.config.theme.dark_theme = value,
                BoolField::SlideAnimation => {
                    data.config.theme.slide_page_animation = value;
                    data.slide_page_animation = value;
                }
                BoolField::AutoMemory => data.config.auto_mem_size = value,
                BoolField::WaitUntilLaunched => data.config.wait_until_launched = value,
                BoolField::DontCloseWindow => {
                    data.config.dont_close_window_when_launched = value;
                }
                BoolField::HideWindow => data.config.hide_window_when_launched = value,
                BoolField::AutoRecheck => {
                    data.config.auto_check_when_first_start_failed = value;
                }
                BoolField::VersionEnable => data.version_setting_config.enable = value,
                BoolField::VersionIndependent => {
                    data.version_setting_config.game_independent = value;
                }
                BoolField::VersionOverrideMemory => {
                    data.version_setting_config.override_max_mem = value;
                    if value {
                        data.version_setting_config.max_mem =
                            data.version_setting_config.max_mem_text.trim().parse().ok();
                    } else {
                        data.version_setting_config.max_mem = None;
                    }
                }
            },
            UiMessage::Choice(field, index) => match field {
                ChoiceField::VersionIndependentMethod => {
                    data.config.version_independent_method = match index {
                        0 => crate::config::VersionIndependentMethod::NoIndependent,
                        1 => crate::config::VersionIndependentMethod::IndependentForNonRelease,
                        2 => crate::config::VersionIndependentMethod::IndependentForModLoader,
                        3 => {
                            crate::config::VersionIndependentMethod::IndependentForModLoaderAndNonRelease
                        }
                        _ => crate::config::VersionIndependentMethod::IndependentAll,
                    };
                }
                ChoiceField::DownloadSource => {
                    data.config.download_source = match index {
                        1 => crate::core::DownloadSource::BmclApi,
                        2 => crate::core::DownloadSource::Mcbbs,
                        _ => crate::core::DownloadSource::Default,
                    };
                }
            },
            UiMessage::ToggleSort => {
                data.versions_sort_mode = match data.versions_sort_mode {
                    0 => 1,
                    1 => 2,
                    2 => 3,
                    _ => 0,
                };
            }
            UiMessage::ToggleFolder(key) => {
                if !data.collapsed_folders.remove(&key) {
                    data.collapsed_folders.insert(key);
                }
            }
            UiMessage::ToggleMod { path, enabled } => {
                if let Some(item) = data.version_mods.iter_mut().find(|item| item.path == path) {
                    item.enabled = enabled;
                }
                if let Some(item) = data
                    .version_mods_filtered
                    .iter_mut()
                    .find(|item| item.path == path)
                {
                    item.enabled = enabled;
                }
                ctx.submit_command(TOGGLE_MOD.with(path).to(Target::Global));
            }
            UiMessage::SelectLoaderVersion { kind, version } => {
                download::select_loader_version(data, kind, &version);
                self.query_pop_page(&ctx, "");
            }
            UiMessage::SelectVanillaVersion(name) => {
                let selected = data
                    .download_state_vanilla
                    .get()
                    .and_then(|versions| versions.all_versions().find(|x| x.name == name).cloned());
                if let Some(version) = selected {
                    data.selected_download_version = Some(version);
                    data.clear_loader_versions();
                    download::update_download_version_name(data);
                    download::reload_current_loader(&mut ctx, data);
                }
                self.query_pop_page(&ctx, "");
            }
            UiMessage::SetSettingPage(index) => {
                data.setting_page_index = index;
                if index == 2 && data.update_available.is_none() {
                    ctx.submit_command(CHECK_UPDATE.with(()).to(Target::Global));
                }
            }
            UiMessage::SetParallel(value) => {
                data.config.download_parallel_amount = value.min(256);
            }
            UiMessage::CopyText(text) => {
                crate::panic::copy_to_clipboard(text);
            }
            UiMessage::OpenModsPage => {
                if !data.config.selected_version.is_empty() {
                    data.mod_download_install_version = data.config.selected_version.clone();
                }
                self.push_page(&ctx, PAGE_MODS_DOWNLOAD);
            }
            UiMessage::OpenVersionDir(sub) => {
                version_setting::open_version_dir(data, sub);
            }
            UiMessage::InstallMod(version) => {
                mod_download_dest::install(&ctx, data, &version);
                self.query_pop_page(&ctx, "");
            }
            UiMessage::CheckUpdate => {
                data.update_available = None;
                data.update_logs = None;
                data.setting_about_update_logs = Some("检查更新中……".into());
                ctx.submit_command(CHECK_UPDATE.with(()).to(Target::Global));
            }
        }

        Task::none()
    }
}

fn event_stream() -> impl futures::Stream<Item = Message> {
    use futures::StreamExt;

    futures::stream::unfold(take_event_receiver(), |receiver| async move {
        let mut receiver = receiver?;
        receiver
            .next()
            .await
            .map(|message| (message, Some(receiver)))
    })
}

fn show_launch_error_page(ctx: &EventCtx) {
    let sink = ctx.get_external_handle();
    thread_main(async move {
        smol::Timer::after(std::time::Duration::from_millis(500)).await;
        trace_auto_page("reset", PAGE_MAIN, "launch error 500ms timer");
        let _ = sink.submit_command(crate::ui::RESET_PAGE_TO, PAGE_MAIN, Target::Global);
        smol::Timer::after(std::time::Duration::from_millis(250)).await;
        trace_auto_page("push", PAGE_LAUNCH_ERROR, "launch error 250ms timer");
        let _ = sink.submit_command(crate::ui::PUSH_PAGE, PAGE_LAUNCH_ERROR, Target::Global);
        Ok(())
    });
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
    thread_main(async move {
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
        )
        .await;
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
                if let Err(err) = downloader.install_authlib_injector().await {
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
                        match scl_core::auth::authlib::refresh_token(single, &client_token, true)
                            .await
                        {
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
