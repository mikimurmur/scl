//! 下载游戏页面。
//!
//! 页面结构：
//! * 根页面只有「游戏版本名称」输入框、六个 40px 高的模块行和底部通栏「安装游戏」按钮；
//! * 点击模块行会进入各加载器的独立选择页（`download_vanilla` / `download_fabric` 等）；
//! * 选择页由「最新版本」行与可折叠的版本分组组成，选中项右侧显示实心圆点。

pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod optifine;
pub mod quiltmc;
pub mod vanilla;

use std::marker::PhantomData;

use druid::{
    kurbo::{BezPath, Circle, Shape},
    piet::{PaintBrush, Text, TextLayout, TextLayoutBuilder},
    widget::{Click, ControllerHost, CrossAxisAlignment, Flex, Scroll},
    Affine, BoxConstraints, Data, Env, Event, EventCtx, Insets, LayoutCtx, Lens, LifeCycle,
    LifeCycleCtx, PaintCtx, Point, RenderContext, Selector, Size, Target, UpdateCtx, Widget,
    WidgetExt,
};
use im::Vector;
use scl_gui_widgets::{
    theme::{
        color::{base, list, main, typography},
        icons::IconKeyPair,
    },
    widgets::{label, FolderList},
};

use crate::{
    app::{self, DOWNLOAD_GAME, VANILLA_DOWNLOAD_SELECT},
    app_state::{AppState, DownloadState, NetworkData},
    core::VersionInfo,
    theme::icons::{
        FOLDER, GAME_FABRIC, GAME_FORGE, GAME_NEOFORGE, GAME_OPTIFINE, GAME_QUILTMC, GAME_VANILLA,
    },
    ui::{
        common, PAGE_DOWNLOAD_FABRIC, PAGE_DOWNLOAD_FORGE, PAGE_DOWNLOAD_NEOFORGE,
        PAGE_DOWNLOAD_OPTIFINE, PAGE_DOWNLOAD_QUILTMC, PAGE_DOWNLOAD_VANILLA, PAGE_GAME_DIR,
        PUSH_PAGE, QUERY_POP_PAGE,
    },
};

/// 选择行中根据数据生成文字的闭包
type SelectRowText<D> = Box<dyn Fn(&D, &Env) -> String>;
/// 选择行中判断是否选中的闭包
type SelectRowSelected<D> = Box<dyn Fn(&D, &Env) -> bool>;

/// 一个带选中圆点的选择行，行高 40px，图标占左侧 40px。
pub struct SelectRow<D> {
    icon_key: IconKeyPair,
    icon_path: BezPath,
    text: SelectRowText<D>,
    selected: SelectRowSelected<D>,
}

impl<D: Data> SelectRow<D> {
    /// 创建一个选择行
    pub fn new(
        icon: IconKeyPair,
        text: impl Fn(&D, &Env) -> String + 'static,
        selected: impl Fn(&D, &Env) -> bool + 'static,
    ) -> Self {
        Self {
            icon_key: icon,
            icon_path: BezPath::new(),
            text: Box::new(text),
            selected: Box::new(selected),
        }
    }

    fn reload_icon(&mut self, env: &Env) {
        let svg = env.get(&self.icon_key.0);
        self.icon_path = BezPath::from_svg(svg.as_ref()).unwrap_or_default();
    }

    /// 点击回调
    pub fn on_click(
        self,
        f: impl Fn(&mut EventCtx, &mut D, &Env) + 'static,
    ) -> ControllerHost<Self, Click<D>> {
        ControllerHost::new(self, Click::new(f))
    }
}

