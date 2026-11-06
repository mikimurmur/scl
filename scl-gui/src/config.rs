//! 启动器配置文件。
//!
//! 配置文件保存在启动器工作目录下的 `.scl.json` 中。所有字段都提供了默认值，
//! 因此旧版本的配置文件也可以被正常读取。

use std::{
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use anyhow::Context;
use druid::{Color, Data, Lens};
use serde::{Deserialize, Serialize};

use crate::{
    core::{AuthMethod, DownloadSource, VersionType},
    theme::icons::IconThemeConfig,
};

/// 启动器配置文件
#[derive(Debug, Clone, Data, Lens, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// 账户数据（官方不写入本文件，仅存 `~/.scl/auth_data.json`；
    /// 反序列化保留是为了兼容旧版本地文件，读取后由 auth 文件覆盖）
    #[serde(skip_serializing)]
    pub accounts_data: AccountData,
    /// 当前选中的版本名称
    pub selected_version: String,
    /// Java 运行时数据
    pub javas: JavaListData,
    /// 启动游戏时是否等待窗口出现
    pub wait_until_launched: bool,
    /// 版本独立方案
    pub version_independent_method: VersionIndependentMethod,
    /// 启动游戏后是否不关闭启动器
    pub dont_close_window_when_launched: bool,
    /// 启动游戏后是否隐藏启动器
    pub hide_window_when_launched: bool,
    /// 首次启动失败时是否自动检查
    pub auto_check_when_first_start_failed: bool,
    /// 是否自动设置内存
    pub auto_mem_size: bool,
    /// 手动设置的内存大小
    pub mem_size: usize,
    /// 额外 JVM 参数
    pub jvm_args: String,
    /// 额外游戏参数
    pub game_args: String,
    /// 包装器路径
    pub wrapper_path: String,
    /// 包装器参数
    pub wrapper_args: String,
    /// 主题设置
    pub theme: ThemeConfig,
    /// 下载源
    pub download_source: DownloadSource,
    /// 下载并发数
    pub download_parallel_amount: usize,
    /// 登录令牌
    pub client_token: String,
    /// 当前 Minecraft 文件夹
    pub minecraft_folder: MinecraftFolder,
    /// 曾经使用过的 Minecraft 文件夹（纯路径字符串数组，与官方一致；
    /// 官方样例 `minecraft_folders` 直接是字符串，不是 `{"Other": ...}` 形态）
    pub minecraft_folders: im::Vector<String>,
    /// 最近使用的 Authlib 服务器
    pub last_authlib_server: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            selected_version: String::new(),
            minecraft_folder: MinecraftFolder::CurrentDirectory,
            minecraft_folders: im::Vector::new(),
            theme: ThemeConfig::default(),
            accounts_data: AccountData::default(),
            javas: JavaListData::default(),
            wait_until_launched: true,
            version_independent_method:
                VersionIndependentMethod::IndependentForModLoaderAndNonRelease,
            dont_close_window_when_launched: false,
            hide_window_when_launched: false,
            auto_check_when_first_start_failed: true,
            auto_mem_size: true,
            mem_size: 0,
            jvm_args: String::new(),
            game_args: String::new(),
            wrapper_path: String::new(),
            wrapper_args: String::new(),
            download_source: DownloadSource::Default,
            download_parallel_amount: 64,
            client_token: generate_client_token(),
            last_authlib_server: String::new(),
        }
    }
}

/// 官方自动内存公式的纯函数版本。
pub fn automated_max_memory(free_mb: u64, mod_count: usize) -> u64 {
    let (mem_min, mem_t1, mem_t2, mem_t3) = if mod_count > 0 {
        let n = mod_count as i64;
        (400 + 7 * n, 1500 + 10 * n, 3000 + 17 * n, 6000 + 34 * n)
    } else {
        (300, 1500, 2500, 4000)
    };

    // Stage 1：保留 100MB，第一档直接取 min(可用, T1)。
    let usable = (free_mb as i64 - 100).max(0);
    let mut result = usable.min(mem_t1);
    let mut remaining = usable - mem_t1 - 100;
    if remaining < 100 {
        return result.max(mem_min) as u64;
    }

    // Stage 2：额外预留 200MB，按 80% 可用内存增量，封顶 T2-T1。
    let delta2 = mem_t2 - mem_t1;
    let numerator = ((usable - mem_t1 - 200) as f64 * 0.8) as i64;
    result += numerator.min(delta2);
    remaining = usable - mem_t1 - (delta2 as f64 / 0.8) as i64 - 300;
    if remaining < 100 {
        return result.max(mem_min) as u64;
    }

    // Stage 3：官方在这里复用 T2-T1 作为增量上限（不是 T3-T2）。
    let numerator = (((remaining.max(200) - 200) as f64) * 0.6) as i64;
    result += numerator.min(delta2);
    remaining = remaining.max(200) - (delta2 as f64 / 0.6) as i64 - 400;
    if remaining < 100 {
        return result.max(mem_min) as u64;
    }

    // Stage 4：按 40% 增量，封顶 T3。
    let numerator = (((remaining.max(300) - 300) as f64) * 0.4) as i64;
    result += numerator.min(mem_t3);
    result.max(mem_min) as u64
}

impl Config {
    /// 获取当前选中的账户
    pub fn selected_account(&self) -> Option<AuthMethod> {
        self.accounts_data
            .accounts
            .get(self.accounts_data.selected)
            .cloned()
    }

    /// 获取指定名称的 Java 运行时
    ///
    /// PATH 默认项中 Java 8+ 和 Java 16+ 的 path 都是 `"java"`，仅按 path
    /// 查找会永远落到列表第一条（Java 8+）。这里与官方一致，用
    /// path + main_version 归属同一套键：先找选中 path 下 8+/16+ 对应的一条，
    /// 否则退回到任意同 path 项、再退回到任意合适主版本项。
    pub fn java_for_version(&self, required_java: u8) -> Option<&JavaRuntimeInfo> {
        let modern = required_java >= 16;
        let selected = if modern {
            &self.javas.selected_modern
        } else {
            &self.javas.selected_leagzy
        };
        let same_bucket = |info: &&JavaRuntimeInfo| (info.main_version >= 16) == modern;
        self.javas
            .javapaths
            .iter()
            .find(|x| x.path == *selected && same_bucket(x))
            .or_else(|| self.javas.javapaths.iter().find(|x| &x.path == selected))
            .or_else(|| self.javas.javapaths.iter().find(same_bucket))
            .or_else(|| self.javas.javapaths.front())
    }
}

