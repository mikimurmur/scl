//! HTTP 包装，虽然是内部使用但是你也可以使用这个来做点 HTTP 请求什么的
//!
//! 或者在二次开发的时候更换成你喜欢的版本
//!
//! 本模块基于阻塞式 [`ureq`]，所有函数均为阻塞式同步调用；需要并行的调用方
//! 应把这些函数放进 `rayon` 线程池执行。

use std::{
    io::Read,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use serde::{de::DeserializeOwned, Serialize};

use crate::prelude::*;

/// 全局 HTTP 客户端：关闭 keep-alive 之外的连接复用问题由 ureq 自行管理，
/// 30 秒连接/读写超时避免服务器异常时永久挂起。
static GLOBAL_CLIENT: OnceLock<ureq::Agent> = OnceLock::new();

fn client() -> &'static ureq::Agent {
    GLOBAL_CLIENT.get_or_init(|| {
        let scl_version = std::option_env!("SCL_VERSION_TYPE").unwrap_or("0.0.0");
        let mut builder = ureq::AgentBuilder::new()
            .user_agent(&format!(
                "SharpCraftLauncher/{scl_version} (github.com/Steve-xmh/SharpCraftLauncher) (stevexmh@qq.com)"
            ))
            .timeout_connect(Duration::from_secs(30))
            .timeout_read(Duration::from_secs(30))
            .timeout_write(Duration::from_secs(30))
            .redirects(5)
            .max_idle_connections_per_host(1024)
            .try_proxy_from_env(true);
        // 显式接管 TLS：native-tls 使用操作系统证书库并做主机名/证书链校验，
        // Linux 动态链接 libssl，Windows 用 SChannel，macOS 用 Security.framework。
        match ureq::native_tls::TlsConnector::new() {
            Ok(connector) => {
                builder = builder.tls_connector(Arc::new(connector));
            }
            Err(err) => {
                tracing::error!("初始化系统 TLS 失败，HTTPS 请求将不可用：{err}");
            }
        }
        builder.build()
    })
}

/// 一个尚未发送的 HTTP 请求
pub struct RequestBuilder {
    inner: ureq::Request,
    body: Option<Vec<u8>>,
}

impl RequestBuilder {
    /// 设置一个请求头
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.inner = self.inner.set(name, value);
        self
    }

    /// 设置整个请求（含读取响应体）的整体超时
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    /// 按需设置整体超时
    pub fn with_optional_timeout(mut self, timeout: Option<Duration>) -> Self {
        if let Some(timeout) = timeout {
            self.inner = self.inner.timeout(timeout);
        }
        self
    }

    /// 设置字符串请求体
    pub fn body_string(mut self, body: impl AsRef<str>) -> Self {
        self.body = Some(body.as_ref().as_bytes().to_vec());
        self
    }

    /// 设置字节请求体
    pub fn body(mut self, body: &[u8]) -> Self {
        self.body = Some(body.to_vec());
        self
    }

    /// 设置 JSON 请求体（自动带上 `Content-Type: application/json`）
    pub fn body_json<T: Serialize + ?Sized>(mut self, body: &T) -> DynResult<Self> {
        self.inner = self
            .inner
            .set("Content-Type", "application/json; charset=utf-8");
        self.body = Some(serde_json::to_vec(body)?);
        Ok(self)
    }

    /// 发送请求并等待响应
    ///
    /// HTTP 4xx/5xx 不作为传输错误返回（与原 `surf` 语义一致），
    /// 调用方通过 [`Response::status`] 自行判断。
    pub fn call(self) -> DynResult<Response> {
        let result = match self.body {
            Some(body) => self.inner.send_bytes(&body),
            None => self.inner.call(),
        };
        match result {
            Ok(response) => Ok(Response(response)),
            Err(ureq::Error::Status(_, response)) => Ok(Response(response)),
            Err(err) => Err(anyhow::anyhow!("{err}")),
        }
    }
}

/// 一个已经收到的 HTTP 响应
pub struct Response(ureq::Response);

