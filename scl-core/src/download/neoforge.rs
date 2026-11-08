//! NeoForge 模组加载器的下载模块
use std::{
    collections::VecDeque,
    io::{BufRead, Read, Write},
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::Context;
use serde_json::Value;

use super::{
    structs::{NeoForgeItemInfo, NeoForgeVersionsData},
    Downloader,
};
use crate::{download::DownloadSource, prelude::*};

/// NeoForge 官方 Maven 的库根地址
const NEOFORGE_OFFICIAL_MAVEN: &str = "https://maven.neoforged.net/releases";
/// BMCLAPI 提供的 Maven 镜像根地址
const NEOFORGE_MIRROR_MAVEN: &str = "https://bmclapi2.bangbang93.com/maven";
/// 安装器失败时保留的输出行数上限
const INSTALLER_OUTPUT_TAIL_LINES: usize = 40;
/// 单行安装器输出的字符上限，避免异常栈或超长 URL 占满内存与弹窗
const INSTALLER_OUTPUT_LINE_CHARS: usize = 200;

/// 把安装器输出的一行放入有界尾部缓冲
fn push_installer_output(tail: &mut VecDeque<String>, line: &str) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    let line: String = line.chars().take(INSTALLER_OUTPUT_LINE_CHARS).collect();
    if tail.len() == INSTALLER_OUTPUT_TAIL_LINES {
        tail.pop_front();
    }
    tail.push_back(line);
}

/// 从安装器输出尾部提炼少量可定位失败原因的摘要行
///
/// 安装器完整输出会写进日志，弹窗只展示去重后的异常行与失败库列表，
/// 避免把几十行栈帧直接塞进对话框。
fn installer_failure_summary(tail: &[String]) -> Vec<String> {
    const MAX_CAUSE_LINES: usize = 3;
    const MAX_FAILED_LIBRARIES: usize = 6;
    const MAX_SUMMARY_LINES: usize = 9;

    let is_cause = |line: &str| {
        line.contains("Exception")
            || line.starts_with("Caused by:")
            || line.contains("Error:")
            || line.contains("failed to download")
            || line.contains("Could not")
            || line.contains("Unable to")
    };

    let mut causes: Vec<String> = Vec::new();
    for line in tail {
        if is_cause(line) && !causes.contains(line) {
            causes.push(line.clone());
        }
    }
    if causes.len() > MAX_CAUSE_LINES {
        causes.drain(..causes.len() - MAX_CAUSE_LINES);
    }

    let mut summary = causes;
    if let Some(index) = tail
        .iter()
        .rposition(|line| line.contains("failed to download"))
    {
        for line in tail.iter().skip(index).take(MAX_FAILED_LIBRARIES + 1) {
            if !summary.contains(line) {
                summary.push(line.clone());
            }
        }
    }

    if summary.is_empty() {
        // 没有识别到异常行时，至少给出最后几行原始输出。
        summary.extend(tail.iter().rev().take(4).rev().cloned());
    }
    summary.truncate(MAX_SUMMARY_LINES);
    summary
}

/// 安装器输出是否表明失败来自库下载（可凭已落盘文件重试恢复）
fn installer_output_suggests_download_failure(tail: &[String]) -> bool {
    tail.iter().any(|line| {
        line.contains("failed to download")
            || line.contains("ConnectException")
            || line.contains("SocketException")
            || line.contains("SocketTimeoutException")
            || line.contains("UnknownHostException")
    })
}

/// 根据下载源决定安装器内嵌库地址应改写到的 Maven 根地址
///
/// 默认源优先官方 Maven；官方不可达时退回 BMCLAPI。否则安装器会绕过启动器
/// 已有的镜像回退逻辑，直接连接被阻断的官方地址并以非零状态退出。
fn neoforge_replace_source(source: &DownloadSource, official_reachable: bool) -> String {
    match source {
        DownloadSource::BMCLAPI => NEOFORGE_MIRROR_MAVEN.to_owned(),
        DownloadSource::Custom(base) => {
            format!("{}/maven", base.as_str().trim_end_matches('/'))
        }
        DownloadSource::Default if !official_reachable => NEOFORGE_MIRROR_MAVEN.to_owned(),
        DownloadSource::Default => NEOFORGE_OFFICIAL_MAVEN.to_owned(),
    }
}

