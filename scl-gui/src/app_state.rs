//! 启动器的全局界面状态。

use std::path::PathBuf;

use druid::{Data, ImageBuf, Lens};
use im::Vector;

pub use crate::core::LocalVersionInfo;
use crate::{
    config::Config,
    core::{AuthMethod, SCLLaunchConfig, VersionInfo},
};

/// 网络数据的状态
#[derive(Debug, Clone, Data, Lens, Default, PartialEq)]
pub struct NetworkData<T> {
    /// 是否正在加载
    pub loading: bool,
    /// 加载到的数据
    pub data: Option<T>,
    /// 加载失败时的错误信息
    pub error: Option<String>,
}

impl<T> NetworkData<T> {
    /// 创建一个已经加载完成的数据
    pub fn new(data: T) -> Self {
        Self {
            loading: false,
            data: Some(data),
            error: None,
        }
    }

    /// 创建一个正在加载的数据
    pub fn new_loading() -> Self {
        Self {
            loading: true,
            data: None,
            error: None,
        }
    }

    /// 创建一个加载失败的数据
    pub fn new_error(msg: impl Into<String>) -> Self {
        Self {
            loading: false,
            data: None,
            error: Some(msg.into()),
        }
    }

    /// 获取数据的引用
    pub fn get(&self) -> Option<&T> {
        self.data.as_ref()
    }
}

/// 下载页面中保存的状态
#[derive(Debug, Clone, Data, Lens, Default, PartialEq)]
pub struct DownloadState<T: Clone> {
    /// 可用的正式版本列表
    pub avaliable_versions: Vector<T>,
    /// 可用的快照版本列表
    pub avaliable_snapshot_versions: Vector<T>,
    /// 可用的旧版 Beta 列表
    pub avaliable_old_beta_versions: Vector<T>,
    /// 可用的旧版 Alpha 列表
    pub avaliable_old_alpha_versions: Vector<T>,
}

impl<T: Clone> DownloadState<T> {
    /// 获取所有版本
    pub fn all_versions(&self) -> impl Iterator<Item = &T> {
        self.avaliable_versions
            .iter()
            .chain(self.avaliable_snapshot_versions.iter())
            .chain(self.avaliable_old_beta_versions.iter())
            .chain(self.avaliable_old_alpha_versions.iter())
    }
}

/// 模组的来源
#[derive(Debug, Clone, Copy, Data, PartialEq, Eq, Default)]
pub enum ModSource {
    /// CurseForge
    #[default]
    CurseForge,
    /// Modrinth
    Modrinth,
}

/// 一个可下载的模组
#[derive(Debug, Clone, Data, Lens)]
pub struct ModState {
    /// 模组 ID
    pub id: String,
    /// 模组名称
    pub name: String,
    /// 模组简介
    pub description: String,
    /// 模组图标（官方格式的 ASCII85 编码字符串）
    pub icon: String,
    /// 模组来源
    pub source: ModSource,
    /// 下载量
    pub download_count: u64,
    /// 模组的字符串 ID（CF slug / MR slug），用于查询中文名
    pub slug: String,
    /// 模组中文名（来自 gitee scl-data 的 MCMOD 译名，可能为空）
    pub cname: String,
}

impl Default for ModState {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            icon: String::new(),
            source: ModSource::CurseForge,
            download_count: 0,
            slug: String::new(),
            cname: String::new(),
        }
    }
}

/// 一个可下载的模组文件
#[derive(Debug, Clone, Data, Lens, PartialEq, Eq, Default)]
pub struct ModFileState {
    /// 文件名
    pub file_name: String,
    /// 下载链接
    pub download_url: String,
    /// 支持的游戏版本
    pub game_versions: Vector<String>,
    /// 支持的模组加载器
    pub loaders: Vector<String>,
    /// 是否是推荐文件
    pub primary: bool,
}

/// 一个已经安装的模组
#[derive(Debug, Clone, Data, Lens)]
pub struct LocalMod {
    /// 模组名称
    pub name: String,
    /// 文件名
    pub file_name: String,
    /// 模组版本
    pub version: String,
    /// 是否启用
    pub enabled: bool,
    /// 模组图标
    pub icon: ImageBuf,
    /// 模组文件路径
    pub path: String,
}

