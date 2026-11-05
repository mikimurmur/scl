//! Microsoft 正版账户登录。

use std::time::Duration;

use druid::{EventCtx, Target};
use scl_core::auth::microsoft::MicrosoftOAuth;

use crate::{
    app::{AppEvent, APP_EVENT},
    app_state::AppState,
    core::CoreTransformer,
    utils::thread::thread_main,
};

/// 启动器使用的 Microsoft OAuth 客户端 ID
pub const CLIENT_ID: &str = env!("SCL_MICROSOFT_CLIENT_ID");

/// 开始微软设备码登录
pub fn login(ctx: &EventCtx, data: &mut AppState) {
    if data.login_busy {
        return;
    }
    let sink = ctx.get_external_handle();
    data.login_busy = true;
    data.login_error = None;
    data.login_microsoft_code.clear();
    data.login_microsoft_url.clear();
    thread_main(move || {
        let oauth = MicrosoftOAuth::new(CLIENT_ID);
        let device = oauth.get_devicecode()?;
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::MicrosoftCode {
                code: device.user_code.to_owned(),
                url: format!(
                    "{}?client_id={}&code={}",
                    device.verification_uri, CLIENT_ID, device.user_code
                ),
            },
            Target::Global,
        );
        let interval = device.interval.max(3) as u64;
        let mut last_error = String::new();
        for _ in 0..(device.expires_in / interval as usize).max(1) {
            std::thread::sleep(Duration::from_secs(interval));
            match oauth.verify_device_code(&device.device_code) {
                Ok(token) => {
                    if !token.error.is_empty() {
                        // 官方按设备码轮询结果的错误码分别给出提示，其余错误原样带出。
                        match token.error.as_str() {
                            "authorization_pending" => {
                                tracing::trace!("登录失败，正在重试：{}", token.error);
                                continue;
                            }
                            "authorization_declined" => {
                                last_error = "授权验证被拒绝，无法登录".into();
                                break;
                            }
                            "bad_verification_code" => {
                                last_error = "验证设备码发生错误：\n设备代码有误".into();
                                break;
                            }
                            "expired_token" => {
                                last_error = "验证设备码已过期，请离开此页面重新获取".into();
                                break;
                            }
                            other => {
                                last_error =
                                    format!("验证设备码发生错误：\n未知的错误回调字符串：{other}");
                                break;
                            }
                        }
                    }
                    let access_token = token.access_token.to_owned_string();
                    let refresh_token = token.refresh_token.to_owned();
                    let _ =
                        sink.submit_command(APP_EVENT, AppEvent::MicrosoftSuccess, Target::Global);
                    match oauth.start_auth(&access_token, &refresh_token) {
                        Ok(auth) => {
                            let _ = sink.submit_command(
                                APP_EVENT,
                                AppEvent::AuthAdded(crate::core::AuthMethod::from_core(auth)),
                                Target::Global,
                            );
                            return Ok(());
                        }
                        Err(err) => {
                            last_error = format!("{err:?}");
                            break;
                        }
                    }
                }
                Err(err) => {
                    // 单次轮询失败不终止登录，保持与原版一致的「失败后重试」行为。
                    last_error = format!("验证设备码发生错误：\n{err}");
                    tracing::error!("登录失败，正在重试：{err}");
                }
            }
        }
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::AuthFailed(if last_error.is_empty() {
                "微软登录超时，请重试".into()
            } else {
                last_error
            }),
            Target::Global,
        );
        Ok(())
    });
}