/// 探测 NeoForge 官方 Maven 是否可达
///
/// 只请求仓库必然存在的 `maven-metadata.xml`，不读取响应体；连接被拒绝
/// （官方 Maven 在国内网络常见的失败形态）会在 `call()` 时直接报错。
fn neoforge_official_maven_reachable() -> bool {
    let probe = format!("{NEOFORGE_OFFICIAL_MAVEN}/net/neoforged/neoforge/maven-metadata.xml");
    match crate::http::get(&probe)
        .timeout(Duration::from_secs(10))
        .call()
    {
        Ok(response) => response.is_success(),
        Err(_) => false,
    }
}

/// NeoForge 模组加载器的安装特质
///
/// 可以通过引入本特质和使用 [`crate::download::Downloader`] 来安装模组加载器
pub trait NeoForgeDownloadExt: Sync {
    /// 根据纯净版本号获取当前可用的所有 NeoForge 版本
    fn get_avaliable_installers(&self, vanilla_version: &str) -> DynResult<NeoForgeVersionsData>;
    /// 运行安装 NeoForge 模组加载器的预安装步骤
    ///
    /// 一般是下载各种库和依赖
    fn install_neoforge_pre(
        &self,
        version_id: &str,
        vanilla_version: &str,
        neoforge_version: &str,
    ) -> DynResult;
    /// 运行安装 NeoForge 模组加载器的后安装步骤
    ///
    /// 通常是修改安装器信息，然后执行安装器完成最后的安装步骤
    fn install_neoforge_post(
        &self,
        version_name: &str,
        version_id: &str,
        neoforge_version: &str,
    ) -> DynResult;
    /// 将安装器的部分信息进行修改，如版本名称，下载源等
    fn modify_neoforge_installer(
        &self,
        from_reader: fs_err::File,
        to_writer: fs_err::File,
        name: &str,
    ) -> DynResult;
}

impl<R: Reporter> NeoForgeDownloadExt for Downloader<R> {
    fn get_avaliable_installers(&self, vanilla_version: &str) -> DynResult<NeoForgeVersionsData> {
        let mut versions_data =
            crate::http::retry_get_json::<Vec<NeoForgeItemInfo>>(match self.source {
                DownloadSource::BMCLAPI => {
                    format!("https://bmclapi2.bangbang93.com/neoforge/list/{vanilla_version}")
                }
                _ => format!("https://bmclapi2.bangbang93.com/neoforge/list/{vanilla_version}"),
            })
            .map_err(|e| {
                anyhow::anyhow!(
                    "下载 NeoForge {} 安装器版本元数据失败：{:?}",
                    vanilla_version,
                    e
                )
            })?;

        versions_data.retain(|x| x.raw_version.starts_with("neoforge")); // 只保留 NeoForge 的包
        versions_data.iter_mut().for_each(|x| {
            if let Some(s) = x.version.strip_prefix("neoforge-") {
                x.version = s.to_owned();
            }
        });
        versions_data.reverse(); // 调转顺序，从最新的开始
        let latest_version = versions_data.first().cloned();

        Ok(NeoForgeVersionsData {
            recommended: None,
            latest: latest_version,
            all_versions: versions_data,
        })
    }