impl Default for LocalMod {
    fn default() -> Self {
        Self {
            name: String::new(),
            file_name: String::new(),
            version: String::new(),
            enabled: true,
            icon: ImageBuf::empty(),
            path: String::new(),
        }
    }
}

/// 一个版本设置页三行数据的加载快照。
///
/// 按版本名缓存：同一版本在本次会话内只加载一次，从子页返回或
/// A→B→A 切换时直接复用，不再读盘重算。
#[derive(Debug, Clone, Data, Lens)]
pub struct VersionSettingCacheEntry {
    /// 版本名称
    pub version_name: String,
    /// 该版本的模组列表（含进入模组管理页时加载的图标）
    pub mods: Vector<LocalMod>,
    /// 模组行描述
    pub mods_desc: String,
    /// 地图行描述
    pub maps_desc: String,
    /// 资源包行描述
    pub resource_packs_desc: String,
    /// 写入缓存时的失效纪元；与当前纪元不同即视为过期
    pub epoch: u64,
}

/// 启动器全局状态
#[derive(Debug, Clone, Data, Lens)]
pub struct AppState {
    /// 配置文件
    pub config: Config,

    /// 已安装的所有版本
    pub versions: Vector<LocalVersionInfo>,
    /// 游戏文件夹页面中新增文件夹的输入框内容
    pub new_minecraft_folder: String,
    /// 版本列表搜索关键词
    pub versions_search_filter: String,
    /// 版本列表排序方式（与官方模式编号 0~3 对齐）：
    /// 0 = 按名称，1 = 发布时间图标（官方未实现时间序，实际按名称），
    /// 2 = 按下载（目录创建）时间，3 = 按版本类型
    pub versions_sort_mode: u8,

    /// 设置页面当前的分页索引
    pub setting_page_index: usize,
    /// 设置页面中的内存大小文本
    pub setting_launch_java_memory_size: String,
    /// 设置页面中的更新日志
    pub setting_about_update_logs: Option<String>,
    /// 设置页面中手动添加 Java 时输入的路径
    pub setting_java_manual_path: String,
    /// Java 运行时扫描后的界面提示（有候选解析失败时非空）
    pub java_scan_message: Option<String>,

    /// 启动游戏时的提示
    pub starting_tip: Option<String>,
    /// 游戏窗口是否已经出现（或进程以 0 退出），用于避免重复回主页
    pub launch_window_ready: bool,
    /// 启动失败时的日志
    pub launch_error_logs: Vector<String>,

    /// 下载时将要使用的版本名称
    pub download_version_name: String,
    /// 下载页面当前选中的加载器分页
    pub download_tab: usize,
    /// 下载页面当前选中的 Forge 版本
    pub download_forge_version: String,
    /// 下载页面当前选中的 NeoForge 版本
    pub download_neoforge_version: String,
    /// 下载页面当前选中的 Fabric 版本
    pub download_fabric_version: String,
    /// 下载页面当前选中的 QuiltMC 版本
    pub download_quiltmc_version: String,
    /// 下载页面当前选中的 Optifine 版本
    pub download_optifine_version: String,
    /// 原版下载状态
    pub download_state_vanilla: NetworkData<DownloadState<VersionInfo>>,
    /// Fabric 下载状态
    pub download_state_fabric: NetworkData<DownloadState<String>>,
    /// QuiltMC 下载状态
    pub download_state_quiltmc: NetworkData<DownloadState<String>>,
    /// Forge 下载状态
    pub download_state_forge: NetworkData<DownloadState<String>>,
    /// NeoForge 下载状态
    pub download_state_neoforge: NetworkData<DownloadState<String>>,
    /// Optifine 下载状态
    pub download_state_optifine: NetworkData<DownloadState<String>>,

