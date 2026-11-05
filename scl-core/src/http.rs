//! HTTP 包装，虽然是内部使用但是你也可以使用这个来做点 HTTP 请求什么的
//!
//! 或者在二次开发的时候更换成你喜欢的版本

use std::{convert::TryInto, sync::Arc, time::Duration};

use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use surf::*;

use crate::prelude::*;

static GLOBAL_CLIENT: Lazy<Arc<Client>> = Lazy::new(|| {
    let scl_version = std::option_env!("SCL_VERSION_TYPE").unwrap_or("0.0.0");
    let client = Config::new()
        .add_header(
            "User-Agent",
            format!("SharpCraftLauncher/{scl_version} (github.com/Steve-xmh/SharpCraftLauncher) (stevexmh@qq.com)"),
        )
        .unwrap()
        .set_timeout(Some(Duration::from_secs(30)))
        .set_http_keep_alive(false) // async-h1 似乎不兼容使用 Keep Alive，会导致解析响应出错
        .set_max_connections_per_host(1024);
    let client = if let Ok(mut proxy) = std::env::var("HTTP_PROXY") {
        let proxy = if proxy.ends_with('/') {
            proxy
        } else {
            proxy.push('/');
            proxy
        };
        if let Ok(uri) = url::Url::parse(&proxy) {
            tracing::trace!("Using http proxy: {uri}");
            client.set_base_url(uri)
        } else {
            client
        }
    } else {
        client
    };
    let client: Client = client.try_into().unwrap();
    Arc::new(client.with(middleware::Redirect::default()))
});

/// Future 重试调用函数，为下载文件失败重试而准备
///
/// 主要是 surf 库不带重试功能，中间件写了也有大堆问题。。。
pub async fn retry_future<O, F: std::future::Future<Output = O>>(
    max_retries: usize,
    future_builder: impl Fn() -> F,
    error_handler: impl Fn(&O) -> bool,
) -> DynResult<O> {
    let mut retries = 0;
    loop {
        retries += 1;
        let f = future_builder();
        let r = f.await;
        if error_handler(&r) || retries >= max_retries {
            return Ok(r);
        }
    }
}

/// 根据所给链接，依次尝试请求下载
///
/// 启发自 PCL1 源代码
///
/// TODO: 如 size 参数为非零值，则将会使用分片下载
pub async fn download(
    uris: &[impl AsRef<str> + std::fmt::Debug],
    dest_path: &str,
    _size: usize,
) -> DynResult {
    for uri in uris {
        // 尝试重试两次，都失败的话就换下一个链接
        let res = retry_future(5, || get(uri), surf::Result::is_ok).await;
        match res {
            Ok(Ok(res)) => {
                if res.status().is_success() {
                    let tmp_dest_path = format!("{dest_path}.tmp");
                    let tmp_file = inner_future::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(&tmp_dest_path)
                        .await?;
                    if inner_future::io::copy(res, tmp_file).await.is_ok() {
                        inner_future::fs::rename(tmp_dest_path, dest_path).await?;
                        return Ok(());
                    }
                } else {
                    tracing::trace!("Error {:?} 状态码错误 {}", uri, res.status());
                }
            }
            Ok(Err(e)) => {
                tracing::trace!("Error {uri:?} {e}")
            }
            Err(e) => {
                tracing::trace!("Error {uri:?} {e}")
            }
        }
    }
    anyhow::bail!(
        "轮询下载文件到 {} 失败，请检查你的网络连接，已尝试的链接 {:?}",
        dest_path,
        uris
    )
}

/// 重试获取 JSON 对象
///
/// 返回的数据结构需要实现 [`serde::de::DeserializeOwned`]
pub async fn retry_get_json<D: DeserializeOwned>(uri: impl AsRef<str>) -> DynResult<D> {
    let res = retry_future(5, || get(uri.as_ref()).recv_json(), surf::Result::is_ok).await;
    let err = match res {
        Ok(Ok(body)) => return Ok(body),
        Ok(Err(e)) => {
            anyhow::anyhow!("{}", e)
        }
        Err(e) => e,
    };
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri.as_ref(),
        err
    )
}

/// 重试获取数据
pub async fn retry_get_bytes(uri: impl AsRef<str>) -> DynResult<Vec<u8>> {
    let res = retry_future(5, || get(uri.as_ref()).recv_bytes(), surf::Result::is_ok).await;
    let err = match res {
        Ok(Ok(body)) => return Ok(body),
        Ok(Err(e)) => {
            anyhow::anyhow!("{}", e)
        }
        Err(e) => e,
    };
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri.as_ref(),
        err
    )
}

