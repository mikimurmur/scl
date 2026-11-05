//! 启动器内部使用的核心数据结构。
//!
//! 这里的数据结构大多和 [`scl_core`] 中的结构一一对应，但是为了能够被
//! Druid 的数据系统追踪变化，额外实现了 [`druid::Data`] 和 [`druid::Lens`]。

use druid::{Data, Lens};
use scl_core::{
    auth::structs::AuthMethod as CoreAuthMethod,
    download::DownloadSource as CoreDownloadSource,
    password::Password,
    version::{structs::SCLLaunchConfig as CoreSCLLaunchConfig, VersionType as CoreVersionType},
};
use serde::{Deserialize, Serialize};

/// 一个可以在启动器内部使用，并且能够被转换回 [`scl_core`] 数据结构的类型
pub trait CoreTransformer<C>: Sized {
    /// 转换为 [`scl_core`] 中的数据结构
    fn into_core(self) -> C;
    /// 从 [`scl_core`] 中的数据结构转换
    fn from_core(core: C) -> Self;
}

/// 版本类型，和 [`scl_core::version::VersionType`] 对应
#[derive(Debug, Clone, Copy, PartialEq, Eq, Data, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum VersionType {
    /// 纯净版本
    Vanilla,
    /// Forge 版本
    Forge,
    /// NeoForge 版本
    NeoForge,
    /// Fabric 版本
    Fabric,
    /// QuiltMC 版本
    QuiltMC,
    /// Optifine 版本
    Optifine,
    /// 未知版本
    #[default]
    Unknown,
}

impl From<CoreVersionType> for VersionType {
    fn from(value: CoreVersionType) -> Self {
        match value {
            CoreVersionType::Vanilla => Self::Vanilla,
            CoreVersionType::Forge => Self::Forge,
            CoreVersionType::NeoForge => Self::NeoForge,
            CoreVersionType::Fabric => Self::Fabric,
            CoreVersionType::QuiltMC => Self::QuiltMC,
            CoreVersionType::Optifine => Self::Optifine,
            CoreVersionType::Unknown => Self::Unknown,
        }
    }
}

impl From<VersionType> for CoreVersionType {
    fn from(value: VersionType) -> Self {
        match value {
            VersionType::Vanilla => Self::Vanilla,
            VersionType::Forge => Self::Forge,
            VersionType::NeoForge => Self::NeoForge,
            VersionType::Fabric => Self::Fabric,
            VersionType::QuiltMC => Self::QuiltMC,
            VersionType::Optifine => Self::Optifine,
            VersionType::Unknown => Self::Unknown,
        }
    }
}

impl VersionType {
    /// 获取用于展示的版本类型名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Vanilla => "原版",
            Self::Forge => "Forge",
            Self::NeoForge => "NeoForge",
            Self::Fabric => "Fabric",
            Self::QuiltMC => "QuiltMC",
            Self::Optifine => "Optifine",
            Self::Unknown => "未知",
        }
    }
}

/// 下载源，和 [`scl_core::download::DownloadSource`] 对应
#[derive(Debug, Clone, PartialEq, Eq, Data, Serialize, Deserialize, Default)]
pub enum DownloadSource {
    /// 官方下载源
    ///
    /// 兼容旧配置：已失效的 MCBBS 下载源在反序列化时回退到默认源。
    #[default]
    #[serde(alias = "Mcbbs")]
    Default,
    /// BMCLAPI 下载源
    BmclApi,
}

impl DownloadSource {
    /// 获取用于展示的下载源名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Default => "官方下载源",
            Self::BmclApi => "BMCLAPI 下载源",
        }
    }

    /// 转换成 [`scl_core::download::DownloadSource`]
    pub fn to_core(&self) -> CoreDownloadSource {
        match self {
            Self::Default => CoreDownloadSource::Default,
            Self::BmclApi => CoreDownloadSource::BMCLAPI,
        }
    }
}

impl CoreTransformer<CoreDownloadSource> for DownloadSource {
    fn into_core(self) -> CoreDownloadSource {
        self.to_core()
    }

    fn from_core(core: CoreDownloadSource) -> Self {
        match core {
            CoreDownloadSource::Default | CoreDownloadSource::Custom(_) => Self::Default,
            CoreDownloadSource::BMCLAPI => Self::BmclApi,
        }
    }
}

