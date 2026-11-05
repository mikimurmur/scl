//! 设置页面的后台逻辑。

use druid::{EventCtx, Target};

use crate::{
    app::{AppEvent, APP_EVENT},
    app_state::AppState,
    utils::{thread::thread_main, update},
};

/// 检查启动器更新
pub fn check_update(ctx: &EventCtx, data: &mut AppState) {
    data.setting_about_update_logs = Some("检查更新中……".into());
    let sink = ctx.get_external_handle();
    thread_main(move || {
        match update::latest_version() {
            Ok(version) => {
                // 官方关于页检查更新只请求 latest 并立刻把整段响应体当作版本号，
                // 不等待更新记录拉取；先提交版本让关于页立即收敛（Cloudflare
                // 拦截页 HTML 也原样展示），更新记录随后在后台补上。
                let _ = sink.submit_command(
                    APP_EVENT,
                    AppEvent::UpdateChecked(Ok((version.clone(), String::new()))),
                    Target::Global,
                );
                let logs = update::fetch_update_logs(&version);
                let _ = sink.submit_command(
                    APP_EVENT,
                    AppEvent::UpdateChecked(Ok((version, logs))),
                    Target::Global,
                );
            }
            Err(err) => {
                let _ = sink.submit_command(
                    APP_EVENT,
                    AppEvent::UpdateChecked(Err(format!("{err:?}"))),
                    Target::Global,
                );
            }
        }
        Ok(())
    });
}

/// 下载最新版并启动内置更新器覆盖当前程序，成功后退出启动器
pub fn update_now(ctx: &EventCtx, _data: &mut AppState) {
    let sink = ctx.get_external_handle();
    thread_main(move || {
        let result = update::self_update();
        let _ = sink.submit_command(
            APP_EVENT,
            AppEvent::UpdateFinished(result.map_err(|err| format!("{err:?}"))),
            Target::Global,
        );
        Ok(())
    });
}