    /// 当前正在编辑的版本设置
    pub version_setting_config: SCLLaunchConfig,
    /// 当前正在编辑的版本类型
    pub version_setting_version_type: u8,
    /// 当前正在编辑的版本名称
    pub version_setting_version_name: String,
    /// 版本重命名输入框内容
    pub version_setting_version_name_rename: String,
    /// 版本模组页面描述
    pub version_setting_mods_desc: String,
    /// 版本地图页面描述
    pub version_setting_maps_desc: String,
    /// 版本资源包页面描述
    pub version_setting_resource_packs_desc: String,
    /// 版本设置页加载批次号：进页/退出页时递增，用于丢弃迟到的后台回调
    pub version_setting_load_token: u64,
    /// 按版本缓存的三行数据；同版本返回时直接复用，不重读
    pub version_setting_cache: Vector<VersionSettingCacheEntry>,
    /// 缓存对应的失效纪元；纪元递增即表示文件被改动、缓存作废
    pub version_setting_cache_epoch: u64,
    /// 正在加载三行数据的版本名（空表示没有在途请求）
    pub version_setting_loading_for: String,
    /// 在途请求的加载 token
    pub version_setting_loading_token: u64,
    /// 按当前搜索词过滤后的模组列表（仅在搜索词/模组列表变化时重建），
    /// 避免滚动等每帧事件里重复克隆整个列表。
    pub version_mods_filtered: im::Vector<LocalMod>,
    /// 版本模组搜索关键词
    pub version_mods_search_filter: String,
    /// 当前版本的本地模组
    pub version_mods: Vector<LocalMod>,

    /// 账户列表搜索关键词
    pub accounts_filter: String,

    /// 离线登录用户名
    pub login_offline_user_name: String,
    /// 离线登录 UUID
    pub login_offline_uuid: String,
    /// Authlib 服务器地址
    pub login_authlib_server_url: String,
    /// Authlib 用户名
    pub login_authlib_username: String,
    /// Authlib 密码
    pub login_authlib_password: String,
    /// Authlib 可选择的角色
    pub login_authlib_available_characters: Vector<AuthMethod>,
    /// 是否正在进行登录
    pub login_busy: bool,
    /// 微软登录的设备码
    pub login_microsoft_code: String,
    /// 微软登录的验证链接
    pub login_microsoft_url: String,
    /// 微软登录是否已经验证成功
    pub login_microsoft_success: bool,
    /// 最近一次登录失败的信息
    pub login_error: Option<String>,
    /// 可用的启动器更新版本
    pub update_available: Option<String>,
    /// 更新日志
    pub update_logs: Option<String>,

    /// Modrinth 搜索关键词
    pub mods_download_modrinth_search_filter: String,
    /// Modrinth 搜索结果
    pub mods_download_modrinth_result_mods: Option<Vector<ModState>>,
    /// CurseForge 搜索关键词
    pub mods_download_curseforge_search_filter: String,
    /// CurseForge 搜索结果
    pub mods_download_curseforge_result_mods: Option<Vector<ModState>>,
    /// 模组文件下载页面中的模组信息
    pub mod_file_downloads_info: Option<ModState>,
    /// 模组文件下载页面中的文件列表
    pub mod_file_downloads_files: Option<Vector<ModFileState>>,
    /// 当前选择下载的模组文件
    pub mod_file_downloads_selected: Option<ModFileState>,

    /// 当前选中的版本
    pub selected_version: Option<LocalVersionInfo>,
    /// 当前正在下载的版本名称
    pub selected_download_version: Option<VersionInfo>,
    /// CurseForge 搜索结果页码
    pub mods_download_curseforge_page: u64,
    pub mods_download_curseforge_searching: bool,
    /// CurseForge 搜索请求代次（迟到回调丢弃；官方同代判定为 result 内页计数）
    pub mods_download_curseforge_search_generation: u64,
    /// Modrinth 搜索结果页码
    pub mods_download_modrinth_page: u64,
    pub mods_download_modrinth_searching: bool,
    /// Modrinth 搜索请求代次（官方以 SEARCH_TIMER 时刻做迟到结果丢弃）
    pub mods_download_modrinth_search_generation: u64,
    /// 模组文件列表请求代次（进入文件页时递增，用于丢弃迟到回调）
    pub mod_file_downloads_generation: u64,
    /// 待安装模组时选择的 Minecraft 版本
    pub mod_download_install_version: String,
    /// 最近一次启动的错误
    pub launch_error: Option<String>,
    /// 启动器是否正在启动游戏
    pub launching: bool,
    /// 首次启动失败后是否已安排“检查文件完整性并重新启动”
    pub auto_recheck_retry: bool,
    /// 是否第一次运行启动器（配置文件原本不存在）
    pub first_run: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let config = Config::default();
        Self::new(config)
    }
}

impl AppState {
    /// 查找当前纪元下指定版本的缓存快照。
    pub fn version_setting_cache_get(&self, name: &str) -> Option<&VersionSettingCacheEntry> {
        self.version_setting_cache.iter().find(|entry| {
            crate::app::version_setting::cache_entry_matches(
                &entry.version_name,
                entry.epoch,
                self.version_setting_cache_epoch,
                name,
            )
        })
    }