    fn install_neoforge_pre(
        &self,
        version_id: &str,
        vanilla_version: &str,
        neoforge_version: &str,
    ) -> DynResult {
        let r = self.reporter.fork();

        let full_path = format!(
            "{root}/net/neoforged/neoforge/{mc}-{neoforge}/neoforge-{mc}-{neoforge}-installer.jar",
            root = self.minecraft_library_path.as_str(),
            mc = vanilla_version,
            neoforge = neoforge_version
        );
        tracing::trace!("Downloading NeoForge Installer {full_path}");
        if std::path::Path::new(&full_path).is_file() {
            return Ok(());
        }
        fs_err::create_dir_all(&full_path[..full_path.rfind('/').unwrap_or(full_path.len())])?;

        r.set_message(format!("下载 NeoForge 安装器 {neoforge_version}"));
        r.add_max_progress(1.);

        let installer_path = format!(
            "net/neoforged/neoforge/{neoforge_version}/neoforge-{neoforge_version}-installer.jar"
        );
        let uris = [
            match self.source {
                DownloadSource::BMCLAPI => format!("{NEOFORGE_MIRROR_MAVEN}/{installer_path}"),
                _ => format!("{NEOFORGE_OFFICIAL_MAVEN}/{installer_path}"),
            },
            format!("{NEOFORGE_MIRROR_MAVEN}/{installer_path}"),
            format!("{NEOFORGE_OFFICIAL_MAVEN}/{installer_path}"),
        ];

        crate::http::download(&uris, &full_path, 0).map_err(|e| {
            anyhow::anyhow!(
                "下载 NeoForge {}-{} 安装器失败：{:?}",
                version_id,
                neoforge_version,
                e
            )
        })?;

        r.add_progress(1.);
        Ok(())
    }

    fn install_neoforge_post(
        &self,
        version_name: &str,
        version_id: &str,
        neoforge_version: &str,
    ) -> DynResult {
        let r = self.reporter.fork();
        // 新版本均使用安装器安装
        let full_path = format!(
            "{root}/net/neoforged/neoforge/{mc}-{neoforge}/neoforge-{mc}-{neoforge}-installer.jar",
            root = self.minecraft_library_path.as_str(),
            mc = version_id,
            neoforge = neoforge_version
        );
        let tmp_full_path = format!(
            "{root}/net/neoforged/neoforge/{mc}-{neoforge}/neoforge-{mc}-{neoforge}-installer.tmp.{tempid}.jar",
            root = self.minecraft_library_path.as_str(),
            mc = version_id,
            neoforge = neoforge_version,
            tempid = std::time::SystemTime::now()
                .elapsed()
                .unwrap_or_default()
                .as_secs()
        );
        tracing::trace!("Writing temp neoforge installer from {full_path} to {tmp_full_path}");
        {
            let from_file = fs_err::OpenOptions::new().read(true).open(&full_path)?;
            let to_file = fs_err::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_full_path)?;
            tracing::trace!("Modifying");
            self.modify_neoforge_installer(from_file, to_file, version_name)
                .with_context(|| {
                    format!(
                        "修改 NeoForge 模组安装器文件 {full_path} 到 {tmp_full_path} 时发生错误"
                    )
                })?;
        }

        r.add_max_progress(2.);
        r.set_message("正在修改安装器参数".into());

        #[cfg(not(windows))]
        let mut cmd = std::process::Command::new(&self.java_path);
        #[cfg(windows)]
        let mut cmd = {
            use std::os::windows::process::CommandExt;
            let mut cmd = std::process::Command::new(&self.java_path);
            cmd.creation_flags(0x08000000);
            cmd
        };

        cmd.stdout(Stdio::piped());
        // GUI 启动时没有可读控制台；安装器的异常栈只写到 stdout/stderr，
        // 必须主动捕获，失败时才能把可定位的尾部输出带进错误信息。
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        // 使用安装器自身声明的入口类执行无头安装，避免依赖内置 bootstrapper
        // 反射调用固定签名的 main 方法。
        let main_class = super::installer_main_class(&full_path)?;

        cmd.arg("-cp");
        cmd.arg(&tmp_full_path);
        cmd.arg(main_class);
        cmd.arg("--installClient");
        cmd.arg(self.minecraft_path.as_str()); // 安装位置

        r.add_progress(1.);
        r.set_message("运行 NeoForge 安装器安装 NeoForge".into());