/// 重试获取字符串
pub async fn retry_get_string(uri: impl AsRef<str>) -> DynResult<String> {
    let res = retry_future(5, || get(uri.as_ref()).recv_string(), surf::Result::is_ok).await;
    let err = match res {
        Ok(Ok(body)) => return Ok(body),
        Ok(Err(e)) => {
            anyhow::anyhow!("{}", e)
        }
        Err(e) => e,
    };
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri.as_ref(),
        err
    )
}

/// 重试获取响应，当取得成功时返回
///
/// 你可能需要自行确认状态码是否成功
pub async fn retry_get(uri: impl AsRef<str>) -> DynResult<Response> {
    let res = retry_future(5, || get(uri.as_ref()), surf::Result::is_ok).await;
    let err = match res {
        Ok(Ok(body)) => return Ok(body),
        Ok(Err(e)) => {
            anyhow::anyhow!(
                "{}: {}",
                e,
                e.backtrace().map(|x| x.to_string()).unwrap_or_default()
            )
        }
        Err(e) => e,
    };
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri.as_ref(),
        err
    )
}

/// 生成简单的 GET 请求
pub fn get(uri: impl AsRef<str>) -> RequestBuilder {
    GLOBAL_CLIENT.get(uri)
}

/// 生成简单的 POST 请求
pub fn post(uri: impl AsRef<str>) -> RequestBuilder {
    GLOBAL_CLIENT.post(uri)
}

/// 针对 Mojang 验证 API 的响应结构
#[derive(Debug, Clone)]
pub enum RequestResult<T> {
    /// 返回的结构是成功的，此处为实际数据
    Ok(T),
    /// 返回的结构是错误的，此处为错误信息结构
    Err(crate::auth::structs::mojang::ErrorResponse),
}

/// 不会进行重试的 HTTP 请求模块
pub mod no_retry {
    use serde::{de::DeserializeOwned, Serialize};

    /// 与全局客户端一致：关闭 keep-alive 并带 30s 超时。
    ///
    /// 原实现直接复用 `surf::get`（默认客户端，keep-alive 开启且无超时），
    /// 在 async-h1 遇到服务器关闭复用的连接时会永久挂起。
    pub fn get(uri: impl AsRef<str>) -> surf::RequestBuilder {
        super::get(uri)
    }

    use super::RequestResult;
    use crate::auth::structs::mojang::ErrorResponse;
    use crate::prelude::DynResult;

    /// 解析 Yggdrasil 响应体。
    ///
    /// 先按成功结构解析，失败再按错误结构（`{"error": ..., "errorMessage": ...}`）解析。
    /// **两种都失败时必须返回第一遍（成功结构）的解析错误**：既有的实现把第二遍
    /// 错误结构解析失败的信息当成最终错误返回，于是成功体的字段缺失会被伪装成
    /// `missing field \`error\``（外置登录真实服务器上的表现），掩盖真正缺的字段。
    fn parse_body<D: DeserializeOwned>(body: &str) -> Result<RequestResult<D>, serde_json::Error> {
        match serde_json::from_str::<D>(body) {
            Ok(result) => Ok(RequestResult::Ok(result)),
            Err(success_err) => match serde_json::from_str::<ErrorResponse>(body) {
                Ok(error) => Ok(RequestResult::Err(error)),
                Err(_) => Err(success_err),
            },
        }
    }

    /// 获取 JSON 对象
    ///
    /// 返回的数据结构需要实现 [`serde::de::DeserializeOwned`]
    ///
    /// 复用 [`GLOBAL_CLIENT`]（关闭 keep-alive + 30s 超时）：裸 `surf::get` 使用
    /// 默认客户端，async-h1 在 keep-alive 连接被服务器关闭后再发请求会一直挂起
    /// （无超时、无错误），实测外置登录在真实服务器上会卡死在元数据请求。
    pub async fn get_data<D: DeserializeOwned>(uri: &str) -> DynResult<RequestResult<D>> {
        let body = super::get(uri)
            .recv_string()
            .await
            .map_err(|e| anyhow::anyhow!("无法接收来自 {} 的响应：{:?}", uri, e))?;
        match parse_body(&body) {
            Ok(result) => Ok(result),
            Err(err) => {
                // 仅在两种结构都解析失败时输出原始响应体，便于定位真实缺口字段；
                // 正常成功/错误响应不会把令牌写进日志。
                tracing::debug!(
                    "[SCL-Core-HTTP] 无法解析 {} 的响应（首个解析错误：{}）：{}",
                    uri,
                    err,
                    body
                );
                Err(err.into())
            }
        }
    }

