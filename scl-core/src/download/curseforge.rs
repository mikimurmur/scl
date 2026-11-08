//! CurseForge 模组下载的结构和接口

/*
    基本链接：https://addons-ecs.forgesvc.net/api/v2/addon/
    某个模组：https://addons-ecs.forgesvc.net/api/v2/addon/[MOD_ID]
    模组详情：https://addons-ecs.forgesvc.net/api/v2/addon/[MOD_ID]/description
    模组文件：https://addons-ecs.forgesvc.net/api/v2/addon/[MOD_ID]/files
    搜索模组：https://addons-ecs.forgesvc.net/api/v2/addon/search
            请求字符串： gameId = 432
                        gameVersion
                        sectionId = 6
                        searchFilter
                        categoryID
                        index
                        pageSize
                        sort:
                            FEATURED: 0
                            POPULARITY: 1
                            LAST_UPDATE: 2
                            NAME: 3
                            AUTHOR: 4
                            TOTAL_DOWNLOADS: 5
*/

use std::{
    fmt::Write as _,
    io::Write as _,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use serde::de::DeserializeOwned;

use crate::prelude::*;

const API_KEY: &str = env!("CURSEFORGE_API_KEY");
const BASE_URL: &str = "https://api.curseforge.com/v1/";
const BASE_URL_SEARCH: &str = "https://api.curseforge.com/v1/mods/search?gameId=432&classId=6";

/// CurseForge 最新的 CDN 请求要求同样携带 API Key。
///
/// 不仅 `api.curseforge.com` 的接口请求需要 `x-api-key`，图标与模组文件的
/// CDN（`media.forgecdn.net` / `edge.forgecdn.net`）请求也需要携带该头，
/// 否则 CDN 会返回 403 或空内容。见 [`get_mod_icon_url`] 与 [`download_mod`]。
fn with_api_key(request: crate::http::RequestBuilder) -> crate::http::RequestBuilder {
    request.header("x-api-key", API_KEY)
}

/// 检查 CurseForge 接口状态码，并把未授权/失败转成可读提示。
fn check_api_status(status: u16, action: &str) -> DynResult {
    if status == 401 || status == 403 {
        anyhow::bail!(
            "CurseForge {action}被拒绝（HTTP {status}）：API key 无效或未生效，请检查 CURSEFORGE_API_KEY"
        );
    }
    if !(200..300).contains(&status) {
        anyhow::bail!("CurseForge {action}请求失败（HTTP {status}）");
    }
    Ok(())
}

/// 解析 CurseForge 接口响应体，把「空响应/HTML 拦截页」转成可读提示。
///
/// 合法 JSON 仍交给 `serde_json` 解析，以保留 OB-02 所需的底层 serde Debug
/// 文本（例如 null 字段的 `line/column`）。
fn decode_api_body<T: DeserializeOwned>(status: u16, body: &str, action: &str) -> DynResult<T> {
    check_api_status(status, action)?;
    let trimmed = body.trim();
    if trimmed.is_empty() {
        anyhow::bail!("CurseForge {action}返回为空（HTTP {status}），请检查 API key 与网络连接");
    }
    if trimmed.starts_with('<') {
        anyhow::bail!(
            "CurseForge {action}返回了非 JSON 内容（HTTP {status}），请检查 API key 与网络代理"
        );
    }
    serde_json::from_str(body).map_err(Into::into)
}

/// 解析 CurseForge 接口响应，并在读取 body 前先检查状态码。
fn decode_api_json<T: DeserializeOwned>(
    response: crate::http::Response,
    action: &str,
) -> DynResult<T> {
    let status = response.status();
    check_api_status(status, action)?;
    let body = response
        .into_string()
        .map_err(|err| anyhow::anyhow!("CurseForge {action}读取响应失败：{err:?}"))?;
    decode_api_body(status, &body, action)
}

#[derive(Debug, Deserialize)]
struct Response<T> {
    pub data: T,
}

impl<T> Deref for Response<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Response<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/// 一个模组资源信息
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModAsset {
    /// 此模组文件的文件 ID 编号
    pub id: i32,
    /// 此模组文件对应的模组 ID
    pub mod_id: i32,
    /// 模组文件的标题（不一定是文件名）
    pub title: String,
    /// 模组文件的介绍（一般是作者的更新记录什么的）
    pub description: String,
    /// 模组文件的缩略图
    pub thumbnail_url: String,
    /// 模组文件的下载链接
    pub url: String,
}

/// 一个模组的信息
#[derive(Debug, Deserialize)]
pub struct ModInfo {
    /// 模组的 ID
    pub id: u64,
    /// 模组的名称
    pub name: String,
    /// 模组的简短介绍
    pub summary: String,
    /// 模组的 Slug（一般是模组的字符串 ID）
    pub slug: String,
    /// 模组的 LOGO 图标
    pub logo: Option<ModAsset>,
}

/// 模组的所需依赖
///
/// TODO：完善模组依赖下载功能
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    // mod_id: i32,
    // relation_type: u8,
}