        let mut attempts = 0;
        let (status, output_tail) = loop {
            attempts += 1;
            if attempts > 1 {
                r.set_message("NeoForge 安装器下载失败，正在重试".into());
            }
            tracing::trace!("Start running installer {cmd:?}");

            let mut child = cmd.spawn()?;

            // stdout 在主线程读取并驱动进度，stderr 由独立线程读取，避免任一管道
            // 写满后安装器阻塞、双方都在等对方清空管道。
            let output_tail = Arc::new(Mutex::new(VecDeque::new()));
            let stderr_tail = Arc::clone(&output_tail);
            let stderr_reader = child.stderr.take().map(|stderr| {
                std::thread::spawn(move || {
                    let mut stderr = std::io::BufReader::new(stderr);
                    let mut buf = String::with_capacity(256);
                    loop {
                        match stderr.read_line(&mut buf) {
                            Ok(0) => break,
                            Ok(len) => {
                                let line = buf[..len].trim();
                                tracing::trace!("[FIB stderr] {line}");
                                if let Ok(mut tail) = stderr_tail.lock() {
                                    push_installer_output(&mut tail, line);
                                }
                                buf.clear();
                            }
                            Err(_) => break,
                        }
                    }
                })
            });

            let ir = r.fork();
            ir.set_message("这需要一点时间……".into());
            let pr = r.fork();

            let mut delay_timer = Instant::now();

            if let Some(stdout) = child.stdout.take() {
                let mut stdout = std::io::BufReader::new(stdout);
                let mut buf = String::with_capacity(256);
                loop {
                    match stdout.read_line(&mut buf) {
                        Ok(0) => break,
                        Ok(len) => {
                            let line = buf[..len].trim();

                            if let Ok(mut tail) = output_tail.lock() {
                                push_installer_output(&mut tail, line);
                            }

                            let delayed = delay_timer.elapsed() > Duration::from_millis(16);

                            if line.starts_with("Patching ") {
                                // 数量太多可以缓一缓
                                if delayed {
                                    pr.set_message(line.to_owned());
                                }
                            } else if delayed {
                                pr.set_message(line.to_owned());
                            }
                            tracing::trace!("[FIB] {line}");

                            if let Some(class_name) = line.strip_prefix("Patching ") {
                                // 数量太多可以缓一缓
                                if delayed {
                                    ir.set_message(format!("正在修补类 {class_name}"));
                                }
                            } else if let Some(url) = line.strip_prefix("Downloading library from ")
                            {
                                ir.set_message(format!("正在下载依赖 {url}"));
                            } else if let Some(url) = line.strip_prefix("Following redirect: ") {
                                ir.set_message(format!("下载重定向至 {url}"));
                            } else if let Some(class_name) = line.strip_prefix("Reading patch ") {
                                ir.set_message(format!("正在读取修补信息 {class_name}"));
                            } else if line == "Task: DOWNLOAD_MOJMAPS" {
                                ir.set_message("正在下载源码对照表".into());
                            } else if line == "Task: MERGE_MAPPING" {
                                ir.set_message("正在合并源码对照表".into());
                            } else if line == "Injecting profile" {
                                ir.set_message("正在注入版本元数据".into());
                            }

                            if delayed {
                                delay_timer = Instant::now();
                            }

                            buf.clear()
                        }
                        Err(_) => break,
                    }
                }
            }

            drop(ir);
            drop(pr);

            let status = child.wait()?;
            if let Some(reader) = stderr_reader {
                let _ = reader.join();
            }
            let output_tail = output_tail
                .lock()
                .map(|tail| tail.iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default();

            if status.success() {
                break (status, output_tail);
            }
            if !output_tail.is_empty() {
                // 完整尾部输出进日志；弹窗只带摘要，避免把整段异常栈糊到用户脸上。
                tracing::error!(
                    "NeoForge {}-{} 安装器第 {} 次执行失败（末尾 {} 行输出）：\n{}",
                    version_id,
                    neoforge_version,
                    attempts,
                    output_tail.len(),
                    output_tail.join("\n")
                );
            }
            // 镜像节点偶发 connection reset/timeout 时，安装器会把已下载成功的
            // 文件留在库里；再执行一次即可复用它们。只对下载类失败重试一次。
            if attempts == 1 && installer_output_suggests_download_failure(&output_tail) {
                tracing::warn!(
                    "NeoForge {}-{} 安装器因库下载失败退出，使用已落盘的库重试一次",
                    version_id,
                    neoforge_version
                );
                continue;
            }
            break (status, output_tail);
        };

        r.add_progress(1.);
        r.remove_progress();
        fs_err::remove_file(tmp_full_path)?;
        // 安装器返回 0 只代表进程正常退出；是否装好要看产物是否真的落盘。
        let installed_version_json = std::path::Path::new(&self.minecraft_version_path)
            .join(version_name)
            .join(format!("{version_name}.json"));
        if !status.success() {
            let summary = installer_failure_summary(&output_tail);
            let detail = if summary.is_empty() {
                String::new()
            } else {
                format!("\n安装器输出（末尾摘要）：\n{}", summary.join("\n"))
            };
            anyhow::bail!(
                "执行 NeoForge {}-{} 安装器失败，运行器返回值：{}{}",
                version_id,
                neoforge_version,
                status,
                detail
            );
        }
        if !installed_version_json.is_file() {
            anyhow::bail!(
                "NeoForge {}-{} 安装器已正常退出（运行器返回值：{}），但未找到版本元数据 {}\n\
                 这通常表示安装器写入了不同的版本名称或目录（NeoForge 与 Forge 的产物布局可能不同），\
                 请检查 versions 目录下的实际产物",
                version_id,
                neoforge_version,
                status,
                installed_version_json.display()
            );
        }
        Ok(())
    }