impl Response {
    /// HTTP 状态码
    pub fn status(&self) -> u16 {
        self.0.status()
    }

    /// 状态码是否在 200..300 之间
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status())
    }

    /// 获取一个响应头
    pub fn header(&self, name: &str) -> Option<&str> {
        self.0.header(name)
    }

    /// 读取响应体为字符串
    pub fn into_string(self) -> DynResult<String> {
        self.0.into_string().map_err(|err| anyhow::anyhow!(err))
    }

    /// 读取响应体为字节
    pub fn into_bytes(self) -> DynResult<Vec<u8>> {
        let mut reader = self.0.into_reader();
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(data)
    }

    /// 读取响应体为 JSON
    ///
    /// 直接使用 `serde_json` 解析，保留底层 serde 错误形态
    /// （「执行后台任务发生错误」弹窗需要显示 `Error(...)` 的 Debug 文本）。
    pub fn into_json<T: DeserializeOwned>(self) -> DynResult<T> {
        let text = self.into_string()?;
        Ok(serde_json::from_str(&text)?)
    }

    /// 获取响应体读取器
    pub fn into_reader(self) -> Box<dyn Read + Send + Sync + 'static> {
        self.0.into_reader()
    }
}

/// 根据所给链接，依次尝试请求下载
///
/// 启发自 PCL1 源代码
///
/// TODO: 如 size 参数为非零值，则将会使用分片下载
pub fn download(
    uris: &[impl AsRef<str> + std::fmt::Debug],
    dest_path: &str,
    size: usize,
) -> DynResult {
    download_with_timeout(uris, dest_path, size, None)
}

/// 与 [`download`] 相同，但给每个链接一个整体超时窗口
///
/// 不可达镜像源的 TCP 连接会停在 SYN-SENT 或持续无响应，没有超时会让整个
/// 下载/修复任务（DEV-05）永久挂起。超时后继续尝试下一个链接。
pub fn download_with_timeout(
    uris: &[impl AsRef<str> + std::fmt::Debug],
    dest_path: &str,
    size: usize,
    timeout: Option<Duration>,
) -> DynResult {
    download_verified_with_timeout(uris, dest_path, size, None, None, timeout)
}

/// 与 [`download_with_timeout`] 相同，并在 rename 前校验下载内容的大小与 SHA1。
///
/// 校验失败不会把坏内容原子替换到目标路径；临时文件删除后会继续尝试同一
/// 链接（最多 5 次）以及后续链接。`expected_size == 0` 或空 SHA1 表示跳过对应校验。
pub fn download_verified_with_timeout(
    uris: &[impl AsRef<str> + std::fmt::Debug],
    dest_path: &str,
    _size: usize,
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
    timeout: Option<Duration>,
) -> DynResult {
    for uri in uris {
        let deadline = timeout.map(|timeout| Instant::now() + timeout);
        let mut retries = 0;
        loop {
            retries += 1;
            let mut request = get(uri);
            if let Some(deadline) = deadline {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    tracing::trace!("Error {:?} 下载超时", uri);
                    break;
                }
                request = request.timeout(remaining);
            }
            match request.call() {
                Ok(response) => {
                    if response.is_success() {
                        let tmp_dest_path = format!("{dest_path}.tmp");
                        let result = (|| -> DynResult {
                            let mut file = fs_err::File::create(&tmp_dest_path)?;
                            std::io::copy(&mut response.into_reader(), &mut file)?;
                            drop(file);
                            verify_downloaded_file(&tmp_dest_path, expected_sha1, expected_size)?;
                            fs_err::rename(&tmp_dest_path, dest_path)?;
                            Ok(())
                        })();
                        if result.is_ok() {
                            return Ok(());
                        }
                        let _ = fs_err::remove_file(&tmp_dest_path);
                        if retries >= 5 {
                            break;
                        }
                    } else {
                        tracing::trace!("Error {:?} 状态码错误 {}", uri, response.status());
                        break;
                    }
                }
                Err(err) => {
                    tracing::trace!("Error {:?} {}", uri, err);
                    if retries >= 5 {
                        break;
                    }
                }
            }
        }
    }
    anyhow::bail!(
        "轮询下载文件到 {} 失败，请检查你的网络连接，已尝试的链接 {:?}",
        dest_path,
        uris
    )
}