/// 一个 Minecraft 文件夹
#[derive(Debug, Clone, Data, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MinecraftFolder {
    /// 当前目录下的 `.minecraft`
    #[default]
    CurrentDirectory,
    /// 用户目录下的 `.minecraft`
    OfficalDirectory,
    /// 其它文件夹
    Other(String),
}

impl MinecraftFolder {
    /// 转换为路径
    pub fn to_path_buf(&self) -> PathBuf {
        match self {
            Self::CurrentDirectory => PathBuf::from(".minecraft"),
            Self::OfficalDirectory => {
                #[cfg(target_os = "windows")]
                let base = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
                #[cfg(not(target_os = "windows"))]
                let base = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                Path::new(&base).join(".minecraft")
            }
            Self::Other(path) => PathBuf::from(path),
        }
    }
}

impl From<String> for MinecraftFolder {
    fn from(value: String) -> Self {
        Self::Other(value)
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// 主题设置
#[derive(Debug, Clone, Data, Lens, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct ThemeConfig {
    /// 是否使用黑暗模式
    ///
    /// 官方行为：`.scl.json` 可以读入 `dark_theme: true` 并立即以暗色启动；
    /// 保存时只在 `true` 时写回该字段，`false` 与默认值一样省略
    /// （官方亮色配置的 `theme` 里没有这个字段）。
    #[serde(default, skip_serializing_if = "is_false")]
    pub dark_theme: bool,
    /// 是否使用滑动页面切换动画
    pub slide_page_animation: bool,
    /// 图标主题
    pub icons_theme: IconThemeConfig,
    /// 背景图片路径
    #[serde(skip_serializing_if = "String::is_empty")]
    pub theme_background_image_path: String,
    /// 次要颜色
    #[serde(default, with = "color_serde", skip_serializing_if = "Option::is_none")]
    pub theme_color_secondary: Option<Color>,
    /// 主要颜色
    #[serde(default, with = "color_serde", skip_serializing_if = "Option::is_none")]
    pub theme_color_primary: Option<Color>,
    /// 强调色
    #[serde(default, with = "color_serde", skip_serializing_if = "Option::is_none")]
    pub theme_color_accent: Option<Color>,
    /// 强调色 1
    #[serde(default, with = "color_serde", skip_serializing_if = "Option::is_none")]
    pub theme_color_accent_1: Option<Color>,
    /// 深强调色
    #[serde(default, with = "color_serde", skip_serializing_if = "Option::is_none")]
    pub theme_color_accent_dark_1: Option<Color>,
    /// 浅强调色
    #[serde(default, with = "color_serde", skip_serializing_if = "Option::is_none")]
    pub theme_color_accent_light_1: Option<Color>,
    /// 标题栏颜色
    #[serde(default, with = "color_serde", skip_serializing_if = "Option::is_none")]
    pub theme_color_title_bar: Option<Color>,
    /// 背景颜色
    #[serde(default, with = "color_serde", skip_serializing_if = "Option::is_none")]
    pub theme_color_background: Option<Color>,
}

/// 颜色字段的序列化模块。
///
/// 配置文件中颜色使用 `#RRGGBBAA` 字符串保存，同时也兼容旧版本的
/// `[r, g, b, a]` 数组格式。
mod color_serde {
    use druid::Color;
    use serde::{de::Error as _, Deserialize, Deserializer, Serializer};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ColorRepr {
        Hex(String),
        Bytes([u8; 4]),
        Floats([f64; 4]),
    }

    pub fn serialize<S>(value: &Option<Color>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(color) => {
                let (r, g, b, a) = color.as_rgba8();
                serializer.serialize_str(&format!("#{r:02x}{g:02x}{b:02x}{a:02x}"))
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = Option::<ColorRepr>::deserialize(deserializer)?;
        match repr {
            None => Ok(None),
            Some(ColorRepr::Hex(text)) => parse_hex(&text)
                .map(Some)
                .ok_or_else(|| D::Error::custom(format!("无效的颜色值：{text}"))),
            Some(ColorRepr::Bytes([r, g, b, a])) => Ok(Some(Color::rgba8(r, g, b, a))),
            Some(ColorRepr::Floats([r, g, b, a])) => Ok(Some(Color::rgba(r, g, b, a))),
        }
    }

    fn parse_hex(text: &str) -> Option<Color> {
        let hex = text.trim().trim_start_matches('#');
        let parse = |range: std::ops::Range<usize>| u8::from_str_radix(hex.get(range)?, 16).ok();
        match hex.len() {
            6 => Some(Color::rgb8(parse(0..2)?, parse(2..4)?, parse(4..6)?)),
            8 => Some(Color::rgba8(
                parse(0..2)?,
                parse(2..4)?,
                parse(4..6)?,
                parse(6..8)?,
            )),
            3 => {
                let expand = |value: &str| u8::from_str_radix(&format!("{value}{value}"), 16).ok();
                Some(Color::rgb8(
                    expand(hex.get(0..1)?)?,
                    expand(hex.get(1..2)?)?,
                    expand(hex.get(2..3)?)?,
                ))
            }
            _ => None,
        }
    }
}

/// 账户数据
#[derive(Debug, Clone, Data, Lens, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct AccountData {
    /// 当前选中的账户索引
    pub selected: usize,
    /// 所有账户
    pub accounts: im::Vector<AuthMethod>,
}

/// Java 运行时数据
#[derive(Debug, Clone, Data, Lens, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct JavaListData {
    /// 选中的 Java 8 运行时路径
    pub selected_leagzy: String,
    /// 选中的 Java 16+ 运行时路径
    pub selected_modern: String,
    /// 所有已发现的 Java 运行时
    pub javapaths: im::Vector<JavaRuntimeInfo>,
}