    /// 写入指定版本的缓存快照（覆盖同名旧条目，最多保留最近 8 个版本）。
    pub fn version_setting_cache_put(&mut self, entry: VersionSettingCacheEntry) {
        if let Some(index) = self
            .version_setting_cache
            .iter()
            .position(|item| item.version_name == entry.version_name)
        {
            self.version_setting_cache.remove(index);
        }
        self.version_setting_cache.push_back(entry);
        while self.version_setting_cache.len() > 8 {
            self.version_setting_cache.pop_front();
        }
    }

    /// 按当前搜索词重建模组过滤缓存（搜索词或模组列表变化时调用）。
    pub fn rebuild_mods_filter(&mut self) {
        let filter = self.version_mods_search_filter.trim().to_lowercase();
        self.version_mods_filtered = self
            .version_mods
            .iter()
            .filter(|item| {
                filter.is_empty()
                    || item.name.to_lowercase().contains(&filter)
                    || item.file_name.to_lowercase().contains(&filter)
            })
            .cloned()
            .collect();
    }

    /// 内存输入框的初始文本。
    ///
    /// 自动内存时配置里的 `mem_size` 是 0，直接回填会显示成真实文本 "0"，
    /// 用户输入 2048 就变成 "02048"（问题 ⑦）。自动模式留空以显示占位符，
    /// 手动模式才回填已保存的数值。
    fn initial_memory_input(config: &Config) -> String {
        if config.auto_mem_size {
            String::new()
        } else {
            config.mem_size.to_string()
        }
    }

    /// 使用配置创建一个新的全局状态
    pub fn new(config: Config) -> Self {
        let memory_size_str = Self::initial_memory_input(&config);
        Self {
            config,
            versions: Vector::new(),
            new_minecraft_folder: String::new(),
            versions_search_filter: String::new(),
            versions_sort_mode: 0,
            setting_page_index: 0,
            setting_launch_java_memory_size: memory_size_str,
            setting_about_update_logs: None,
            setting_java_manual_path: String::new(),
            java_scan_message: None,
            starting_tip: None,
            launch_window_ready: false,
            launch_error_logs: Vector::new(),
            download_version_name: String::new(),
            download_tab: 0,
            download_forge_version: String::new(),
            download_neoforge_version: String::new(),
            download_fabric_version: String::new(),
            download_quiltmc_version: String::new(),
            download_optifine_version: String::new(),
            download_state_vanilla: NetworkData::new_loading(),
            download_state_fabric: NetworkData::new_loading(),
            download_state_quiltmc: NetworkData::new_loading(),
            download_state_forge: NetworkData::new_loading(),
            download_state_neoforge: NetworkData::new_loading(),
            download_state_optifine: NetworkData::new_loading(),
            version_setting_config: SCLLaunchConfig::default(),
            version_setting_version_type: 0,
            version_setting_version_name: String::new(),
            version_setting_version_name_rename: String::new(),
            version_setting_mods_desc: String::new(),
            version_setting_maps_desc: String::new(),
            version_setting_resource_packs_desc: String::new(),
            version_setting_load_token: 0,
            version_setting_cache: Vector::new(),
            version_setting_cache_epoch: 0,
            version_setting_loading_for: String::new(),
            version_setting_loading_token: 0,
            version_mods_filtered: im::Vector::new(),
            version_mods_search_filter: String::new(),
            version_mods: Vector::new(),
            accounts_filter: String::new(),
            login_offline_user_name: String::new(),
            login_offline_uuid: String::new(),
            login_authlib_server_url: String::new(),
            login_authlib_username: String::new(),
            login_authlib_password: String::new(),
            login_authlib_available_characters: Vector::new(),
            login_busy: false,
            login_microsoft_code: String::new(),
            login_microsoft_url: String::new(),
            login_microsoft_success: false,
            login_error: None,
            update_available: None,
            update_logs: None,
            mods_download_modrinth_search_filter: String::new(),
            mods_download_modrinth_result_mods: None,
            mods_download_curseforge_search_filter: String::new(),
            mods_download_curseforge_result_mods: None,
            mod_file_downloads_info: None,
            mod_file_downloads_files: None,
            mod_file_downloads_selected: None,
            selected_version: None,
            selected_download_version: None,
            mods_download_curseforge_page: 0,
            mods_download_curseforge_searching: false,
            mods_download_curseforge_search_generation: 0,
            mods_download_modrinth_page: 0,
            mods_download_modrinth_searching: false,
            mods_download_modrinth_search_generation: 0,
            mod_file_downloads_generation: 0,
            mod_download_install_version: String::new(),
            launch_error: None,
            launching: false,
            auto_recheck_retry: false,
            first_run: false,
        }
    }