impl<D: Data> Widget<D> for SelectRow<D> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut D, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                ctx.set_active(false);
                ctx.request_paint();
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &D, env: &Env) {
        match event {
            LifeCycle::WidgetAdded => self.reload_icon(env),
            LifeCycle::HotChanged(_) => ctx.request_paint(),
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old: &D, _data: &D, env: &Env) {
        if ctx.env_key_changed(&self.icon_key.0) {
            self.reload_icon(env);
        }
        ctx.request_paint();
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &D, _env: &Env) -> Size {
        bc.constrain(Size::new(bc.max().width, 40.))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &D, env: &Env) {
        let size = ctx.size();
        if ctx.is_hot() {
            let active = ctx.is_active();
            let bg = if active {
                env.get(base::LOW)
            } else {
                env.get(list::LIST_LOW)
            };
            ctx.fill(size.to_rect(), &PaintBrush::Color(bg));
        }

        // 图标在 40×40 的格子内居中
        let icon_brush = PaintBrush::Color(env.get(if env.get(main::IS_DARK) {
            &self.icon_key.2
        } else {
            &self.icon_key.1
        }));
        ctx.with_save(|ctx| {
            ctx.transform(Affine::translate(
                ((Size::new(40., 40.) - self.icon_path.bounding_box().size()) / 2.).to_vec2(),
            ));
            ctx.fill_even_odd(self.icon_path.to_owned(), &icon_brush);
        });

        let font = env.get(typography::BODY);
        if let Ok(layout) = ctx
            .text()
            .new_text_layout((self.text)(data, env))
            .font(font.family.clone(), font.size)
            .text_color(env.get(base::MEDIUM))
            .build()
        {
            let y = (size.height - layout.size().height) / 2.;
            ctx.with_save(|ctx| {
                ctx.clip(druid::Rect::new(40., 0., size.width - 30., size.height));
                ctx.draw_text(&layout, (40., y));
            });
        }

        if (self.selected)(data, env) {
            ctx.fill(
                Circle::new(Point::new(size.width - 18., size.height / 2.), 4.),
                &PaintBrush::Color(env.get(base::HIGH)),
            );
        }
    }
}

/// 创建一个选择行并绑定点击回调
pub(crate) fn select_row<D: Data>(
    icon: IconKeyPair,
    text: impl Fn(&D, &Env) -> String + 'static,
    selected: impl Fn(&D, &Env) -> bool + 'static,
    on_click: impl Fn(&mut EventCtx, &mut D, &Env) + 'static,
) -> impl Widget<D> {
    SelectRow::new(icon, text, selected).on_click(on_click)
}

/// 选择页列表中的一项，附带是否被选中
#[derive(Clone, Debug, Data)]
pub struct SelectableVersion<T> {
    /// 是否被选中
    pub selected: bool,
    /// 版本数据
    pub version: T,
}

/// 加载器选择页数据：版本列表 + 该加载器自己的选中版本槽位
#[derive(Clone, Debug, Default, PartialEq, Data, Lens)]
pub struct LoaderPageData<T: Clone> {
    /// 版本列表
    pub state: DownloadState<T>,
    /// 当前加载器选中的版本；空字符串表示「不安装」
    pub selected: String,
}

fn string_loader_state<'a>(
    data: &'a AppState,
    kind: &str,
) -> &'a NetworkData<DownloadState<String>> {
    match kind {
        "forge" => &data.download_state_forge,
        "neoforge" => &data.download_state_neoforge,
        "fabric" => &data.download_state_fabric,
        "quiltmc" => &data.download_state_quiltmc,
        "optifine" => &data.download_state_optifine,
        _ => &data.download_state_fabric,
    }
}

fn string_loader_state_mut<'a>(
    data: &'a mut AppState,
    kind: &str,
) -> &'a mut NetworkData<DownloadState<String>> {
    match kind {
        "forge" => &mut data.download_state_forge,
        "neoforge" => &mut data.download_state_neoforge,
        "fabric" => &mut data.download_state_fabric,
        "quiltmc" => &mut data.download_state_quiltmc,
        "optifine" => &mut data.download_state_optifine,
        _ => &mut data.download_state_fabric,
    }
}

/// 五个字符串加载器页面的 AppState 镜头
pub struct StringLoaderPageLens {
    kind: &'static str,
}

impl StringLoaderPageLens {
    /// 创建指定加载器的页面镜头
    pub fn new(kind: &'static str) -> Self {
        Self { kind }
    }
}

impl Lens<AppState, NetworkData<LoaderPageData<String>>> for StringLoaderPageLens {
    fn with<V, F: FnOnce(&NetworkData<LoaderPageData<String>>) -> V>(
        &self,
        data: &AppState,
        f: F,
    ) -> V {
        let selected = data.loader_version(self.kind).to_owned();
        let network = string_loader_state(data, self.kind);
        let page = match &network.data {
            Some(state) => NetworkData::new(LoaderPageData {
                state: state.clone(),
                selected,
            }),
            None => NetworkData {
                loading: network.loading,
                data: None,
                error: network.error.clone(),
            },
        };
        f(&page)
    }