/// 一个模组文件信息
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFile {
    /// 模组文件的文件名
    pub file_name: String,
    /// 模组文件的下载链接
    pub download_url: String,
    /// 模组的所需依赖
    pub dependencies: Vec<Dependency>,
    /// 模组支持的游戏版本
    pub game_versions: Vec<String>,
}

/// 使用搜索 API 时的排序方式
#[derive(Debug, Clone, Copy, Default)]
pub enum SearchSortMethod {
    /// 按推荐排序
    #[default]
    Featured,
    /// 按热门度排序
    Populatity,
    /// 按最新更新排序
    LastUpdate,
    /// 按名称排序
    Name,
    /// 按作者名称排序
    Author,
    /// 按总下载量排序
    TotalDownloads,
}

impl SearchSortMethod {
    fn to_query(self) -> u8 {
        match self {
            SearchSortMethod::Featured => 0,
            SearchSortMethod::Populatity => 1,
            SearchSortMethod::LastUpdate => 2,
            SearchSortMethod::Name => 3,
            SearchSortMethod::Author => 4,
            SearchSortMethod::TotalDownloads => 5,
        }
    }
}

/// 搜索参数，将其传入到 [`self::search_mods`] 方法以搜索模组
#[derive(Default)]
pub struct SearchParams {
    /// 搜索支持指定游戏版本的模组
    pub game_version: String,
    /// 当前的搜索页码
    pub index: u64,
    /// 当前搜索的每页项目数量
    pub page_size: u64,
    /// 模组类型 ID
    pub category_id: u64,
    /// 搜索的关键字
    pub search_filter: String,
    /// 搜索结果的排序方式
    pub sort: SearchSortMethod,
}

/// 根据关键词从 Curseforge 搜索模组列表
pub fn search_mods(
    SearchParams {
        game_version,
        index,
        page_size,
        category_id,
        search_filter,
        sort,
    }: SearchParams,
) -> DynResult<Vec<ModInfo>> {
    let mut base_url = BASE_URL_SEARCH.to_string();
    let _ = write!(&mut base_url, "&sort={}", sort.to_query());
    if !search_filter.is_empty() {
        let _ = write!(
            &mut base_url,
            "&searchFilter={}",
            urlencoding::encode(&search_filter)
        );
    }
    if !game_version.is_empty() {
        let _ = write!(&mut base_url, "&gameVersion={game_version}");
    }
    if index > 0 {
        let _ = write!(&mut base_url, "&index={index}");
    }
    if page_size > 0 && page_size <= 30 {
        let _ = write!(&mut base_url, "&pageSize={page_size}");
    } else {
        let _ = write!(&mut base_url, "&pageSize={}", 20);
    }
    if category_id > 0 {
        let _ = write!(&mut base_url, "&categoryID={category_id}");
    }
    tracing::trace!("Searching by {base_url}");
    let response = crate::http::get(&base_url)
        .header("x-api-key", API_KEY)
        .call()?;
    let data: Response<Vec<ModInfo>> = decode_api_json(response, "搜索接口")?;
    Ok(data.data)
}

/// 通过模组在 Curseforge 的 ID 获取详情信息
pub fn get_mod_info(modid: u64) -> DynResult<ModInfo> {
    let response = crate::http::get(format!("{BASE_URL}mods/{modid}"))
        .header("x-api-key", API_KEY)
        .call()?;
    let data: Response<ModInfo> = decode_api_json(response, "模组信息接口")?;
    Ok(data.data)
}

/// 获取模组在 Curseforge 的 ID 获取可下载的模组文件列表
pub fn get_mod_files(modid: u64) -> DynResult<Vec<ModFile>> {
    let response = crate::http::get(format!("{BASE_URL}mods/{modid}/files"))
        .header("x-api-key", API_KEY)
        .call()?;
    let data: Response<Vec<ModFile>> = decode_api_json(response, "文件列表接口")?;
    Ok(data.data)
}