    /// 读取指定加载器当前选中的版本
    pub fn loader_version(&self, kind: &str) -> &str {
        match kind {
            "forge" => &self.download_forge_version,
            "neoforge" => &self.download_neoforge_version,
            "fabric" => &self.download_fabric_version,
            "quiltmc" => &self.download_quiltmc_version,
            "optifine" => &self.download_optifine_version,
            _ => "",
        }
    }

    /// 写入指定加载器当前选中的版本
    pub fn set_loader_version(&mut self, kind: &str, version: impl Into<String>) {
        let version = version.into();
        match kind {
            "forge" => self.download_forge_version = version,
            "neoforge" => self.download_neoforge_version = version,
            "fabric" => self.download_fabric_version = version,
            "quiltmc" => self.download_quiltmc_version = version,
            "optifine" => self.download_optifine_version = version,
            _ => {}
        }
    }

    /// 清空所有加载器的选中版本（切换原版版本时使用）
    pub fn clear_loader_versions(&mut self) {
        self.download_forge_version.clear();
        self.download_neoforge_version.clear();
        self.download_fabric_version.clear();
        self.download_quiltmc_version.clear();
        self.download_optifine_version.clear();
    }

    /// 获取 Minecraft 文件夹
    pub fn minecraft_folder(&self) -> PathBuf {
        self.config.minecraft_folder.to_path_buf()
    }

    /// 获取版本文件夹
    pub fn versions_folder(&self) -> PathBuf {
        self.minecraft_folder().join("versions")
    }

    /// 获取当前选中的账户
    pub fn current_account(&self) -> Option<AuthMethod> {
        self.config.selected_account()
    }

    /// 根据名称查找已经安装的版本
    pub fn find_version(&self, name: &str) -> Option<&LocalVersionInfo> {
        self.versions.iter().find(|x| x.name == name)
    }

    /// 更新当前选中的版本
    pub fn update_selected_version(&mut self) {
        self.selected_version = self.find_version(&self.config.selected_version).cloned();
    }
}