/// 启动器账户数据
///
/// 字段和 [`scl_core::auth::structs::AuthMethod`] 完全一致，但是令牌使用字符串存储，
/// 以便能够被 Druid 的数据系统直接使用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    /// 离线账户
    Offline {
        /// 离线玩家的名称
        player_name: String,
        /// 离线玩家的 UUID
        uuid: String,
    },
    /// Mojang (Yggdrasil) 账户
    Mojang {
        /// 登录令牌
        access_token: String,
        /// 玩家的 UUID
        uuid: String,
        /// 玩家的名称
        player_name: String,
        /// 头部皮肤数据
        #[serde(default)]
        head_skin: Vec<u8>,
        /// 帽子皮肤数据
        #[serde(default)]
        hat_skin: Vec<u8>,
    },
    /// 微软账户
    Microsoft {
        /// 登录令牌
        access_token: String,
        /// 刷新令牌
        refresh_token: String,
        /// 玩家的 UUID
        uuid: String,
        /// XUID
        #[serde(default)]
        xuid: String,
        /// 玩家的名称
        player_name: String,
        /// 头部皮肤数据
        #[serde(default)]
        head_skin: Vec<u8>,
        /// 帽子皮肤数据
        #[serde(default)]
        hat_skin: Vec<u8>,
    },
    /// Authlib-Injector 外置登录账户
    AuthlibInjector {
        /// API 地址
        api_location: String,
        /// 服务器名称
        #[serde(default)]
        server_name: String,
        /// 服务器主页
        #[serde(default)]
        server_homepage: String,
        /// 服务器元数据
        #[serde(default)]
        server_meta: String,
        /// 登录令牌
        access_token: String,
        /// 玩家的 UUID
        uuid: String,
        /// 玩家的名称
        player_name: String,
        /// 头部皮肤数据
        #[serde(default)]
        head_skin: Vec<u8>,
        /// 帽子皮肤数据
        #[serde(default)]
        hat_skin: Vec<u8>,
    },
}

impl Data for AuthMethod {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl AuthMethod {
    /// 获取账户名称
    pub fn player_name(&self) -> &str {
        match self {
            Self::Offline { player_name, .. }
            | Self::Mojang { player_name, .. }
            | Self::Microsoft { player_name, .. }
            | Self::AuthlibInjector { player_name, .. } => player_name,
        }
    }

    /// 获取账户 UUID
    pub fn uuid(&self) -> &str {
        match self {
            Self::Offline { uuid, .. }
            | Self::Mojang { uuid, .. }
            | Self::Microsoft { uuid, .. }
            | Self::AuthlibInjector { uuid, .. } => uuid,
        }
    }

    /// 获取账户类型名称
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Offline { .. } => "离线账户",
            Self::Mojang { .. } => "Mojang 正版账户",
            Self::Microsoft { .. } => "Microsoft 正版账户",
            Self::AuthlibInjector { .. } => "Authlib Injector 第三方账户",
        }
    }

    /// 获取头部皮肤数据
    pub fn head_skin(&self) -> &[u8] {
        match self {
            Self::Offline { .. } => &[],
            Self::Mojang { head_skin, .. }
            | Self::Microsoft { head_skin, .. }
            | Self::AuthlibInjector { head_skin, .. } => head_skin,
        }
    }

    /// 设置头部与帽子皮肤数据
    pub fn set_head_skin(&mut self, head: Vec<u8>, hat: Vec<u8>) {
        match self {
            Self::Offline { .. } => {}
            Self::Mojang {
                head_skin,
                hat_skin,
                ..
            }
            | Self::Microsoft {
                head_skin,
                hat_skin,
                ..
            }
            | Self::AuthlibInjector {
                head_skin,
                hat_skin,
                ..
            } => {
                *head_skin = head;
                *hat_skin = hat;
            }
        }
    }

    /// 获取帽子皮肤数据
    pub fn hat_skin(&self) -> &[u8] {
        match self {
            Self::Offline { .. } => &[],
            Self::Mojang { hat_skin, .. }
            | Self::Microsoft { hat_skin, .. }
            | Self::AuthlibInjector { hat_skin, .. } => hat_skin,
        }
    }

    /// 判断是否为同一个账户
    pub fn is_same_account(&self, other: &Self) -> bool {
        self.uuid() == other.uuid()
            && self.player_name() == other.player_name()
            && std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    /// 获取用于展示的简短名称
    pub fn display_name(&self) -> String {
        if self.player_name().is_empty() {
            self.type_name().to_owned()
        } else {
            self.player_name().to_owned()
        }
    }
}

impl CoreTransformer<CoreAuthMethod> for AuthMethod {
    fn into_core(self) -> CoreAuthMethod {
        match self {
            Self::Offline { player_name, uuid } => CoreAuthMethod::Offline { player_name, uuid },
            Self::Mojang {
                access_token,
                uuid,
                player_name,
                head_skin,
                hat_skin,
            } => CoreAuthMethod::Mojang {
                access_token: Password::from(access_token),
                uuid,
                player_name,
                head_skin,
                hat_skin,
            },
            Self::Microsoft {
                access_token,
                refresh_token,
                uuid,
                xuid,
                player_name,
                head_skin,
                hat_skin,
            } => CoreAuthMethod::Microsoft {
                access_token: Password::from(access_token),
                refresh_token: Password::from(refresh_token),
                uuid,
                xuid,
                player_name,
                head_skin,
                hat_skin,
            },
            Self::AuthlibInjector {
                api_location,
                server_name,
                server_homepage,
                server_meta,
                access_token,
                uuid,
                player_name,
                head_skin,
                hat_skin,
            } => CoreAuthMethod::AuthlibInjector {
                api_location,
                server_name,
                server_homepage,
                server_meta,
                access_token: Password::from(access_token),
                uuid,
                player_name,
                head_skin,
                hat_skin,
            },
        }
    }