    fn with_mut<V, F: FnOnce(&mut NetworkData<LoaderPageData<String>>) -> V>(
        &self,
        data: &mut AppState,
        f: F,
    ) -> V {
        let mut page = self.with(data, |page| page.clone());
        let result = f(&mut page);
        if let Some(page_data) = page.data {
            {
                *string_loader_state_mut(data, self.kind) = NetworkData::new(page_data.state);
            }
            data.set_loader_version(self.kind, page_data.selected);
        } else {
            *string_loader_state_mut(data, self.kind) = NetworkData {
                loading: page.loading,
                data: None,
                error: page.error,
            };
        }
        result
    }
}

/// 原版版本页的 AppState 镜头
pub struct VanillaLoaderPageLens;

impl Lens<AppState, NetworkData<LoaderPageData<VersionInfo>>> for VanillaLoaderPageLens {
    fn with<V, F: FnOnce(&NetworkData<LoaderPageData<VersionInfo>>) -> V>(
        &self,
        data: &AppState,
        f: F,
    ) -> V {
        let selected = data
            .selected_download_version
            .as_ref()
            .map(|version| version.name.clone())
            .unwrap_or_default();
        let network = &data.download_state_vanilla;
        let page = match &network.data {
            Some(state) => NetworkData::new(LoaderPageData {
                state: state.clone(),
                selected,
            }),
            None => NetworkData {
                loading: network.loading,
                data: None,
                error: network.error.clone(),
            },
        };
        f(&page)
    }

    fn with_mut<V, F: FnOnce(&mut NetworkData<LoaderPageData<VersionInfo>>) -> V>(
        &self,
        data: &mut AppState,
        f: F,
    ) -> V {
        let mut page = self.with(data, |page| page.clone());
        let result = f(&mut page);
        if let Some(page_data) = page.data {
            let selected_version = page_data
                .state
                .all_versions()
                .find(|version| version.name == page_data.selected)
                .cloned();
            data.download_state_vanilla = NetworkData::new(page_data.state);
            data.selected_download_version = selected_version;
        } else {
            data.download_state_vanilla = NetworkData {
                loading: page.loading,
                data: None,
                error: page.error,
            };
        }
        result
    }
}

/// 把加载器版本列表映射为带选中标记的列表
pub struct SelectedVersionsLens<L, T> {
    list: L,
    key: fn(&T) -> String,
    marker: PhantomData<T>,
}

/// 创建带选中标记的版本列表镜头
pub(crate) fn selected_versions<L, T>(
    list: L,
    key: fn(&T) -> String,
) -> SelectedVersionsLens<L, T> {
    SelectedVersionsLens {
        list,
        key,
        marker: PhantomData,
    }
}

impl<L, T> Lens<LoaderPageData<T>, Vector<SelectableVersion<T>>> for SelectedVersionsLens<L, T>
where
    T: Clone + 'static,
    L: Lens<LoaderPageData<T>, Vector<T>>,
{
    fn with<V, F: FnOnce(&Vector<SelectableVersion<T>>) -> V>(
        &self,
        data: &LoaderPageData<T>,
        f: F,
    ) -> V {
        let selected = data.selected.clone();
        let versions = self.list.with(data, |versions| versions.clone());
        let versions = versions
            .into_iter()
            .map(|version| SelectableVersion {
                selected: (self.key)(&version) == selected,
                version,
            })
            .collect();
        f(&versions)
    }

    fn with_mut<V, F: FnOnce(&mut Vector<SelectableVersion<T>>) -> V>(
        &self,
        data: &mut LoaderPageData<T>,
        f: F,
    ) -> V {
        let selected = data.selected.clone();
        let versions = self.list.with(data, |versions| versions.clone());
        let mut versions = versions
            .into_iter()
            .map(|version| SelectableVersion {
                selected: (self.key)(&version) == selected,
                version,
            })
            .collect();
        f(&mut versions)
    }
}

/// 加载器选择页里通用的「不安装」行
pub(crate) fn no_install_row(
    icon: IconKeyPair,
    selector: Selector<String>,
) -> impl Widget<LoaderPageData<String>> {
    select_row(
        icon,
        |_, _| "不安装".to_owned(),
        |data: &LoaderPageData<String>, _| data.selected.is_empty(),
        move |ctx, _data, _| {
            ctx.submit_command(selector.with(String::new()).to(Target::Global));
            ctx.submit_command(QUERY_POP_PAGE.with("").to(Target::Global));
        },
    )
}