    fn modify_neoforge_installer(
        &self,
        from_reader: fs_err::File,
        to_writer: fs_err::File,
        name: &str,
    ) -> DynResult {
        tracing::trace!("Opening file");
        let mut file = zip::ZipArchive::new(std::io::BufReader::new(from_reader))
            .context("打开 NeoForge 安装器时发生错误")?;
        tracing::trace!("Opening file");
        let mut out_file = zip::ZipWriter::new(to_writer);
        // 安装器从 install_profile.json 下载处理器依赖，从 version.json 下载游戏库；
        // 两处都要改写，否则真实安装会只完成一半。
        let official_reachable = match self.source {
            DownloadSource::Default => neoforge_official_maven_reachable(),
            _ => false,
        };
        let replace_source = neoforge_replace_source(&self.source, official_reachable);
        if matches!(self.source, DownloadSource::Default) && !official_reachable {
            tracing::warn!(
                "官方 NeoForge Maven {} 不可达，安装器内嵌库地址已改写到镜像 {}",
                NEOFORGE_OFFICIAL_MAVEN,
                NEOFORGE_MIRROR_MAVEN
            );
        }
        tracing::trace!("Reading file");
        for index in 0..file.len() {
            if let Ok(mut entry) = file.by_index(index) {
                // 保留清单：SimpleInstaller 会读取自身 JAR 的 Manifest，
                // 缺失时会直接抛出 NullPointerException。
                if entry.name().starts_with("META-INF") && entry.name() != "META-INF/MANIFEST.MF" {
                    continue;
                }
                if entry.is_file() {
                    // tracing::trace!("Writting file {}", entry.name());
                    match entry.name() {
                        "install_profile.json" => {
                            let mut data = String::with_capacity(entry.size() as usize);
                            entry.read_to_string(&mut data)?;
                            let mut install_profile: Value = serde_json::from_str(&data)?;
                            if let Value::Object(obj) = &mut install_profile {
                                if let Some(Value::String(version)) = obj.get_mut("version") {
                                    *version = name.to_owned();
                                    tracing::trace!("已修改 version 字段为 {version}");
                                }
                                if let Some(Value::Object(obj)) = obj.get_mut("install") {
                                    if let Some(Value::String(target)) = obj.get_mut("target") {
                                        *target = name.to_owned();
                                        tracing::trace!("已修改 install.target 字段为 {target}");
                                    }
                                }
                                if let Some(Value::Array(array)) = obj.get_mut("libraries") {
                                    for (i, lib) in array.iter_mut().enumerate() {
                                        if let Value::Object(obj) = lib {
                                            obj.remove("serverreq");
                                            obj.insert("clientreq".into(), Value::Bool(true));
                                            if let Some(Value::Object(obj)) =
                                                obj.get_mut("downloads")
                                            {
                                                if let Some(Value::Object(obj)) =
                                                    obj.get_mut("artifact")
                                                {
                                                    if let Some(Value::String(down_url)) =
                                                        obj.get_mut("url")
                                                    {
                                                        if let Some(down_path) = down_url
                                                            .strip_prefix(NEOFORGE_OFFICIAL_MAVEN)
                                                        {
                                                            *down_url = format!(
                                                                "{replace_source}{down_path}"
                                                            );
                                                            tracing::trace!(
                                                                "已修改 libraries[{i}].download.artifact.url 字段"
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            if let Some(Value::String(down_url)) =
                                                obj.get_mut("url")
                                            {
                                                if let Some(down_path) = down_url.strip_prefix(
                                                    "https://maven.neoforged.net/releases",
                                                ) {
                                                    *down_url =
                                                        format!("{replace_source}{down_path}");
                                                    tracing::trace!(
                                                        "已修改 libraries[{i}].url 字段"
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                                // 1.12.2 之前的镜像源
                                if let Some(Value::Object(obj)) = obj.get_mut("versionInfo") {
                                    if let Some(Value::Array(array)) = obj.get_mut("libraries") {
                                        for (i, lib) in array.iter_mut().enumerate() {
                                            if let Value::Object(obj) = lib {
                                                obj.remove("serverreq");
                                                obj.insert("clientreq".into(), Value::Bool(true));
                                                if let Some(Value::Object(obj)) =
                                                    obj.get_mut("downloads")
                                                {
                                                    if let Some(Value::Object(obj)) =
                                                        obj.get_mut("artifact")
                                                    {
                                                        if let Some(Value::String(down_url)) =
                                                            obj.get_mut("url")
                                                        {
                                                            if let Some(down_path) = down_url
                                                                .strip_prefix(
                                                                    NEOFORGE_OFFICIAL_MAVEN,
                                                                )
                                                            {
                                                                *down_url = format!(
                                                                    "{replace_source}{down_path}"
                                                                );
                                                                tracing::trace!(
                                                                    "已修改 libraries[{i}].download.artifact.url 字段"
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                                if let Some(Value::String(down_url)) =
                                                    obj.get_mut("url")
                                                {
                                                    if let Some(down_path) = down_url
                                                        .strip_prefix(NEOFORGE_OFFICIAL_MAVEN)
                                                    {
                                                        *down_url =
                                                            format!("{replace_source}{down_path}");
                                                        tracing::trace!("已修改 versionInfo.libraries[{i}].url 字段");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            #[cfg(debug_assertions)]
                            tracing::trace!(
                                "修改完毕：\n{}",
                                serde_json::to_string_pretty(&install_profile)?
                            );
                            let output = serde_json::to_vec_pretty(&install_profile)?;
                            out_file.start_file(
                                entry.name(),
                                zip::write::SimpleFileOptions::default(),
                            )?;
                            out_file.write_all(&output)?;
                        }
                        "version.json" => {
                            let mut data = String::with_capacity(entry.size() as usize);
                            entry.read_to_string(&mut data)?;
                            let mut version_json: Value = serde_json::from_str(&data)?;
                            if let Value::Object(obj) = &mut version_json {
                                if let Some(Value::String(id)) = obj.get_mut("id") {
                                    *id = name.to_owned();
                                }
                            }
                            super::rewrite_library_urls(
                                &mut version_json,
                                &[NEOFORGE_OFFICIAL_MAVEN],
                                &replace_source,
                            );
                            out_file.start_file(
                                entry.name(),
                                zip::write::SimpleFileOptions::default(),
                            )?;
                            out_file.write_all(&serde_json::to_vec_pretty(&version_json)?)?;
                        }
                        _ => {
                            // tracing::trace!("Copied file {}", entry.name());
                            out_file.raw_copy_file(entry)?
                        }
                    }
                } else if entry.is_dir() {
                    // tracing::trace!("Added dir {}", entry.name());
                    out_file
                        .add_directory(entry.name(), zip::write::SimpleFileOptions::default())?;
                }
            }
        }
        let mut to_writer = out_file.finish()?;
        let _ = to_writer.flush();
        let _ = to_writer.sync_all();
        // std::io::copy(&mut from_reader, &mut to_writer)?;
        // to_writer.sync_all()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use super::*;

    #[test]
    fn installer_output_tail_is_bounded() {
        let mut tail = VecDeque::new();
        for index in 0..(INSTALLER_OUTPUT_TAIL_LINES + 5) {
            push_installer_output(&mut tail, &format!("line {index}"));
        }
        assert_eq!(tail.len(), INSTALLER_OUTPUT_TAIL_LINES);
        assert_eq!(tail.front().map(String::as_str), Some("line 5"));

        push_installer_output(&mut tail, &"x".repeat(INSTALLER_OUTPUT_LINE_CHARS + 50));
        assert_eq!(
            tail.back().map(|line| line.chars().count()),
            Some(INSTALLER_OUTPUT_LINE_CHARS)
        );
    }

    #[test]
    fn installer_failure_summary_keeps_cause_and_failed_libraries() {
        let tail: Vec<String> = [
            "Considering library net.neoforged.fancymodloader:earlydisplay:4.0.45",
            "java.net.ConnectException: Connection refused",
            "\tat java.base/sun.nio.ch.Net.pollConnect(Native Method)",
            "java.net.ConnectException: Connection refused",
            "These libraries failed to download. Try again.",
            "net.neoforged.fancymodloader:earlydisplay:4.0.45",
            "net.neoforged.fancymodloader:loader:4.0.45",
            "net.fabricmc:sponge-mixin:0.16.4+mixin.0.8.7",
            "net.neoforged:neoforge:21.1.265:universal",
            "There was an error during installation",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();

        let summary = installer_failure_summary(&tail);
        assert_eq!(summary[0], "java.net.ConnectException: Connection refused");
        assert_eq!(
            summary
                .iter()
                .filter(|line| line.contains("Connection refused"))
                .count(),
            1,
            "重复的异常行应去重"
        );
        assert!(
            summary
                .iter()
                .any(|line| line == "net.neoforged:neoforge:21.1.265:universal"),
            "应带出下载失败的库坐标"
        );
        assert!(
            !summary.iter().any(|line| line.contains("sun.nio.ch")),
            "不应把栈帧塞进摘要"
        );
        assert!(summary.len() <= 9);
    }

    #[test]
    fn installer_failure_summary_falls_back_to_tail() {
        let tail: Vec<String> = ["a", "b", "c", "d", "e"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let summary = installer_failure_summary(&tail);
        assert_eq!(
            summary,
            ["b", "c", "d", "e"]
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn retry_only_for_download_failures() {
        let download_failure = [
            "java.net.SocketException: Connection reset".to_owned(),
            "These libraries failed to download. Try again.".to_owned(),
        ];
        assert!(installer_output_suggests_download_failure(
            &download_failure
        ));

        let argument_failure = ["Invalid argument: --installClient".to_owned()];
        assert!(!installer_output_suggests_download_failure(
            &argument_failure
        ));
    }

    #[test]
    fn replace_source_falls_back_to_mirror_when_official_unreachable() {
        assert_eq!(
            neoforge_replace_source(&DownloadSource::Default, true),
            NEOFORGE_OFFICIAL_MAVEN
        );
        assert_eq!(
            neoforge_replace_source(&DownloadSource::Default, false),
            NEOFORGE_MIRROR_MAVEN
        );
        assert_eq!(
            neoforge_replace_source(&DownloadSource::BMCLAPI, true),
            NEOFORGE_MIRROR_MAVEN
        );
        let custom = DownloadSource::Custom(
            "https://mirror.example/base/"
                .parse()
                .expect("解析自定义源"),
        );
        assert_eq!(
            neoforge_replace_source(&custom, false),
            "https://mirror.example/base/maven"
        );
    }

    #[test]
    fn modify_installer_rewrites_library_urls() {
        let dir = std::env::temp_dir().join(format!("scl_neoforge_modify_{}", std::process::id()));
        let _ = fs_err::remove_dir_all(&dir);
        fs_err::create_dir_all(&dir).expect("创建临时目录");
        let src = dir.join("installer.jar");
        let dst = dir.join("installer.tmp.jar");
        {
            let file = fs_err::File::create(&src).expect("创建安装器");
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("META-INF/MANIFEST.MF", options)
                .expect("写入清单");
            zip.write_all(b"Main-Class: net.minecraftforge.installer.SimpleInstaller\n")
                .expect("写入清单内容");
            zip.start_file("META-INF/SIGNATURE.RSA", options)
                .expect("写入签名");
            zip.write_all(b"signature").expect("写入签名内容");
            zip.start_file("install_profile.json", options)
                .expect("写入 install_profile");
            zip.write_all(
                br#"{
                    "version": "old",
                    "install": {"target": "old"},
                    "libraries": [
                        {"name": "a", "url": "https://maven.neoforged.net/releases/net/a.jar",
                         "downloads": {"artifact": {"url": "https://maven.neoforged.net/releases/net/a.jar"}}},
                        {"name": "b", "url": "https://libraries.minecraft.net/net/b.jar"}
                    ],
                    "versionInfo": {"libraries": [
                        {"name": "c", "url": "https://maven.neoforged.net/releases/net/c.jar"}
                    ]}
                }"#,
            )
            .expect("写入 install_profile 内容");
            zip.start_file("version.json", options)
                .expect("写入 version.json");
            zip.write_all(
                br#"{
                    "id": "old",
                    "libraries": [
                        {"name": "d", "url": "https://maven.neoforged.net/releases/net/d.jar",
                         "downloads": {"artifact": {"url": "https://maven.neoforged.net/releases/net/d.jar"}}},
                        {"name": "e", "downloads": {"artifact": {"url": "https://libraries.minecraft.net/net/e.jar"}}}
                    ]
                }"#,
            )
            .expect("写入 version.json 内容");
            zip.finish().expect("完成安装器");
        }

        let downloader: Downloader<()> = Downloader::default().with_source(DownloadSource::BMCLAPI);
        let from = fs_err::OpenOptions::new()
            .read(true)
            .open(&src)
            .expect("打开源安装器");
        let to = fs_err::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&dst)
            .expect("创建目标安装器");
        downloader
            .modify_neoforge_installer(from, to, "new-name")
            .expect("改写安装器");

        let file = fs_err::File::open(&dst).expect("打开改写后的安装器");
        let mut zip = zip::ZipArchive::new(file).expect("解析改写后的安装器");
        assert!(
            zip.by_name("META-INF/MANIFEST.MF").is_ok(),
            "安装器读取自身清单，必须保留"
        );
        assert!(
            zip.by_name("META-INF/SIGNATURE.RSA").is_err(),
            "改写后签名失效，应丢弃其它 META-INF 文件"
        );

        let mut profile = String::new();
        zip.by_name("install_profile.json")
            .expect("读取 install_profile")
            .read_to_string(&mut profile)
            .expect("读取 install_profile 内容");
        let profile: Value = serde_json::from_str(&profile).expect("解析 install_profile");
        assert_eq!(profile["version"], "new-name");
        assert_eq!(profile["install"]["target"], "new-name");
        assert_eq!(
            profile["libraries"][0]["downloads"]["artifact"]["url"],
            format!("{NEOFORGE_MIRROR_MAVEN}/net/a.jar")
        );
        assert_eq!(
            profile["libraries"][0]["url"],
            format!("{NEOFORGE_MIRROR_MAVEN}/net/a.jar")
        );
        assert_eq!(
            profile["libraries"][1]["url"], "https://libraries.minecraft.net/net/b.jar",
            "非 NeoForge Maven 的地址不应改写"
        );
        assert_eq!(
            profile["versionInfo"]["libraries"][0]["url"],
            format!("{NEOFORGE_MIRROR_MAVEN}/net/c.jar")
        );

        let mut version = String::new();
        zip.by_name("version.json")
            .expect("读取 version.json")
            .read_to_string(&mut version)
            .expect("读取 version.json 内容");
        let version: Value = serde_json::from_str(&version).expect("解析 version.json");
        assert_eq!(version["id"], "new-name");
        assert_eq!(
            version["libraries"][0]["url"],
            format!("{NEOFORGE_MIRROR_MAVEN}/net/d.jar")
        );
        assert_eq!(
            version["libraries"][0]["downloads"]["artifact"]["url"],
            format!("{NEOFORGE_MIRROR_MAVEN}/net/d.jar")
        );
        assert_eq!(
            version["libraries"][1]["downloads"]["artifact"]["url"],
            "https://libraries.minecraft.net/net/e.jar"
        );

        let _ = fs_err::remove_dir_all(&dir);
    }
}