impl Default for JavaListData {
    fn default() -> Self {
        #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
        let mut javapaths = vec![
            JavaRuntimeInfo {
                path: "java".into(),
                name: "PATH 环境变量里的 Java 8+".into(),
                version: "1.8".into(),
                is_64bit: cfg!(target_pointer_width = "64"),
                main_version: 8,
            },
            JavaRuntimeInfo {
                path: "java".into(),
                name: "PATH 环境变量里的 Java 16+".into(),
                version: "16".into(),
                is_64bit: cfg!(target_pointer_width = "64"),
                main_version: 16,
            },
        ];
        #[cfg(target_os = "windows")]
        javapaths.extend([
            JavaRuntimeInfo {
                path: "javaw".into(),
                name: "PATH 环境变量里的 Java 8+ (无窗口)".into(),
                version: "1.8".into(),
                is_64bit: cfg!(target_pointer_width = "64"),
                main_version: 8,
            },
            JavaRuntimeInfo {
                path: "javaw".into(),
                name: "PATH 环境变量里的 Java 16+ (无窗口)".into(),
                version: "16".into(),
                is_64bit: cfg!(target_pointer_width = "64"),
                main_version: 16,
            },
        ]);
        Self {
            selected_leagzy: "java".into(),
            selected_modern: "java".into(),
            javapaths: javapaths.into(),
        }
    }
}

/// 一个 Java 运行时
#[derive(Debug, Clone, Data, Lens, PartialEq, Eq, Default)]
pub struct JavaRuntimeInfo {
    /// Java 可执行文件路径
    pub path: String,
    /// 用于展示的名称
    pub name: String,
    /// 完整版本号
    pub version: String,
    /// 是否为 64 位
    pub is_64bit: bool,
    /// 主版本号
    pub main_version: u8,
}

/// 旧版本配置中的 Java 版本字段，可能是字符串或整数
#[derive(Deserialize)]
#[serde(untagged)]
enum JavaVersionRepr {
    Number(u64),
    String(String),
}

/// 旧版本配置中的 Java 运行时结构
#[derive(Deserialize, Default)]
#[serde(default)]
struct JavaRuntimeInfoRepr {
    path: String,
    name: String,
    version: Option<JavaVersionRepr>,
    is_64bit: Option<bool>,
    main_version: Option<u8>,
}

impl Serialize for JavaRuntimeInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("JavaRuntimeInfo", 5)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("selected", &true)?;
        state.serialize_field("arch", std::env::consts::ARCH)?;
        state.serialize_field("version", &self.main_version)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for JavaRuntimeInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let repr = JavaRuntimeInfoRepr::deserialize(deserializer)?;
        let version = match repr.version {
            Some(JavaVersionRepr::Number(version)) => {
                if version == 8 {
                    "1.8".to_owned()
                } else {
                    version.to_string()
                }
            }
            Some(JavaVersionRepr::String(version)) => version,
            None => String::new(),
        };
        let main_version = repr
            .main_version
            .unwrap_or_else(|| java_main_version(&version));
        Ok(Self {
            path: repr.path,
            name: repr.name,
            version,
            is_64bit: repr.is_64bit.unwrap_or(cfg!(target_pointer_width = "64")),
            main_version,
        })
    }
}

/// 从完整 Java 版本号中解析主版本号
fn java_main_version(version: &str) -> u8 {
    let version = version.trim();
    if let Some(rest) = version.strip_prefix("1.") {
        rest.split(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    } else {
        version
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }
}

impl JavaRuntimeInfo {
    /// 判断两个 Java 运行时是否是同一项。
    ///
    /// 配置文件的 `version` 字段只持久化主版本号（见 [`Serialize`] 实现），
    /// 回读后是 "21"，而重新扫描得到的是完整版本号 "21.0.11"；把完整版本号
    /// 纳入比较会让同一个运行时在每次扫描后都被当成新条目累积（问题 ⑩）。
    /// 因此以「可执行文件路径 + 主版本号」作为身份键，完整版本号只用于展示。
    pub fn is_same_rt(&self, other: &Self) -> bool {
        self.path == other.path && self.main_version == other.main_version
    }
}

/// 版本独立方案
#[derive(Debug, Clone, Copy, PartialEq, Eq, Data, Serialize, Deserialize, Default)]
pub enum VersionIndependentMethod {
    /// 不使用版本独立
    #[default]
    NoIndependent,
    /// 非正式版本使用版本独立
    IndependentForNonRelease,
    /// 模组加载器版本使用版本独立
    IndependentForModLoader,
    /// 模组加载器版本和非正式版本使用版本独立
    IndependentForModLoaderAndNonRelease,
    /// 全部版本使用版本独立
    IndependentAll,
}

impl VersionIndependentMethod {
    pub fn should_apply(
        self,
        minecraft_version: &scl_core::semver::MinecraftVersion,
        version_type: VersionType,
    ) -> bool {
        let is_mod_loader = matches!(version_type, VersionType::Forge | VersionType::Fabric);
        let is_non_release = !matches!(
            minecraft_version,
            scl_core::semver::MinecraftVersion::Release(..)
        );
        match self {
            Self::NoIndependent => false,
            Self::IndependentForNonRelease => is_non_release,
            Self::IndependentForModLoader => is_mod_loader,
            Self::IndependentForModLoaderAndNonRelease => is_mod_loader || is_non_release,
            Self::IndependentAll => true,
        }
    }
}

/// 保存配置文件的互斥锁
static SAVE_CONFIG_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn save_config_mutex() -> &'static Mutex<()> {
    SAVE_CONFIG_MUTEX.get_or_init(|| Mutex::new(()))
}

/// 获取配置文件路径
pub fn config_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".scl.json")
}

/// 官方账户数据的存放目录：`~/.scl`
fn auth_data_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".scl"))
}

/// 官方账户数据文件路径：`~/.scl/auth_data.json`
fn auth_data_path() -> Option<PathBuf> {
    auth_data_dir().map(|dir| dir.join("auth_data.json"))
}

/// 同步读取配置文件
pub fn get_config_sync() -> anyhow::Result<Config> {
    let path = config_path();
    let data = match fs_err::read_to_string(&path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Config::default());
        }
        Err(err) => {
            return Err(err).with_context(|| format!("读取配置文件 {} 失败", path.display()))
        }
    };
    let config: Config = serde_json::from_str(data.trim_start_matches('\u{feff}'))
        .with_context(|| format!("解析配置文件 {} 失败", path.display()))?;
    Ok(config)
}