/// 校验刚下载到临时路径的文件大小与 SHA1
fn verify_downloaded_file(
    path: &str,
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
) -> DynResult {
    let meta = fs_err::metadata(path)?;
    if let Some(size) = expected_size {
        if size > 0 && meta.len() != size {
            anyhow::bail!(
                "下载文件 {} 大小不匹配：期望 {} 字节，实际 {} 字节",
                path,
                size,
                meta.len()
            );
        }
    }
    if let Some(sha1) = expected_sha1.filter(|sha1| !sha1.is_empty()) {
        let mut file = fs_err::File::open(path)?;
        let actual = crate::utils::get_data_sha1(&mut file)?;
        if !actual.eq_ignore_ascii_case(sha1) {
            anyhow::bail!(
                "下载文件 {} SHA1 不匹配：期望 {}，实际 {}",
                path,
                sha1,
                actual
            );
        }
    }
    Ok(())
}

/// 重试获取 JSON 对象
///
/// 返回的数据结构需要实现 [`serde::de::DeserializeOwned`]
pub fn retry_get_json<D: DeserializeOwned>(uri: impl AsRef<str>) -> DynResult<D> {
    retry_get_json_timeout(uri, None)
}

/// [`retry_get_json`] 的带整体超时版本
pub fn retry_get_json_timeout<D: DeserializeOwned>(
    uri: impl AsRef<str>,
    timeout: Option<Duration>,
) -> DynResult<D> {
    let uri = uri.as_ref();
    let mut last_err = None;
    for _ in 0..5 {
        match get(uri)
            .with_optional_timeout(timeout)
            .call()
            .and_then(|response| response.into_json::<D>())
        {
            Ok(body) => return Ok(body),
            Err(err) => last_err = Some(err),
        }
    }
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri,
        last_err.map(|err| format!("{err:?}")).unwrap_or_default()
    )
}

/// 重试获取数据
pub fn retry_get_bytes(uri: impl AsRef<str>) -> DynResult<Vec<u8>> {
    retry_get_bytes_timeout(uri, None)
}

/// [`retry_get_bytes`] 的带整体超时版本
pub fn retry_get_bytes_timeout(
    uri: impl AsRef<str>,
    timeout: Option<Duration>,
) -> DynResult<Vec<u8>> {
    let uri = uri.as_ref();
    let mut last_err = None;
    for _ in 0..5 {
        match get(uri)
            .with_optional_timeout(timeout)
            .call()
            .and_then(|response| response.into_bytes())
        {
            Ok(body) => return Ok(body),
            Err(err) => last_err = Some(err),
        }
    }
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri,
        last_err.map(|err| format!("{err:?}")).unwrap_or_default()
    )
}

/// 重试获取字符串
pub fn retry_get_string(uri: impl AsRef<str>) -> DynResult<String> {
    retry_get_string_timeout(uri, None)
}

/// [`retry_get_string`] 的带整体超时版本
pub fn retry_get_string_timeout(
    uri: impl AsRef<str>,
    timeout: Option<Duration>,
) -> DynResult<String> {
    let uri = uri.as_ref();
    let mut last_err = None;
    for _ in 0..5 {
        match get(uri)
            .with_optional_timeout(timeout)
            .call()
            .and_then(|response| response.into_string())
        {
            Ok(body) => return Ok(body),
            Err(err) => last_err = Some(err),
        }
    }
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri,
        last_err.map(|err| format!("{err:?}")).unwrap_or_default()
    )
}

/// 重试获取响应，当取得成功时返回
///
/// 你可能需要自行确认状态码是否成功
pub fn retry_get(uri: impl AsRef<str>) -> DynResult<Response> {
    retry_get_timeout(uri, None)
}