/// 填充用于对照录屏的模拟数据。
///
/// 仅在设置了 `SCL_MOCK` 环境变量时由 `main` 调用，方便在不进行任何交互的
/// 情况下逐个页面截图并与录屏抽帧对比。
pub fn apply_mock(state: &mut AppState) {
    use crate::{
        config::{AccountData, VersionIndependentMethod},
        core::{AuthMethod, DownloadSource, VersionInfo, VersionType},
    };

    let now = chrono::Local::now().timestamp_millis();

    state.config.theme.dark_theme = std::env::var("SCL_MOCK_DARK")
        .map(|value| value != "0")
        .unwrap_or(true);
    state.config.wait_until_launched = true;
    state.config.dont_close_window_when_launched = true;
    state.config.hide_window_when_launched = false;
    state.config.auto_check_when_first_start_failed = true;
    state.config.auto_mem_size = true;
    state.config.mem_size = 0;
    state.config.version_independent_method =
        VersionIndependentMethod::IndependentForModLoaderAndNonRelease;
    state.config.download_source = DownloadSource::BmclApi;
    state.config.download_parallel_amount = 65;

    state.config.accounts_data = AccountData {
        accounts: im::vector![AuthMethod::Microsoft {
            access_token: "mock-access-token".into(),
            refresh_token: "mock-refresh-token".into(),
            uuid: "00000000-0000-0000-0000-000000000000".into(),
            xuid: "0".into(),
            player_name: "fgfgui_link".into(),
            head_skin: Vec::new(),
            hat_skin: Vec::new(),
        }],
        selected: 0,
    };

    let version = |name: &str, version_type: VersionType, age: i64| LocalVersionInfo {
        name: name.to_owned(),
        version_type,
        path: "/tmp/scl-mock/.minecraft/versions".into(),
        created_date: now - age,
        access_date: now - age,
    };
    if std::env::var_os("SCL_MOCK_EMPTY_VERSIONS").is_some() {
        state.versions = im::Vector::new();
    } else {
        state.versions = im::vector![
            version("1.20.1-Fabric", VersionType::Fabric, 60_000),
            version("1.20.1", VersionType::Vanilla, 120_000),
            version("1.20.1-Forge", VersionType::Forge, 180_000),
            version("1.20.1-forge-47.4.10", VersionType::Forge, 240_000),
            version("1.21.1", VersionType::Vanilla, 300_000),
            version("1.21.1-NeoForge", VersionType::NeoForge, 360_000),
            version("1.21.4", VersionType::Vanilla, 420_000),
            version("1.21.4-Fabric", VersionType::Fabric, 480_000),
            version("1.21.4-NeoForge", VersionType::NeoForge, 540_000),
            version("1.21.8-NeoForge", VersionType::NeoForge, 600_000),
            version("26.1.2", VersionType::Vanilla, 660_000),
            version("26.1.2-Fabric", VersionType::Fabric, 720_000),
            version("26.2", VersionType::Vanilla, 780_000),
            version("26.3", VersionType::Vanilla, 840_000),
            version("26.3-Fabric", VersionType::Fabric, 900_000),
            version("26.3-snapshot-4", VersionType::Vanilla, 960_000),
            version("26.3FO", VersionType::Forge, 1_020_000),
            version("GregTech_Leisure", VersionType::Forge, 1_080_000),
            version("NAST easy c1.9.4", VersionType::Forge, 1_140_000),
            version("TeaCon2026modpack.c58149ed", VersionType::Forge, 1_200_000),
            version(
                "fabric-loader-0.16.14-1.20.1",
                VersionType::Fabric,
                1_260_000
            ),
            version("neoforge-21.1.172", VersionType::NeoForge, 1_320_000),
            version("neoforge-26.1.2.84", VersionType::NeoForge, 1_380_000),
            version("26.4-snapshot-6", VersionType::Vanilla, 1_440_000),
            version(".tmp", VersionType::Unknown, 1_500_000),
        ];
    }
    state.config.selected_version = "1.20.1-Fabric".into();
    state.update_selected_version();

    let vanilla = |name: &str, kind: &str| VersionInfo {
        name: name.to_owned(),
        version_type: kind.to_owned(),
        url: format!("https://mock.invalid/{name}.json"),
        time: "2026-01-01T00:00:00+00:00".into(),
        release_time: "2026-01-01T00:00:00+00:00".into(),
    };
    state.download_state_vanilla = NetworkData::new(DownloadState {
        avaliable_versions: im::vector![
            vanilla("26.4-snapshot-6", "release"),
            vanilla("26.3", "release"),
            vanilla("26.2", "release"),
            vanilla("26.1.2", "release"),
            vanilla("1.21.8", "release"),
            vanilla("1.21.7", "release"),
            vanilla("1.21.6", "release"),
            vanilla("1.21.5", "release"),
            vanilla("1.21.4", "release"),
            vanilla("1.21.1", "release"),
            vanilla("1.20.1", "release"),
            vanilla("1.20", "release"),
            vanilla("1.19.4", "release"),
            vanilla("1.18.2", "release"),
            vanilla("1.16.5", "release"),
            vanilla("1.12.2", "release"),
        ],
        avaliable_snapshot_versions: im::vector![
            vanilla("26.4-snapshot-5", "snapshot"),
            vanilla("26.4-snapshot-4", "snapshot"),
        ],
        avaliable_old_beta_versions: im::vector![],
        avaliable_old_alpha_versions: im::vector![],
    });
    state.selected_download_version = state.download_state_vanilla.get().map(|state| {
        state
            .avaliable_versions
            .iter()
            .find(|version| version.name == "26.3")
            .cloned()
            .unwrap_or_default()
    });
    let loader_versions = |prefix: &str, versions: &[&str]| DownloadState {
        avaliable_versions: versions
            .iter()
            .map(|version| format!("{prefix}{version}"))
            .collect(),
        avaliable_snapshot_versions: im::Vector::new(),
        avaliable_old_beta_versions: im::Vector::new(),
        avaliable_old_alpha_versions: im::Vector::new(),
    };
    state.download_state_fabric = NetworkData::new(loader_versions(
        "",
        &["0.16.14", "0.16.10", "0.15.11", "0.15.7", "0.14.24"],
    ));
    state.download_state_quiltmc =
        NetworkData::new(loader_versions("", &["0.27.1", "0.26.4", "0.25.0"]));
    state.download_state_forge = NetworkData::new(loader_versions(
        "",
        &["47.4.10", "47.3.0", "47.2.0", "47.1.3", "43.3.5"],
    ));
    state.download_state_neoforge = NetworkData::new(loader_versions(
        "",
        &["21.1.172", "21.1.169", "21.1.80", "20.6.119"],
    ));
    state.download_state_optifine =
        NetworkData::new(loader_versions("", &["1.20.1_HD_U_I6", "1.20.1_HD_U_I5"]));
    state.download_tab = std::env::var("SCL_MOCK_DOWNLOAD_TAB")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    state.download_version_name = "26.3".into();

    state.setting_page_index = std::env::var("SCL_MOCK_SETTINGS_TAB")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    state.setting_launch_java_memory_size = AppState::initial_memory_input(&state.config);
    state.setting_about_update_logs =
        Some("SCL 1.2.0\n- 修复了一些已知问题\n- 优化了下载速度\n- 支持 NeoForge 安装".into());
    state.update_logs = state.setting_about_update_logs.clone();
    state.update_available = Some("1.2.1".into());

    state.version_setting_version_name = "1.20.1-Fabric".into();
    state.version_setting_version_type =
        if std::env::var_os("SCL_MOCK_VERSION_SETTING_EMPTY").is_some() {
            0
        } else {
            3
        };
    state.version_setting_config = SCLLaunchConfig::default();
    // 官方页面在数据尚未加载时输入框与三条菜单描述都是空串（此时菜单右侧画箭头）。
    // 截图比对时用 SCL_MOCK_VERSION_SETTING_EMPTY=1 复现这个状态。
    if std::env::var_os("SCL_MOCK_VERSION_SETTING_EMPTY").is_none() {
        state.version_setting_version_name_rename = "1.20.1-Fabric".into();
        // 官方描述格式为「已安装 N」/「已游玩 N」，0 条时为「无」（见 on_app_push_page）。
        state.rebuild_mods_filter();
        state.version_setting_mods_desc = "已安装 19".into();
        state.version_setting_maps_desc = "无".into();
        state.version_setting_resource_packs_desc = "无".into();
    }

    let local_mod = |name: &str, version: &str| LocalMod {
        name: name.to_owned(),
        file_name: format!("{name}-{version}.jar"),
        version: version.to_owned(),
        enabled: true,
        icon: ImageBuf::empty(),
        path: format!("/tmp/scl-mock/mods/{name}-{version}.jar"),
    };
    state.version_mods = im::vector![
        local_mod("ModernFix", "5.25.2+mc1.20.1"),
        local_mod("GeckoLib 4", "4.8.4"),
        local_mod("AppleSkin", "2.5.2+mc1.20.1"),
        local_mod("Mouse Tweaks", "2.26"),
        local_mod("ImmediatelyFast", "1.5.5+1.20.4"),
        local_mod("Jade", "11.13.3+fabric"),
        local_mod("Randomlocke PvP", "1.3.0"),
    ];

    let mod_state = |name: &str, description: &str, source: ModSource, downloads: u64| ModState {
        id: name.to_lowercase().replace(' ', "-"),
        name: name.to_owned(),
        description: description.to_owned(),
        icon: String::new(),
        source,
        download_count: downloads,
        slug: String::new(),
        cname: String::new(),
    };
    let mod_result = im::vector![
        mod_state(
            "Fabric API",
            "Lightweight and modular API providing common hooks and intercompatibility measures.",
            ModSource::Modrinth,
            1_000_000
        ),
        mod_state(
            "Sodium",
            "A high-performance rendering engine replacement.",
            ModSource::Modrinth,
            900_000
        ),
        mod_state(
            "Iris Shaders",
            "A modern shader pack loader for Minecraft.",
            ModSource::Modrinth,
            800_000
        ),
        mod_state(
            "Entity Culling",
            "Using path-tracing to hide Block-/Entitys.",
            ModSource::Modrinth,
            700_000
        ),
        mod_state(
            "Cloth Config API",
            "Configuration Library for Minecraft Mods.",
            ModSource::Modrinth,
            600_000
        ),
        mod_state(
            "FerriteCore",
            "Memory usage optimizations.",
            ModSource::Modrinth,
            500_000
        ),
        mod_state(
            "Mod Menu",
            "Adds a mod menu to view the list of mods.",
            ModSource::Modrinth,
            400_000
        ),
    ];
    state.mods_download_modrinth_result_mods = Some(mod_result.clone());
    state.mods_download_curseforge_result_mods = Some(mod_result);
    state.mods_download_modrinth_search_filter = "fabric".into();
    state.mods_download_curseforge_search_filter = "fabric".into();
    state.mods_download_modrinth_page = 1;
    state.mods_download_curseforge_page = 1;
    state.version_mods_search_filter = String::new();
    state.mod_download_install_version = "1.20.1-Fabric".into();
    state.mod_file_downloads_info = Some(mod_state(
        "ModernFix",
        "ModernFix is an all-in-one mod that improves performance and fixes bugs.",
        ModSource::CurseForge,
        123_456,
    ));
    state.mod_file_downloads_files = Some(im::vector![ModFileState {
        file_name: "modernfix-5.25.2+mc1.20.1.jar".into(),
        download_url: "https://mock.invalid/modernfix.jar".into(),
        game_versions: im::vector!["1.20.1".to_owned()],
        loaders: im::vector!["fabric".to_owned()],
        primary: true,
    }]);
    state.mod_file_downloads_selected = state
        .mod_file_downloads_files
        .as_ref()
        .and_then(|files| files.front().cloned());

    state.login_offline_user_name = "fgfgui_link".into();
    state.login_offline_uuid = "00000000-0000-0000-0000-000000000000".into();
    state.login_microsoft_code = "47FC8SCT".into();
    state.login_microsoft_url = "https://microsoft.com/link".into();
    match std::env::var("SCL_MOCK_MICROSOFT_STATE").as_deref() {
        Ok("loading") => {
            state.login_microsoft_code.clear();
            state.login_microsoft_url.clear();
        }
        Ok("success") => {
            state.login_microsoft_success = true;
        }
        Ok("error") => {
            state.login_error = Some("模拟的 Microsoft 登录错误".into());
        }
        _ => {}
    }
    state.login_authlib_server_url = "https://littleskin.cn/api/yggdrasil".into();
    state.login_authlib_username = "fgfgui_link".into();
    state.login_authlib_password = "password".into();
    state.login_authlib_available_characters = im::vector![AuthMethod::Offline {
        player_name: "fgfgui_link".into(),
        uuid: "00000000-0000-0000-0000-000000000000".into(),
    }];

    // SCL_MOCK_TIP：注入用于截图对照的固定贴士文本。
    state.starting_tip = Some(std::env::var("SCL_MOCK_TIP").unwrap_or_else(|_| {
        "你知道吗？\n\nSCL 启动器第一次出现是在小萧的教室电脑里，那时是 2021 年的一月呢~".into()
    }));
    // 版本设置页的根容器在 starting_tip 非空时整体禁用；直接截图该页时清掉提示。
    if std::env::var("SCL_MOCK_PAGE")
        .map(|page| page.contains("version_setting"))
        .unwrap_or(false)
    {
        state.starting_tip = None;
    }
    state.launch_error_logs = im::vector![
        "[INFO] 检查版本文件……".to_owned(),
        "[ERROR] 启动失败：未找到有效的 Java 运行时".to_owned(),
    ];
    state.launch_error = Some("启动失败：未找到有效的 Java 运行时".into());
    state.config.minecraft_folder = crate::config::MinecraftFolder::CurrentDirectory;
    state.config.minecraft_folders = im::Vector::new();
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use crate::config::Config;

    #[test]
    fn auto_memory_leaves_input_empty_for_placeholder() {
        // 问题 ⑦：自动内存时 mem_size 是 0，输入框不能把 "0" 当真实文本显示。
        let config = Config::default();
        assert!(config.auto_mem_size);
        assert_eq!(AppState::initial_memory_input(&config), "");
        assert_eq!(
            AppState::new(config).setting_launch_java_memory_size,
            ""
        );
    }

    #[test]
    fn manual_memory_restores_saved_value() {
        let mut config = Config::default();
        config.auto_mem_size = false;
        config.mem_size = 2048;
        assert_eq!(AppState::initial_memory_input(&config), "2048");
        assert_eq!(
            AppState::new(config).setting_launch_java_memory_size,
            "2048"
        );
    }
}