/// 读取 `~/.scl/auth_data.json` 中的账户数据
pub fn load_auth_data() -> Option<AccountData> {
    let path = auth_data_path()?;
    if !path.is_file() {
        return None;
    }
    load_auth_data_from(&path)
}

/// 从指定路径读取账户数据（供测试与 [`load_auth_data`] 复用）
fn load_auth_data_from(path: &Path) -> Option<AccountData> {
    let data = match fs_err::read_to_string(path) {
        Ok(data) => data,
        Err(err) => {
            tracing::error!("读取账户配置信息 {} 失败：{err}", path.display());
            return None;
        }
    };
    match serde_json::from_str(data.trim_start_matches('\u{feff}')) {
        Ok(accounts) => Some(accounts),
        Err(err) => {
            tracing::error!("解析账户配置信息 {} 失败：{err}", path.display());
            None
        }
    }
}

/// 用 `~/.scl/auth_data.json` 覆盖配置里的账户数据（官方启动时的读取优先级）
///
/// 官方先读 `.scl.json` 再读 `auth_data.json`，后者存在即覆盖；
/// 即使后者里账户列表为空也以它为准（auth 文件一旦存在就是账户的唯一权威来源）。
pub fn apply_auth_override(config: &mut Config, auth: Option<AccountData>) {
    if let Some(accounts) = auth {
        config.accounts_data = accounts;
    }
}

/// 把账户数据写入指定目录下的 `auth_data.json`（紧凑 JSON）
fn save_auth_data_in_dir_sync(dir: &Path, accounts: &AccountData) -> anyhow::Result<()> {
    if !dir.is_dir() {
        fs_err::create_dir_all(dir).with_context(|| format!("创建目录 {} 失败", dir.display()))?;
    }
    let path = dir.join("auth_data.json");
    let data = serde_json::to_vec(accounts)?;
    crate::utils::prelude::write_file_safe_sync(&path, data).context("无法写入账户配置信息")?;
    Ok(())
}

/// 按官方双轨存储保存账户数据到 `~/.scl/auth_data.json`
fn save_auth_data_sync(accounts: &AccountData) -> anyhow::Result<()> {
    match auth_data_dir() {
        Some(dir) => save_auth_data_in_dir_sync(&dir, accounts),
        None => {
            tracing::warn!("警告：用户不存在用户文件夹！将无法保存账户令牌！");
            Ok(())
        }
    }
}

/// 保存配置文件
///
/// 顺序：加锁 → 日志「正在后台保存配置文件」→
/// 先写 `~/.scl/auth_data.json`（紧凑）→ 再写 `./.scl.json`（pretty）→
/// 日志「配置文件后台保存完成」。
pub fn save_config(config: &Config) -> anyhow::Result<()> {
    tracing::info!("正在后台保存配置文件");
    let _guard = save_config_mutex()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    save_auth_data_sync(&config.accounts_data)?;
    let data = serde_json::to_vec_pretty(config)?;
    let path = config_path();
    crate::utils::prelude::write_file_safe_sync(&path, data)
        .with_context(|| format!("写入配置文件 {} 失败", path.display()))?;
    tracing::info!("配置文件后台保存完成");
    Ok(())
}

/// 同步保存配置文件
pub fn save_config_sync(config: &Config) -> anyhow::Result<()> {
    let _guard = save_config_mutex()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    save_auth_data_sync(&config.accounts_data)?;
    let data = serde_json::to_vec_pretty(config)?;
    save_config_data_sync(&config_path(), &data)
}

/// 同步写入配置数据：临时文件 + rename，关窗瞬间崩溃也不会截断 `.scl.json`。
fn save_config_data_sync(path: &Path, data: &[u8]) -> anyhow::Result<()> {
    crate::utils::prelude::write_file_safe_sync(path, data)
        .with_context(|| format!("写入配置文件 {} 失败", path.display()))
}