/// 判断“最新版本”行是否应显示选中圆点。
///
/// 版本列表尚未加载时 `latest` 为 `None`，该行显示“无”；
/// 此时不能把空选中值与“不安装”的空值当成同一项，否则两行会同时命中。
fn latest_row_selected(latest: Option<&String>, selected: &str) -> bool {
    latest.is_some_and(|latest| latest.as_str() == selected)
}

/// 加载器选择页里通用的「最新版本」行
pub(crate) fn loader_latest_row(
    icon: IconKeyPair,
    prefix: &'static str,
    snapshot: bool,
    selector: Selector<String>,
) -> impl Widget<LoaderPageData<String>> {
    select_row(
        icon,
        move |data: &LoaderPageData<String>, _| {
            let latest = if snapshot {
                data.state.avaliable_snapshot_versions.front()
            } else {
                data.state.avaliable_versions.front()
            };
            format!("{prefix}{}", latest.map(String::as_str).unwrap_or("无"))
        },
        move |data: &LoaderPageData<String>, _| {
            let latest = if snapshot {
                data.state.avaliable_snapshot_versions.front()
            } else {
                data.state.avaliable_versions.front()
            };
            latest_row_selected(latest, data.selected.as_str())
        },
        move |ctx, data, _| {
            let latest = if snapshot {
                data.state.avaliable_snapshot_versions.front().cloned()
            } else {
                data.state.avaliable_versions.front().cloned()
            };
            if let Some(version) = latest {
                ctx.submit_command(selector.with(version).to(Target::Global));
                ctx.submit_command(QUERY_POP_PAGE.with("").to(Target::Global));
            }
        },
    )
}

/// 加载器选择页里用于 `List` 的一行
pub(crate) fn loader_version_row(
    icon: IconKeyPair,
    selector: Selector<String>,
) -> impl Widget<SelectableVersion<String>> {
    select_row(
        icon,
        |item: &SelectableVersion<String>, _| item.version.clone(),
        |item: &SelectableVersion<String>, _| item.selected,
        move |ctx, item, _| {
            let version = item.version.clone();
            ctx.submit_command(selector.with(version).to(Target::Global));
            ctx.submit_command(QUERY_POP_PAGE.with("").to(Target::Global));
        },
    )
}

/// 加载器选择页里的折叠分组
pub(crate) fn loader_folder<L>(
    icon: IconKeyPair,
    title: &'static str,
    selector: Selector<String>,
    list_lens: L,
) -> impl Widget<LoaderPageData<String>>
where
    L: Lens<LoaderPageData<String>, Vector<String>> + 'static,
{
    let list = druid::widget::List::new(move || loader_version_row(icon.clone(), selector)).lens(
        selected_versions(list_lens, |version: &String| version.clone()),
    );
    FolderList::new(list, title).expand_width()
}

/// 版本列表里的一项（用于原版版本页）
pub(crate) fn vanilla_version_row() -> impl Widget<SelectableVersion<VersionInfo>> {
    select_row(
        GAME_VANILLA,
        |item: &SelectableVersion<VersionInfo>, _| item.version.name.clone(),
        |item: &SelectableVersion<VersionInfo>, _| item.selected,
        |ctx, item, _| {
            ctx.submit_command(
                VANILLA_DOWNLOAD_SELECT
                    .with(item.version.name.clone())
                    .to(Target::Global),
            );
            ctx.submit_command(QUERY_POP_PAGE.with("").to(Target::Global));
        },
    )
}

/// 原版版本选择页里的「最新版本」行
pub(crate) fn vanilla_latest_row(
    prefix: &'static str,
    snapshot: bool,
) -> impl Widget<LoaderPageData<VersionInfo>> {
    select_row(
        GAME_VANILLA,
        move |data: &LoaderPageData<VersionInfo>, _| {
            let latest = if snapshot {
                data.state.avaliable_snapshot_versions.front()
            } else {
                data.state.avaliable_versions.front()
            };
            format!(
                "{prefix}{}",
                latest.map(|v| v.name.as_str()).unwrap_or("无")
            )
        },
        move |data: &LoaderPageData<VersionInfo>, _| {
            let latest = if snapshot {
                data.state.avaliable_snapshot_versions.front()
            } else {
                data.state.avaliable_versions.front()
            };
            data.selected.as_str() == latest.map(|v| v.name.as_str()).unwrap_or("")
        },
        move |ctx, data, _| {
            let latest = if snapshot {
                data.state.avaliable_snapshot_versions.front().cloned()
            } else {
                data.state.avaliable_versions.front().cloned()
            };
            if let Some(version) = latest {
                ctx.submit_command(
                    VANILLA_DOWNLOAD_SELECT
                        .with(version.name)
                        .to(Target::Global),
                );
                ctx.submit_command(QUERY_POP_PAGE.with("").to(Target::Global));
            }
        },
    )
}