    fn from_core(core: CoreAuthMethod) -> Self {
        match core {
            CoreAuthMethod::Offline { player_name, uuid } => Self::Offline { player_name, uuid },
            CoreAuthMethod::Mojang {
                access_token,
                uuid,
                player_name,
                head_skin,
                hat_skin,
            } => Self::Mojang {
                access_token: access_token.to_owned_string(),
                uuid,
                player_name,
                head_skin,
                hat_skin,
            },
            CoreAuthMethod::Microsoft {
                access_token,
                refresh_token,
                uuid,
                xuid,
                player_name,
                head_skin,
                hat_skin,
            } => Self::Microsoft {
                access_token: access_token.to_owned_string(),
                refresh_token: refresh_token.to_owned_string(),
                uuid,
                xuid,
                player_name,
                head_skin,
                hat_skin,
            },
            CoreAuthMethod::AuthlibInjector {
                api_location,
                server_name,
                server_homepage,
                server_meta,
                access_token,
                uuid,
                player_name,
                head_skin,
                hat_skin,
            } => Self::AuthlibInjector {
                api_location,
                server_name,
                server_homepage,
                server_meta,
                access_token: access_token.to_owned_string(),
                uuid,
                player_name,
                head_skin,
                hat_skin,
            },
        }
    }
}

/// 远端版本信息，对应下载清单中的一项
#[derive(Debug, Clone, Default, PartialEq, Eq, Data, Lens, Serialize, Deserialize)]
pub struct VersionInfo {
    /// 版本名称
    pub name: String,
    /// 版本类型，例如 release / snapshot
    pub version_type: String,
    /// 版本元数据下载链接
    pub url: String,
    /// 更新时间
    pub time: String,
    /// 发布时间
    pub release_time: String,
}

impl CoreTransformer<scl_core::download::structs::VersionInfo> for VersionInfo {
    fn into_core(self) -> scl_core::download::structs::VersionInfo {
        scl_core::download::structs::VersionInfo {
            id: self.name,
            version_type: self.version_type,
            url: self.url,
            time: self.time,
            release_time: self.release_time,
        }
    }

    fn from_core(core: scl_core::download::structs::VersionInfo) -> Self {
        Self {
            name: core.id,
            version_type: core.version_type,
            url: core.url,
            time: core.time,
            release_time: core.release_time,
        }
    }
}

/// 启动器独立版本设置
///
/// 这里在 [`scl_core::version::structs::SCLLaunchConfig`] 的基础上增加了少量
/// 仅在界面上使用的字段。序列化到版本文件夹下的 `.scl.json` 时，这些字段会被忽略。
#[derive(Debug, Clone, Default, PartialEq, Eq, Data, Lens, Serialize, Deserialize)]
#[serde(default)]
pub struct SCLLaunchConfig {
    /// 是否启用独立版本设置
    #[serde(skip)]
    pub enable: bool,
    /// 最大内存
    pub max_mem: Option<usize>,
    /// 界面使用的内存文本
    #[serde(skip)]
    pub max_mem_text: String,
    /// 是否覆盖自动内存设置
    #[serde(skip)]
    pub override_max_mem: bool,
    /// Java 运行时路径
    pub java_path: String,
    /// 是否使用版本独立
    pub game_independent: bool,
    /// 窗口标题
    pub window_title: String,
    /// 额外 JVM 参数
    pub jvm_args: String,
    /// 额外游戏参数
    pub game_args: String,
    /// 包装器路径
    pub wrapper_path: String,
    /// 包装器参数
    pub wrapper_args: String,
}

impl CoreTransformer<CoreSCLLaunchConfig> for SCLLaunchConfig {
    fn into_core(self) -> CoreSCLLaunchConfig {
        CoreSCLLaunchConfig {
            max_mem: self.max_mem,
            java_path: self.java_path,
            game_independent: self.game_independent,
            window_title: self.window_title,
            jvm_args: self.jvm_args,
            game_args: self.game_args,
            wrapper_path: self.wrapper_path,
            wrapper_args: self.wrapper_args,
        }
    }

    fn from_core(core: CoreSCLLaunchConfig) -> Self {
        Self {
            enable: true,
            max_mem: core.max_mem,
            max_mem_text: core.max_mem.map(|x| x.to_string()).unwrap_or_default(),
            override_max_mem: core.max_mem.is_some(),
            java_path: core.java_path,
            game_independent: core.game_independent,
            window_title: core.window_title,
            jvm_args: core.jvm_args,
            game_args: core.game_args,
            wrapper_path: core.wrapper_path,
            wrapper_args: core.wrapper_args,
        }
    }
}

/// 一个已经安装的游戏版本
#[derive(Debug, Clone, Default, PartialEq, Eq, Data, Lens)]
pub struct LocalVersionInfo {
    /// 版本名称
    pub name: String,
    /// 版本类型
    pub version_type: VersionType,
    /// 版本目录
    pub path: String,
    /// 版本创建时间，Unix 毫秒时间戳
    pub created_date: i64,
    /// 版本最后访问时间，Unix 毫秒时间戳
    pub access_date: i64,
}