/// 生成一个随机的客户端令牌
fn generate_client_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id() as u128;
    let mut state = nanos ^ (pid << 64);
    let mut bytes = [0u8; 16];
    for byte in &mut bytes {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = state as u8;
    }
    // 官方配置中的 client_token 是 32 位小写十六进制（无连字符），
    // 例如 2275955bdb930d290304a4e08f7bdb5d。
    format!(
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

#[cfg(test)]
mod automated_memory_tests {
    use super::automated_max_memory;

    #[test]
    fn matches_official_901m_sample() {
        // -Xmx901m 对应 MemFree≈1001MB（保留 100MB）。
        assert_eq!(automated_max_memory(1001, 0), 901);
        assert_eq!(automated_max_memory(1001, 29), 901);
        assert_eq!(automated_max_memory(1000, 0), 900);
    }

    #[test]
    fn matches_recovered_1792_and_2238_inputs() {
        // 1792/2238 是同一公式在可用内存更充足时的输出，不是另一个公式。
        assert_eq!(automated_max_memory(1892, 40), 1792);
        assert_eq!(automated_max_memory(2650, 29), 2238);
        assert_eq!(automated_max_memory(2048, 0), 1698);
    }

    #[test]
    fn matches_official_stage_reserve_accounting() {
        // 旧实现少扣 100MB 预留，会在该输入下多给 1MB。
        assert_eq!(automated_max_memory(3252, 0), 2500);
        // 高内存+40 个模组时旧实现最多会多算约 120MB。
        assert_eq!(automated_max_memory(11999, 40), 6983);
    }

    #[test]
    fn clamps_to_official_minimum() {
        assert_eq!(automated_max_memory(0, 0), 300);
        assert_eq!(automated_max_memory(0, 40), 680);
    }

    #[test]
    fn handles_tiny_free_memory_without_negative_result() {
        // free < 100MB 时 usable 归零，只返回下限；不能出现负数或 panic。
        for free in [0u64, 1, 50, 99, 100, 101] {
            assert_eq!(automated_max_memory(free, 0), 300, "free={free}");
            assert_eq!(automated_max_memory(free, 40), 680, "free={free}");
        }
    }

    #[test]
    fn handles_large_mod_count_without_overflow() {
        // 100 万个模组：阈值仍在 i64 范围内，不应 panic，结果保持正值。
        let free = 0;
        assert_eq!(automated_max_memory(free, 1_000_000), 7_000_400);
        let value = automated_max_memory(100_000_000, 1_000_000);
        assert!(value > 0);
        assert!(value < u32::MAX as u64);
        assert!(value >= 7_000_400);
    }

    #[test]
    fn handles_huge_free_memory_without_wrap() {
        // u64::MAX 转 i64 会回绕成 -1，实现内部先 max(0) 再套下限，结果仍是 300。
        assert_eq!(automated_max_memory(u64::MAX, 0), 300);
        assert_eq!(automated_max_memory(u64::MAX, 1_000_000), 7_000_400);
        // i64::MAX 级别的可用内存会走完四个阶段（f64 乘法 + 截断），结果封顶在 7500。
        assert_eq!(automated_max_memory(i64::MAX as u64, 0), 7_500);
    }
}

#[cfg(test)]
mod should_apply_tests {
    use scl_core::semver::MinecraftVersion;

    use super::*;

    #[test]
    fn independent_for_mod_loader_matches_official_matrix() {
        let release = MinecraftVersion::Release(1, 20, 1);
        let cases = [
            (VersionType::Vanilla, false),
            (VersionType::Forge, true),
            (VersionType::NeoForge, false),
            (VersionType::Fabric, true),
            (VersionType::QuiltMC, false),
            (VersionType::Optifine, false),
            (VersionType::Unknown, false),
        ];
        for (version_type, expected) in cases {
            assert_eq!(
                VersionIndependentMethod::IndependentForModLoader
                    .should_apply(&release, version_type),
                expected,
                "IndependentForModLoader + {version_type:?} 应为 {expected}"
            );
        }
    }

    /// 非正式版判定用的是 Minecraft 版本号（rsi 的 niche 判定），
    /// 不是版本类型：Forge 1.20.1 的 `minecraft_version` 是 Release，不算非正式。
    #[test]
    fn independent_for_non_release_uses_minecraft_version() {
        let release = MinecraftVersion::Release(1, 20, 1);
        let snapshot = MinecraftVersion::Snapshot(23, 31, 'a');
        let custom = MinecraftVersion::Custom("weird".into());

        for version_type in [
            VersionType::Vanilla,
            VersionType::Forge,
            VersionType::NeoForge,
            VersionType::Fabric,
        ] {
            assert!(
                !VersionIndependentMethod::IndependentForNonRelease
                    .should_apply(&release, version_type),
                "Release 版本无论类型都应视为正式版：{version_type:?}"
            );
        }
        assert!(VersionIndependentMethod::IndependentForNonRelease
            .should_apply(&snapshot, VersionType::Vanilla));
        assert!(VersionIndependentMethod::IndependentForNonRelease
            .should_apply(&custom, VersionType::Vanilla));
    }

    #[test]
    fn combined_and_edge_methods_match_official_matrix() {
        let release = MinecraftVersion::Release(1, 20, 1);
        let snapshot = MinecraftVersion::Snapshot(23, 31, 'a');

        for (version_type, expected) in [
            (VersionType::Vanilla, false),
            (VersionType::Forge, true),
            (VersionType::NeoForge, false),
            (VersionType::Fabric, true),
            (VersionType::QuiltMC, false),
        ] {
            assert_eq!(
                VersionIndependentMethod::IndependentForModLoaderAndNonRelease
                    .should_apply(&release, version_type),
                expected
            );
        }
        // 快照算非正式，即使版本类型是原版
        assert!(
            VersionIndependentMethod::IndependentForModLoaderAndNonRelease
                .should_apply(&snapshot, VersionType::Vanilla)
        );

        assert!(
            !VersionIndependentMethod::NoIndependent.should_apply(&snapshot, VersionType::Fabric)
        );
        assert!(
            VersionIndependentMethod::IndependentAll.should_apply(&release, VersionType::Vanilla)
        );
    }
}

#[cfg(test)]
mod config_migration_tests {
    use super::*;

    const OFFICIAL_CONFIG: &str = r#"{
  "selected_version": "",
  "javas": {
    "selected_leagzy": "java",
    "selected_modern": "java",
    "javapaths": [
      {
        "name": "PATH 环境变量里的 Java 8+",
        "path": "java",
        "selected": true,
        "arch": "x86_64",
        "version": 8
      },
      {
        "name": "PATH 环境变量里的 Java 16+",
        "path": "java",
        "selected": true,
        "arch": "x86_64",
        "version": 16
      }
    ]
  },
  "wait_until_launched": true,
  "version_independent_method": "IndependentForModLoaderAndNonRelease",
  "dont_close_window_when_launched": false,
  "hide_window_when_launched": false,
  "auto_check_when_first_start_failed": true,
  "auto_mem_size": true,
  "mem_size": 0,
  "jvm_args": "",
  "game_args": "",
  "wrapper_path": "",
  "wrapper_args": "",
  "theme": {
    "slide_page_animation": false,
    "icons_theme": {}
  },
  "download_source": "Default",
  "download_parallel_amount": 64,
  "client_token": "2275955bdb930d290304a4e08f7bdb5d",
  "minecraft_folder": "CurrentDirectory",
  "minecraft_folders": [],
  "last_authlib_server": ""
}"#;

    /// 本地旧格式：`version` 是字符串，且带 `is_64bit`/`main_version`/`arch`。
    const LEGACY_CONFIG: &str = r#"{
  "selected_version": "26.3",
  "javas": {
    "selected_leagzy": "java",
    "selected_modern": "java",
    "javapaths": [
      {
        "name": "PATH 环境变量里的 Java 8+",
        "path": "java",
        "selected": true,
        "arch": "x86_64",
        "version": "1.8.0_392",
        "is_64bit": true,
        "main_version": 8
      },
      {
        "name": "PATH 环境变量里的 Java 16+",
        "path": "java",
        "selected": true,
        "arch": "x86_64",
        "version": "21.0.2",
        "is_64bit": true,
        "main_version": 21
      }
    ]
  },
  "theme": {
    "dark_theme": false,
    "slide_page_animation": false,
    "icons_theme": {}
  },
  "minecraft_folder": "CurrentDirectory",
  "minecraft_folders": []
}"#;

    const OFFICIAL_TOP_LEVEL_ORDER: &[&str] = &[
        "selected_version",
        "javas",
        "wait_until_launched",
        "version_independent_method",
        "dont_close_window_when_launched",
        "hide_window_when_launched",
        "auto_check_when_first_start_failed",
        "auto_mem_size",
        "mem_size",
        "jvm_args",
        "game_args",
        "wrapper_path",
        "wrapper_args",
        "theme",
        "download_source",
        "download_parallel_amount",
        "client_token",
        "minecraft_folder",
        "minecraft_folders",
        "last_authlib_server",
    ];

    fn pretty(config: &Config) -> String {
        serde_json::to_string_pretty(config).expect("配置应能序列化")
    }

    fn assert_top_level_order(json: &str) {
        let mut last = 0;
        for key in OFFICIAL_TOP_LEVEL_ORDER {
            let needle = format!("\n  \"{key}\":");
            let pos = json
                .find(&needle)
                .unwrap_or_else(|| panic!("序列化结果缺少字段 {key}"));
            assert!(pos > last, "字段 {key} 的写出顺序与官方不一致");
            last = pos;
        }
    }

    fn first_java_object(json: &str) -> &str {
        let list = json.find("\"javapaths\": [").expect("缺少 javapaths");
        let start = json[list..].find('{').expect("缺少 Java 对象") + list;
        let end = json[start..].find('}').expect("Java 对象未闭合") + start;
        &json[start..=end]
    }

    fn assert_java_entry_order(entry: &str) {
        let mut last = 0;
        for key in ["name", "path", "selected", "arch", "version"] {
            let needle = format!("\"{key}\":");
            let pos = entry
                .find(&needle)
                .unwrap_or_else(|| panic!("Java 对象缺少字段 {key}"));
            assert!(pos > last, "Java 字段 {key} 的写出顺序与官方不一致");
            last = pos;
        }
        // 旧格式字段不应再写出。
        assert!(!entry.contains("\"is_64bit\":"));
        assert!(!entry.contains("\"main_version\":"));
    }

    #[test]
    fn reads_official_config_and_writes_same_schema() {
        let config: Config = serde_json::from_str(OFFICIAL_CONFIG).expect("官方配置应能读取");
        assert_eq!(config.javas.javapaths.len(), 2);
        assert_eq!(config.javas.javapaths[0].version, "1.8");
        assert_eq!(config.javas.javapaths[0].main_version, 8);
        assert_eq!(config.javas.javapaths[1].version, "16");
        assert_eq!(config.javas.javapaths[1].main_version, 16);
        assert_eq!(config.java_for_version(8).unwrap().main_version, 8);
        assert_eq!(config.java_for_version(21).unwrap().main_version, 16);

        // 官方样例本身已是官方字段顺序，完整往返应逐字节一致。
        let json = pretty(&config);
        assert_eq!(json, OFFICIAL_CONFIG);
        assert_top_level_order(&json);
        assert_java_entry_order(first_java_object(&json));
    }

    /// `minecraft_folders` 是纯字符串数组：样例直读往返，
    /// 自定义文件夹保持路径字符串形态，不再写成 `{"Other": ...}`。
    #[test]
    fn official_minecraft_folders_round_trips_as_string_array() {
        let sample = r#"{
  "selected_version": "",
  "minecraft_folder": "CurrentDirectory",
  "minecraft_folders": [
    "/home/link/games/minecraft",
    "/mnt/data/.minecraft"
  ]
}"#;
        let config: Config = serde_json::from_str(sample).expect("官方形态应能读取");
        assert_eq!(
            config.minecraft_folders.iter().cloned().collect::<Vec<_>>(),
            vec![
                "/home/link/games/minecraft".to_owned(),
                "/mnt/data/.minecraft".to_owned(),
            ],
            "minecraft_folders 应按纯字符串读取"
        );
        assert_eq!(config.minecraft_folder, MinecraftFolder::CurrentDirectory);

        let json = pretty(&config);
        assert!(
            json.contains("\"minecraft_folders\": [\n    \"/home/link/games/minecraft\""),
            "写出应保持官方纯字符串形态：{json}"
        );
        assert!(
            !json.contains("\"Other\""),
            "不应再写出 {{\"Other\": ...}} 枚举形态：{json}"
        );

        // 读回往返等价
        let reloaded: Config = serde_json::from_str(&json).expect("往返结果应能读取");
        assert_eq!(reloaded.minecraft_folders, config.minecraft_folders);
        assert_eq!(reloaded.minecraft_folder, config.minecraft_folder);
    }

    #[test]
    fn migrates_legacy_java_fields_to_official_schema() {
        let config: Config = serde_json::from_str(LEGACY_CONFIG).expect("旧配置应能读取");
        assert_eq!(config.javas.javapaths[0].version, "1.8.0_392");
        assert_eq!(config.javas.javapaths[0].main_version, 8);
        assert_eq!(config.javas.javapaths[1].version, "21.0.2");
        assert_eq!(config.javas.javapaths[1].main_version, 21);
        assert_eq!(config.java_for_version(8).unwrap().main_version, 8);
        assert_eq!(config.java_for_version(16).unwrap().main_version, 21);

        let json = pretty(&config);
        assert_top_level_order(&json);
        // `version` 写回整数，`is_64bit`/`main_version` 不再出现。
        assert_java_entry_order(first_java_object(&json));
        assert!(json.contains("\"version\": 8"));
        assert!(json.contains("\"version\": 21"));
    }

    #[test]
    fn handles_missing_fields_and_empty_javas() {
        // 只有部分字段：其余字段按默认值补齐。
        let config: Config =
            serde_json::from_str(r#"{"selected_version":"26.3"}"#).expect("缺失字段应能读取");
        assert_eq!(config.javas.javapaths.len(), 2);
        assert_eq!(config.java_for_version(8).unwrap().main_version, 8);
        assert_eq!(config.java_for_version(16).unwrap().main_version, 16);
        assert_top_level_order(&pretty(&config));

        // 显式空列表：不能回退到默认 PATH 项。
        let config: Config =
            serde_json::from_str(r#"{"javas":{"javapaths":[]}}"#).expect("空 javas 应能读取");
        assert!(config.javas.javapaths.is_empty());
        assert!(config.java_for_version(8).is_none());
        assert!(config.java_for_version(16).is_none());
        let json = pretty(&config);
        assert_top_level_order(&json);
        assert!(json.contains("\"javapaths\": []"));
    }

    #[test]
    fn dark_theme_true_is_read_and_written_back() {
        // 官方暗色配置：读入 true → 主题为暗色；保存后字段仍在。
        let config: Config =
            serde_json::from_str(r#"{"theme":{"dark_theme":true}}"#).expect("暗色配置应能读取");
        assert!(config.theme.dark_theme);
        let json = pretty(&config);
        assert!(
            json.contains("\"dark_theme\": true"),
            "dark_theme=true 必须写回：{json}"
        );
        let reloaded: Config = serde_json::from_str(&json).expect("写回后应能再次读取");
        assert!(reloaded.theme.dark_theme, "重载后仍应为暗色");
    }

    #[test]
    fn dark_theme_false_is_omitted_on_save() {
        // 官方亮色配置：false 与默认值一样省略字段。
        let config: Config =
            serde_json::from_str(r#"{"theme":{"dark_theme":false}}"#).expect("亮色配置应能读取");
        assert!(!config.theme.dark_theme);
        assert!(
            !pretty(&config).contains("\"dark_theme\""),
            "dark_theme=false 不应写回"
        );
    }
}

#[cfg(test)]
mod sync_save_tests {
    use super::*;

    fn test_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "scl_sync_save_test_{}_{}_{tag}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs_err::create_dir_all(&dir).expect("创建临时目录");
        dir
    }

    fn temp_residue(dir: &std::path::Path) -> Vec<String> {
        fs_err::read_dir(dir)
            .expect("读取临时目录")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".scl-tmp"))
            .collect()
    }

    fn sample_config(version: &str, mem_size: usize) -> Config {
        Config {
            selected_version: version.into(),
            mem_size,
            ..Config::default()
        }
    }

    #[test]
    fn sync_save_creates_missing_target() {
        let dir = test_dir("first");
        let path = dir.join(".scl.json");
        assert!(!path.exists(), "用例前提：配置文件不存在");

        let config = sample_config("first-save", 2048);
        let data = serde_json::to_vec_pretty(&config).expect("序列化配置");
        save_config_data_sync(&path, &data).expect("首次同步保存配置");

        assert!(path.exists(), "首次保存必须创建配置文件");
        assert!(
            temp_residue(&dir).is_empty(),
            "首次保存后不应有临时文件残留"
        );
        let disk = fs_err::read_to_string(&path).expect("读取配置");
        let loaded: Config = serde_json::from_str(&disk).expect("反序列化配置");
        assert_eq!(loaded, config);
        let _ = fs_err::remove_dir_all(&dir);
    }

    #[test]
    fn sync_save_roundtrip_replaces_file_atomically() {
        let dir = test_dir("roundtrip");
        let path = dir.join(".scl.json");
        // 预置旧内容，确认 rename 是完整替换而不是追加。
        fs_err::write(&path, "old").expect("写入旧配置");

        let config = sample_config("round-trip", 4321);
        let data = serde_json::to_vec_pretty(&config).expect("序列化配置");
        save_config_data_sync(&path, &data).expect("同步保存配置");

        // 临时文件必须已被 rename 消费（临时名唯一，改为扫描目录）。
        assert!(
            temp_residue(&dir).is_empty(),
            "覆盖保存后不应有临时文件残留"
        );
        let disk = fs_err::read_to_string(&path).expect("读取配置");
        let loaded: Config = serde_json::from_str(&disk).expect("反序列化配置");
        assert_eq!(loaded, config);
        let _ = fs_err::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod auth_data_tests {
    use std::fs;

    use super::*;
    use crate::core::AuthMethod;

    fn test_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "scl_auth_data_test_{}_{}_{tag}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("创建临时目录");
        dir
    }

    fn offline_account(name: &str) -> AccountData {
        AccountData {
            selected: 0,
            accounts: im::vector![AuthMethod::Offline {
                player_name: name.to_owned(),
                uuid: "00000000-0000-0000-0000-000000000000".to_owned(),
            }],
        }
    }

    #[test]
    fn auth_data_json_matches_official_compact_format() {
        assert_eq!(
            serde_json::to_string(&AccountData::default()).expect("序列化空账户"),
            r#"{"selected":0,"accounts":[]}"#
        );

        let accounts = offline_account("TestPlayer");
        let json = serde_json::to_string(&accounts).expect("序列化账户");
        assert_eq!(
            json,
            r#"{"selected":0,"accounts":[{"Offline":{"player_name":"TestPlayer","uuid":"00000000-0000-0000-0000-000000000000"}}]}"#,
            "官方 auth_data.json 为紧凑 JSON 且 selected 在 accounts 之前"
        );
        assert!(!json.contains('\n'), "官方 auth_data.json 不含换行");
    }

    /// 官方启动优先级：先读 .scl.json，再用 auth_data.json 覆盖账户数据。
    #[test]
    fn auth_load_overrides_scl_accounts() {
        let dir = test_dir("override");
        let auth_path = dir.join("auth_data.json");
        fs::write(
            &auth_path,
            r#"{"selected":0,"accounts":[{"Offline":{"player_name":"FromAuthFile","uuid":"u"}}]}"#,
        )
        .expect("写入 auth 文件");

        let auth = load_auth_data_from(&auth_path).expect("应能读取 auth 文件");
        let mut config = Config {
            accounts_data: offline_account("FromSclJson"),
            ..Config::default()
        };
        apply_auth_override(&mut config, Some(auth));

        assert_eq!(
            config
                .accounts_data
                .accounts
                .front()
                .map(AuthMethod::display_name),
            Some("FromAuthFile".to_owned()),
            "auth_data.json 存在时应覆盖 .scl.json 的账户"
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// auth 文件存在但账户为空：官方语义是覆盖（auth 文件是账户的权威来源）。
    #[test]
    fn auth_file_present_even_empty_wins() {
        let dir = test_dir("empty-wins");
        let auth_path = dir.join("auth_data.json");
        fs::write(&auth_path, r#"{"selected":0,"accounts":[]}"#).expect("写入空 auth 文件");

        let auth = load_auth_data_from(&auth_path).expect("空账户文件应能解析");
        let mut config = Config {
            accounts_data: offline_account("OldAccount"),
            ..Config::default()
        };
        apply_auth_override(&mut config, Some(auth));

        assert!(
            config.accounts_data.accounts.is_empty(),
            "官方语义：auth 文件一旦存在即覆盖，即使账户列表为空"
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// auth 文件缺失或损坏时保留 .scl.json 的账户数据（旧数据兼容）。
    #[test]
    fn auth_missing_or_broken_keeps_scl_accounts() {
        let dir = test_dir("missing");
        let mut config = Config {
            accounts_data: offline_account("Kept"),
            ..Config::default()
        };

        apply_auth_override(&mut config, None);
        assert_eq!(
            config
                .accounts_data
                .accounts
                .front()
                .map(AuthMethod::display_name),
            Some("Kept".to_owned()),
            "auth 文件缺失时保留 .scl.json 账户"
        );

        let broken = dir.join("auth_data.json");
        fs::write(&broken, "{not-json").expect("写入损坏 auth 文件");
        assert!(
            load_auth_data_from(&broken).is_none(),
            "损坏的 auth 文件应返回 None"
        );
        apply_auth_override(&mut config, load_auth_data_from(&broken));
        assert_eq!(
            config
                .accounts_data
                .accounts
                .front()
                .map(AuthMethod::display_name),
            Some("Kept".to_owned()),
            "auth 文件损坏时保留 .scl.json 账户"
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// 同步保存双轨落盘：先 ~/.scl/auth_data.json（紧凑），再 .scl.json（pretty）。
    /// `.scl.json` 恒 20 键，账户只进 auth 文件。
    #[test]
    fn sync_save_writes_both_files_in_official_format() {
        let dir = test_dir("dual-write");
        let auth_dir = dir.join("home").join(".scl");
        let scl_path = dir.join(".scl.json");

        let config = Config {
            accounts_data: offline_account("DualWrite"),
            ..Config::default()
        };
        let data = serde_json::to_vec_pretty(&config).expect("序列化配置");

        // 官方顺序：auth 先于 .scl.json
        save_auth_data_in_dir_sync(&auth_dir, &config.accounts_data).expect("写入账户文件");
        save_config_data_sync(&scl_path, &data).expect("写入配置文件");

        let auth_disk =
            fs::read_to_string(auth_dir.join("auth_data.json")).expect("auth 文件应被创建");
        assert_eq!(
            auth_disk,
            r#"{"selected":0,"accounts":[{"Offline":{"player_name":"DualWrite","uuid":"00000000-0000-0000-0000-000000000000"}}]}"#,
            "auth_data.json 应为官方紧凑格式"
        );

        let scl_disk = fs::read_to_string(&scl_path).expect("读取配置文件");
        let loaded: Config = serde_json::from_str(&scl_disk).expect("反序列化配置");
        // 账户不进 .scl.json，读回后 accounts_data 为默认值
        let expected = Config {
            accounts_data: AccountData::default(),
            ..config.clone()
        };
        assert_eq!(loaded, expected);
        assert!(
            !scl_disk.contains("\"accounts_data\""),
            "官方 .scl.json 恒 20 键，账户数据不应写入"
        );
        let key_count = serde_json::from_str::<serde_json::Value>(&scl_disk)
            .expect("解析配置")
            .as_object()
            .expect("配置应为对象")
            .len();
        assert_eq!(
            key_count, 20,
            "官方 .scl.json 恒 20 个键（serialize_map(0x14)）"
        );
        assert!(
            scl_disk.starts_with("{\n  \"selected_version\""),
            "官方 .scl.json 第一个键是 selected_version"
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// 旧版只写 .scl.json（accounts 在 selected 之前的旧字段顺序）的数据迁移：
    /// 读取正常，下一次保存按官方行为回填 auth_data.json。
    #[test]
    fn old_scl_json_accounts_migrate_to_auth_file() {
        let dir = test_dir("migrate");
        // 旧版本恢复程序写出的字段顺序：accounts 在前
        let old_json = r#"{
  "accounts_data": {
    "accounts": [
      {
        "Offline": {
          "player_name": "Migrated",
          "uuid": "00000000-0000-0000-0000-000000000000"
        }
      }
    ],
    "selected": 0
  },
  "selected_version": "old"
}"#;
        let config: Config = serde_json::from_str(old_json).expect("旧字段顺序应能正常解析");
        assert_eq!(
            config
                .accounts_data
                .accounts
                .front()
                .map(AuthMethod::display_name),
            Some("Migrated".to_owned())
        );

        // 下一次保存：auth_data.json 按官方紧凑格式创建
        let auth_dir = dir.join("home").join(".scl");
        save_auth_data_in_dir_sync(&auth_dir, &config.accounts_data).expect("回填 auth 文件");
        let auth_disk =
            fs::read_to_string(auth_dir.join("auth_data.json")).expect("读取 auth 文件");
        assert!(
            auth_disk.contains(r#""player_name":"Migrated""#),
            "迁移后 auth_data.json 应包含旧数据里的账户：{auth_disk}"
        );
        assert!(
            auth_disk.starts_with(r#"{"selected":"#),
            "回填文件应使用官方字段顺序"
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// 双轨保存同样写入 auth_data.json 与 .scl.json。
    #[test]
    fn save_writes_both_files() {
        let dir = test_dir("dual-save");
        let auth_dir = dir.join("home").join(".scl");
        let scl_path = dir.join(".scl.json");

        let config = Config {
            accounts_data: offline_account("BackgroundSave"),
            ..Config::default()
        };
        let data = serde_json::to_vec_pretty(&config).expect("序列化配置");

        save_auth_data_in_dir_sync(&auth_dir, &config.accounts_data).expect("写入账户文件");
        crate::utils::prelude::write_file_safe_sync(&scl_path, data).expect("写入配置文件");

        let auth_disk =
            fs::read_to_string(auth_dir.join("auth_data.json")).expect("auth 文件应被创建");
        assert!(auth_disk.contains(r#""player_name":"BackgroundSave""#));
        assert!(!auth_disk.contains('\n'), "写入也应为紧凑格式");
        assert!(scl_path.exists());
        let _ = fs::remove_dir_all(dir);
    }
}