/// 原版版本选择页里的折叠分组
pub(crate) fn vanilla_folder<L>(
    title: &'static str,
    list_lens: L,
) -> impl Widget<LoaderPageData<VersionInfo>>
where
    L: Lens<LoaderPageData<VersionInfo>, Vector<VersionInfo>> + 'static,
{
    let list = druid::widget::List::new(vanilla_version_row)
        .lens(selected_versions(list_lens, |version: &VersionInfo| {
            version.name.clone()
        }));
    FolderList::new(list, title).expand_width()
}

fn vanilla_status(_data: &AppState, _env: &Env) -> String {
    // 原版 Minecraft 行只显示已选中的原版版本名称，未选中时显示“加载中”，
    _data
        .selected_download_version
        .as_ref()
        .map(|version| version.name.clone())
        .unwrap_or_else(|| "加载中".to_owned())
}

fn loader_status(
    data: &AppState,
    state: &NetworkData<DownloadState<String>>,
    selected: &str,
) -> String {
    if data.selected_download_version.is_none() {
        return "不可用".to_owned();
    }
    if state.get().is_none() {
        if state.error.is_some() {
            "请求失败".to_owned()
        } else {
            "查询中".to_owned()
        }
    } else if !selected.is_empty() {
        selected.to_owned()
    } else if state
        .get()
        .map(|state| state.all_versions().next().is_none())
        .unwrap_or(false)
    {
        "无可用版本".to_owned()
    } else {
        "可安装".to_owned()
    }
}

fn forge_status(data: &AppState, _env: &Env) -> String {
    loader_status(
        data,
        &data.download_state_forge,
        &data.download_forge_version,
    )
}

fn neoforge_status(data: &AppState, _env: &Env) -> String {
    loader_status(
        data,
        &data.download_state_neoforge,
        &data.download_neoforge_version,
    )
}

fn fabric_status(data: &AppState, _env: &Env) -> String {
    loader_status(
        data,
        &data.download_state_fabric,
        &data.download_fabric_version,
    )
}

fn quiltmc_status(data: &AppState, _env: &Env) -> String {
    loader_status(
        data,
        &data.download_state_quiltmc,
        &data.download_quiltmc_version,
    )
}

fn optifine_status(data: &AppState, _env: &Env) -> String {
    loader_status(
        data,
        &data.download_state_optifine,
        &data.download_optifine_version,
    )
}

fn install_dir_status(data: &AppState, _env: &Env) -> String {
    match &data.config.minecraft_folder {
        crate::config::MinecraftFolder::CurrentDirectory => "当前游戏文件夹".to_owned(),
        crate::config::MinecraftFolder::OfficalDirectory => "官方游戏文件夹".to_owned(),
        crate::config::MinecraftFolder::Other(path) => path.clone(),
    }
}

fn module_row(
    icon: IconKeyPair,
    title: &'static str,
    desc: fn(&AppState, &Env) -> String,
    page: &'static str,
) -> impl Widget<AppState> {
    scl_gui_widgets::widgets::DownloadModuleItem::dynamic(
        icon,
        move |_: &AppState, _: &Env| title.to_owned(),
        desc,
    )
    .on_click(move |ctx, _data, _env| {
        ctx.submit_command(PUSH_PAGE.with(page).to(Target::Global));
    })
}