/// [`retry_get`] 的带整体超时版本
pub fn retry_get_timeout(uri: impl AsRef<str>, timeout: Option<Duration>) -> DynResult<Response> {
    let uri = uri.as_ref();
    let mut last_err = None;
    for _ in 0..5 {
        match get(uri).with_optional_timeout(timeout).call() {
            Ok(response) => return Ok(response),
            Err(err) => last_err = Some(err),
        }
    }
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri,
        last_err.map(|err| format!("{err:?}")).unwrap_or_default()
    )
}

/// 生成简单的 GET 请求
pub fn get(uri: impl AsRef<str>) -> RequestBuilder {
    RequestBuilder {
        inner: client().get(uri.as_ref()),
        body: None,
    }
}

/// 生成简单的 POST 请求
pub fn post(uri: impl AsRef<str>) -> RequestBuilder {
    RequestBuilder {
        inner: client().post(uri.as_ref()),
        body: None,
    }
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

    use super::RequestResult;
    use crate::{auth::structs::mojang::ErrorResponse, prelude::DynResult};

    /// 使用与全局客户端一致的连接/读写超时
    pub fn get(uri: impl AsRef<str>) -> super::RequestBuilder {
        super::get(uri)
    }

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
    pub fn get_data<D: DeserializeOwned>(uri: &str) -> DynResult<RequestResult<D>> {
        let body = super::get(uri)
            .call()
            .and_then(|response| response.into_string())
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

    /// 带请求体去获取 JSON 对象
    ///
    /// 传入的请求体需要实现 [`serde::ser::Serialize`] 和 [`std::fmt::Debug`]
    ///
    /// 返回的数据结构需要实现 [`serde::de::DeserializeOwned`]
    pub fn post_data<D: DeserializeOwned, S: Serialize + std::fmt::Debug>(
        uri: &str,
        body: &S,
    ) -> DynResult<RequestResult<D>> {
        let request_body = serde_json::to_vec(body)
            .map_err(|e| anyhow::anyhow!("无法解析请求主体给 {}：{:?}", uri, e))?;
        let body = super::post(uri)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(&request_body)
            .call()
            .and_then(|response| response.into_string())
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
        use crate::{auth::structs::mojang::AuthenticateResponse, http::RequestResult};

        const FULL: &str = r#"{"accessToken":"jwt.token.value","clientToken":"cf63005efb2fd19f2d2942786bc9153e","availableProfiles":[{"id":"11111111222233334444555555555555","name":"RoleA"}],"selectedProfile":{"id":"11111111222233334444555555555555","name":"RoleA"},"user":{"id":"aaaa","properties":[]}}"#;

        #[test]
        fn parses_full_success_body() {
            let parsed = parse_body::<AuthenticateResponse>(FULL).unwrap();
            match parsed {
                RequestResult::Ok(response) => {
                    assert_eq!(response.access_token.as_string(), "jwt.token.value");
                    assert_eq!(response.client_token, "cf63005efb2fd19f2d2942786bc9153e");
                    assert_eq!(response.available_profiles.len(), 1);
                    assert_eq!(response.selected_profile.as_ref().unwrap().name, "RoleA");
                }
                RequestResult::Err(err) => panic!("应解析为成功响应，实际为错误：{err:?}"),
            }
        }

        #[test]
        fn parses_refresh_without_available_profiles() {
            let body = r#"{"accessToken":"jwt.token.value","clientToken":"cf63005efb2fd19f2d2942786bc9153e","selectedProfile":{"id":"11111111222233334444555555555555","name":"RoleA"}}"#;
            match parse_body::<AuthenticateResponse>(body).unwrap() {
                RequestResult::Ok(response) => {
                    assert!(response.available_profiles.is_empty());
                    assert_eq!(response.selected_profile.as_ref().unwrap().name, "RoleA");
                }
                RequestResult::Err(err) => panic!("应解析为成功响应，实际为错误：{err:?}"),
            }
        }

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

#[cfg(test)]
mod timeout_tests {
    use std::{
        io::Write,
        net::TcpListener,
        thread,
        time::{Duration, Instant},
    };

    use super::{download_verified_with_timeout, download_with_timeout};

    /// 本地 HTTP 测试串行执行，避免多个测试同时占用全局 ureq 客户端。
    static NETWORK_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock_network_test() -> std::sync::MutexGuard<'static, ()> {
        NETWORK_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn serve_body(
        listener: TcpListener,
        body: &'static [u8],
        connections: usize,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            for _ in 0..connections {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(body);
                let _ = stream.flush();
                // 给客户端留出读取窗口，避免测试线程先退出造成空响应。
                thread::sleep(Duration::from_millis(10));
            }
        })
    }

    /// 一个接受连接但永不响应的服务器必须被超时窗口打断，
    /// 否则下载/修复任务会永久挂起（DEV-05）。
    #[test]
    fn download_times_out_on_stalled_server() {
        let _guard = lock_network_test();
        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定本地端口");
        let addr = listener.local_addr().expect("获取本地地址");
        let handle = thread::spawn(move || {
            // 只接受连接、读掉请求，不发送任何响应
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.write_all(b"");
                thread::sleep(Duration::from_secs(10));
            }
        });

        let uri = format!("http://{addr}/stall");
        let start = Instant::now();
        let result = download_with_timeout(
            std::slice::from_ref(&uri),
            "/tmp/scl-http-timeout-test.bin",
            0,
            Some(Duration::from_millis(200)),
        );
        assert!(result.is_err(), "不可响应的服务器应返回下载失败");
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "超时窗口应限制等待时间，实际 {:?}",
            start.elapsed()
        );
        let _ = fs_err::remove_file("/tmp/scl-http-timeout-test.bin.tmp");
        handle.join().ok();
    }

    /// 内容 SHA1 不匹配时不得把坏内容落盘到目标路径（P0 下载后校验）。
    #[test]
    fn download_rejects_bad_sha1() {
        let _guard = lock_network_test();
        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定本地端口");
        let addr = listener.local_addr().expect("获取本地地址");
        let handle = serve_body(listener, b"bad", 5);
        let dest = format!("/tmp/scl-http-bad-sha1-{}.bin", std::process::id());
        let _ = fs_err::remove_file(&dest);
        let uri = format!("http://{addr}/bad");
        let result = download_verified_with_timeout(
            std::slice::from_ref(&uri),
            &dest,
            0,
            Some("0000000000000000000000000000000000000000"),
            Some(3),
            Some(Duration::from_secs(5)),
        );
        assert!(result.is_err(), "SHA1 不匹配应返回错误");
        assert!(!std::path::Path::new(&dest).exists(), "坏内容不得落盘");
        let _ = fs_err::remove_file(format!("{dest}.tmp"));
        handle.join().ok();
    }

    /// 大小与 SHA1 都匹配时应正常落盘。
    #[test]
    fn download_accepts_matching_sha1_and_size() {
        let _guard = lock_network_test();
        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定本地端口");
        let addr = listener.local_addr().expect("获取本地地址");
        let handle = serve_body(listener, b"good", 1);
        let dest = format!("/tmp/scl-http-good-sha1-{}.bin", std::process::id());
        let _ = fs_err::remove_file(&dest);
        let sha1 = sha1_smol::Sha1::from(b"good").hexdigest();
        let uri = format!("http://{addr}/good");
        download_verified_with_timeout(
            std::slice::from_ref(&uri),
            &dest,
            0,
            Some(&sha1),
            Some(4),
            Some(Duration::from_secs(5)),
        )
        .expect("匹配的大小与 SHA1 应下载成功");
        assert_eq!(fs_err::read(&dest).expect("读取下载文件"), b"good".to_vec());
        let _ = fs_err::remove_file(&dest);
        handle.join().ok();
    }
}