    /// 带请求体去获取 JSON 对象
    ///
    /// 传入的请求体需要实现 [`serde::ser::Serialize`] 和 [`std::fmt::Debug`]
    ///
    /// 返回的数据结构需要实现 [`serde::de::DeserializeOwned`]
    pub async fn post_data<D: DeserializeOwned, S: Serialize + std::fmt::Debug>(
        uri: &str,
        body: &S,
    ) -> DynResult<RequestResult<D>> {
        // 同上：必须使用带超时且关闭 keep-alive 的全局客户端。
        let body = super::post(uri)
            .header("Content-Type", "application/json; charset=utf-8")
            .body_json(body)
            .map_err(|e| anyhow::anyhow!("无法解析请求主体给 {}：{:?}", uri, e))?
            .recv_string()
            .await
            .map_err(|e| anyhow::anyhow!("无法接收来自 {} 的响应：{:?}", uri, e))?;
        match parse_body(&body) {
            Ok(result) => Ok(result),
            Err(err) => {
                tracing::debug!(
                    "[SCL-Core-HTTP] 无法解析 {} 的响应（首个解析错误：{}）：{}",
                    uri,
                    err,
                    body
                );
                Err(err.into())
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::parse_body;
        use crate::auth::structs::mojang::AuthenticateResponse;
        use crate::http::RequestResult;

        /// littleskin / yggdrasil-connect 的完整成功响应
        const FULL: &str = r#"{"accessToken":"jwt.token.value","clientToken":"cf63005efb2fd19f2d2942786bc9153e","availableProfiles":[{"id":"11111111222233334444555555555555","name":"RoleA"}],"selectedProfile":{"id":"11111111222233334444555555555555","name":"RoleA"},"user":{"id":"aaaa","properties":[]}}"#;

        #[test]
        fn parses_full_success_body() {
            let parsed = parse_body::<AuthenticateResponse>(FULL).unwrap();
            match parsed {
                RequestResult::Ok(response) => {
                    assert_eq!(response.access_token.as_string(), "jwt.token.value");
                    assert_eq!(response.client_token, "cf63005efb2fd19f2d2942786bc9153e");
                    assert_eq!(response.available_profiles.len(), 1);
                    assert_eq!(
                        response.selected_profile.as_ref().unwrap().name, "RoleA"
                    );
                }
                RequestResult::Err(err) => panic!("应解析为成功响应，实际为错误：{err:?}"),
            }
        }

        /// 刷新接口按 Yggdrasil 规范可以省略 availableProfiles（真实服务器出现过），
        /// 此时必须按成功响应解析，而不是退化为错误结构后报 `missing field error`。
        #[test]
        fn parses_refresh_without_available_profiles() {
            let body = r#"{"accessToken":"jwt.token.value","clientToken":"cf63005efb2fd19f2d2942786bc9153e","selectedProfile":{"id":"11111111222233334444555555555555","name":"RoleA"}}"#;
            match parse_body::<AuthenticateResponse>(body).unwrap() {
                RequestResult::Ok(response) => {
                    assert!(response.available_profiles.is_empty());
                    assert_eq!(
                        response.selected_profile.as_ref().unwrap().name, "RoleA"
                    );
                }
                RequestResult::Err(err) => panic!("应解析为成功响应，实际为错误：{err:?}"),
            }
        }

        /// 部分实现不回传 clientToken。
        #[test]
        fn parses_response_without_client_token() {
            let body = r#"{"accessToken":"jwt.token.value","selectedProfile":{"id":"11111111222233334444555555555555","name":"RoleA"}}"#;
            match parse_body::<AuthenticateResponse>(body).unwrap() {
                RequestResult::Ok(response) => {
                    assert_eq!(response.client_token, "");
                }
                RequestResult::Err(err) => panic!("应解析为成功响应，实际为错误：{err:?}"),
            }
        }

        /// 错误响应仍走 Err 分支，保留服务端文案。
        #[test]
        fn parses_error_body() {
            let body = r#"{"error":"ForbiddenOperationException","errorMessage":"用户名或密码错误。Incorrect username or password."}"#;
            match parse_body::<AuthenticateResponse>(body).unwrap() {
                RequestResult::Err(err) => {
                    assert_eq!(err.error, "ForbiddenOperationException");
                }
                RequestResult::Ok(_) => panic!("应解析为错误响应"),
            }
        }

        /// 两种结构都解析失败时，返回的是成功结构的真实错误，而不是
        /// `missing field \`error\``（这是外置登录报错的根因）。
        #[test]
        fn reports_first_parse_error_when_both_fail() {
            let body = r#"{"accessToken":123,"clientToken":"c"}"#;
            let err = parse_body::<AuthenticateResponse>(body).unwrap_err();
            assert!(
                err.to_string().contains("invalid type: integer")
                    || err.to_string().contains("accessToken"),
                "应返回首个解析错误，实际：{err}"
            );
        }
    }
}