fn make_request(data: &AppState) -> Option<app::DownloadRequest> {
    let vanilla = data.selected_download_version.clone()?;
    let mut request = app::DownloadRequest::vanilla(vanilla.clone());
    request.forge = data.download_forge_version.clone();
    request.neoforge = data.download_neoforge_version.clone();
    request.fabric = data.download_fabric_version.clone();
    request.quiltmc = data.download_quiltmc_version.clone();
    request.optifine = data.download_optifine_version.clone();
    request.version_name = if data.download_version_name.trim().is_empty() {
        app::download::format_version_name(
            &vanilla.name,
            &request.fabric,
            &request.quiltmc,
            &request.forge,
            &request.neoforge,
            &request.optifine,
        )
    } else {
        data.download_version_name.trim().to_owned()
    };
    Some(request)
}

/// 创建设置页一样的模块行容器
fn module_row_for(
    icon: IconKeyPair,
    title: &'static str,
    desc: fn(&AppState, &Env) -> String,
    page: &'static str,
) -> Box<dyn Widget<AppState>> {
    Box::new(module_row(icon, title, desc, page))
}

/// 创建下载游戏根页面
pub fn ui_builder() -> Box<dyn Widget<AppState>> {
    // 原版输入框只做水平 10px 内缩（Insets::new(10., 0., 10., 0.)），垂直方向不加 padding，
    // 高度固定 32；根页面本身没有 padding。
    let version_name = common::text_box("")
        .lens(AppState::download_version_name)
        .expand_width()
        .fix_height(32.)
        .padding(Insets::new(10., 0., 10., 0.));

    // 原版把六个加载器行（Minecraft/Forge/NeoForge/Fabric/QuiltMC/OptiFine）放在一个
    // 可滚动的 Flex::column 里，由 Scroll 占据剩余空间；
    // 「安装目录」行在 Scroll 之外，紧贴滚动区域底部，因此窗口较矮时会和上一行更近。
    let loader_rows = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(module_row_for(
            GAME_VANILLA,
            "Minecraft",
            vanilla_status,
            PAGE_DOWNLOAD_VANILLA,
        ))
        .with_child(module_row_for(
            GAME_FORGE,
            "Forge",
            forge_status,
            PAGE_DOWNLOAD_FORGE,
        ))
        .with_child(module_row_for(
            GAME_NEOFORGE,
            "NeoForge",
            neoforge_status,
            PAGE_DOWNLOAD_NEOFORGE,
        ))
        .with_child(module_row_for(
            GAME_FABRIC,
            "Fabric",
            fabric_status,
            PAGE_DOWNLOAD_FABRIC,
        ))
        .with_child(module_row_for(
            GAME_QUILTMC,
            "QuiltMC",
            quiltmc_status,
            PAGE_DOWNLOAD_QUILTMC,
        ))
        .with_child(module_row_for(
            GAME_OPTIFINE,
            "Optifine",
            optifine_status,
            PAGE_DOWNLOAD_OPTIFINE,
        ));

    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        // 原版顶部两个标签都是 label::new 默认样式（BASE 13px）并整体 padding(10.)，
        // 不是 BODY 14px；根页面没有额外 padding，水平缩进由这里提供。
        .with_child(label::new("游戏版本名称").padding(10.))
        .with_child(version_name)
        .with_child(label::new("游戏组件").padding(10.))
        .with_flex_child(
            Scroll::new(loader_rows.expand_width()).vertical().expand(),
            1.0,
        )
        .with_child(module_row_for(
            FOLDER,
            "安装目录",
            install_dir_status,
            PAGE_GAME_DIR,
        ))
        .with_child(
            common::accent_button("安装游戏")
                .expand_width()
                .fix_height(32.)
                .padding(10.)
                .on_click(|ctx, data, _env| {
                    if let Some(request) = make_request(data) {
                        ctx.submit_command(DOWNLOAD_GAME.with(request).to(Target::Global));
                    }
                }),
        );

    Box::new(content)
}

#[cfg(test)]
mod tests {
    use super::latest_row_selected;

    #[test]
    fn empty_latest_does_not_select_empty_choice() {
        // 版本列表为空时“最新版本”行显示“无”，选中值为空字符串；
        // 这个空值不能再与“不安装”行的空选中值同时命中。
        assert!(!latest_row_selected(None, ""));
        assert!(!latest_row_selected(None, "1.20.1"));
    }

    #[test]
    fn latest_row_selected_only_for_matching_version() {
        let latest = "1.20.1".to_owned();
        assert!(latest_row_selected(Some(&latest), "1.20.1"));
        assert!(!latest_row_selected(Some(&latest), ""));
        assert!(!latest_row_selected(Some(&latest), "1.20.0"));
    }
}