/// 获取模组在 Curseforge 的 ID 获取模组的图标
/// 通过图标 URL 获取模组图标
pub fn get_mod_icon_url(url: &str) -> DynResult<image::DynamicImage> {
    // CDN 请求同样需要携带 API Key（CurseForge 新规）。
    let data = with_api_key(crate::http::get(url)).call()?.into_bytes()?;
    // 官方在 mods_download_*::get_icon 闭包里对图标做 resize_to_fill
    image::load_from_memory(&data)
        .map(|img| img.resize_to_fill(64, 64, image::imageops::FilterType::Triangle))
        .map_err(|e| anyhow::anyhow!(e))
}

/// 通过模组信息里的 logo 地址获取模组图标
pub fn get_mod_icon(mod_info: &ModInfo) -> DynResult<image::DynamicImage> {
    if let Some(logo) = &mod_info.logo {
        get_mod_icon_url(&logo.thumbnail_url)
    } else {
        anyhow::bail!("Mod icon image is empty")
    }
}

/// 获取模组在 Curseforge 的 ID 获取模组的图标
pub fn get_mod_icon_by_id(modid: u64) -> DynResult<image::DynamicImage> {
    let mod_info = get_mod_info(modid)?;
    get_mod_icon(&mod_info)
}

/// 下载模组
pub fn download_mod(
    _ctx: Option<impl Reporter>,
    _name: &str,
    url: &str,
    dest: PathBuf,
) -> DynResult {
    // CDN 请求同样需要携带 API Key（CurseForge 新规）。
    let response = with_api_key(crate::http::get(url)).call()?;
    if !response.is_success() {
        anyhow::bail!(
            "下载模组文件到 {} 失败：CDN 返回状态码 {}",
            dest.display(),
            response.status()
        );
    }
    let temp_path = format!("{}.tmp", dest.to_str().unwrap());
    let result = (|| -> DynResult {
        let mut file = fs_err::File::create(&temp_path)?;
        std::io::copy(&mut response.into_reader(), &mut file)?;
        file.flush()?;
        fs_err::rename(&temp_path, &dest)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs_err::remove_file(&temp_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::decode_api_body;

    /// 「执行后台任务发生错误」弹窗的错误文本保留底层 serde Debug 形态。
    ///
    /// 直接使用 serde_json 的解析错误（不再经过 surf 包装），
    /// GUI 层以 `{err:#?}`（alternate）取底层 Debug。
    #[test]
    fn body_json_error_keeps_serde_debug_shape() {
        #[derive(serde::Deserialize, Debug)]
        struct Sample {
            // 字段只需存在以触发 serde 解析错误，无需读取
            #[allow(dead_code)]
            slug: String,
        }
        // 与 CF API 同款：字段为 null 而结构体声明 String
        let body = r#"[{"slug":null}]"#.as_bytes();
        let err = serde_json::from_slice::<Vec<Sample>>(body).unwrap_err();
        let rendered = format!("{err:#?}");
        assert!(
            rendered.starts_with("Error(\"invalid type: null, expected a string\""),
            "应为 serde Debug 形态，实际：{rendered}"
        );
        assert!(
            rendered.contains("line: 1, column:"),
            "应含 line/column 定位，实际：{rendered}"
        );
        assert!(
            !rendered.contains(" at line 1 column"),
            "不应是 Display 形态（at line ... column ...），实际：{rendered}"
        );
    }

    /// 401/403 必须给出「API key 无效或未生效」的可读提示，而不是 serde 行列号。
    #[test]
    fn unauthorized_response_reports_api_key_hint() {
        let err = decode_api_body::<serde_json::Value>(403, "", "搜索接口").unwrap_err();
        let text = err.to_string();
        assert!(text.contains("API key"), "{text}");
        assert!(!text.contains("line: 1, column"), "{text}");
    }

    /// 200 但空 body（或 HTML 拦截页）也要给出可读提示。
    #[test]
    fn empty_or_html_response_reports_readable_hint() {
        let err = decode_api_body::<serde_json::Value>(200, "", "搜索接口").unwrap_err();
        assert!(err.to_string().contains("返回为空"), "{err}");

        let err = decode_api_body::<serde_json::Value>(200, "<html>blocked</html>", "搜索接口")
            .unwrap_err();
        assert!(err.to_string().contains("非 JSON 内容"), "{err}");
    }
}
