//! Forge 模组加载器的下载模块
use std::{
    io::{BufRead, Read, Write},
    process::Stdio,
    time::{Duration, Instant},
};

use anyhow::Context;
use serde_json::Value;

use super::{structs::ForgeVersionsData, Downloader};
use crate::{
    download::{
        structs::{ForgeItemInfo, ForgePromoItem},
        DownloadSource,
    },
    prelude::*,
};

/// Forge 模组加载器的安装特质
///
/// 可以通过引入本特质和使用 [`crate::download::Downloader`] 来安装模组加载器
pub trait ForgeDownloadExt: Sync {
    /// 根据纯净版本号获取当前可用的所有 Forge 版本
    fn get_avaliable_installers(&self, vanilla_version: &str) -> DynResult<ForgeVersionsData>;
    /// 运行安装 Forge 模组加载器的预安装步骤
    ///
    /// 一般是下载各种库和依赖
    fn install_forge_pre(
        &self,
        version_id: &str,
        vanilla_version: &str,
        forge_version: &str,
    ) -> DynResult;
    /// 运行安装 Forge 模组加载器的后安装步骤
    ///
    /// 通常是修改安装器信息，然后执行安装器完成最后的安装步骤
    fn install_forge_post(
        &self,
        version_name: &str,
        version_id: &str,
        forge_version: &str,
    ) -> DynResult;
    /// 将安装器的部分信息进行修改，如版本名称，下载源等
    fn modify_forge_installer(
        &self,
        from_reader: fs_err::File,
        to_writer: fs_err::File,
        name: &str,
    ) -> DynResult;
}

impl<R: Reporter> ForgeDownloadExt for Downloader<R> {
    fn get_avaliable_installers(&self, vanilla_version: &str) -> DynResult<ForgeVersionsData> {
        let (versions_data, version_promo) = rayon::join(
            || {
                crate::http::retry_get(match self.source {
                    DownloadSource::BMCLAPI => {
                        format!("https://bmclapi2.bangbang93.com/forge/minecraft/{vanilla_version}")
                    }
                    _ => {
                        format!("https://bmclapi2.bangbang93.com/forge/minecraft/{vanilla_version}")
                    }
                })
            },
            || {
                crate::http::retry_get(match self.source {
                    DownloadSource::BMCLAPI => "https://bmclapi2.bangbang93.com/forge/promos",
                    _ => "https://bmclapi2.bangbang93.com/forge/promos",
                })
            },
        );
        let versions_data = versions_data.map_err(|e| {
            anyhow::anyhow!(
                "下载 Forge {} 安装器版本元数据失败：{:?}",
                vanilla_version,
                e
            )
        })?;
        let version_promo = version_promo.map_err(|e| {
            anyhow::anyhow!(
                "下载 Forge {} 安装器版本元数据失败：{:?}",
                vanilla_version,
                e
            )
        })?;
        let (version_promo, info) = rayon::join(
            || -> DynResult<Vec<ForgePromoItem>> { version_promo.into_json() },
            || -> DynResult<Vec<ForgeItemInfo>> { versions_data.into_json() },
        );
        let version_promo = version_promo.map_err(|e| anyhow::anyhow!(e))?;
        let mut info: Vec<ForgeItemInfo> = info.map_err(|e| anyhow::anyhow!(e))?;

        info.sort_by(|a, b| {
            let a: u64 = {
                let mut r = 0;
                for (i, x) in a
                    .version
                    .split('.')
                    .map(|x| str::parse::<u16>(x).unwrap_or_default())
                    .enumerate()
                {
                    r |= (x as u64) << (64 - (i + 1) * 16);
                }
                r
            };
            let b: u64 = {
                let mut r = 0;
                for (i, x) in b
                    .version
                    .split('.')
                    .map(|x| str::parse::<u16>(x).unwrap_or_default())
                    .enumerate()
                {
                    r |= (x as u64) << (64 - (i + 1) * 16);
                }
                r
            };
            a.cmp(&b)
        });

        let recommended_version = version_promo
            .iter()
            .find(|a| a.name == format!("{vanilla_version}-recommended"))
            .and_then(|a| a.build.to_owned());
        let latest_version = version_promo
            .iter()
            .find(|a| a.name == format!("{vanilla_version}-latest"))
            .and_then(|a| a.build.to_owned());

        info.reverse(); // 调转顺序，从最新的开始
        Ok(ForgeVersionsData {
            recommended: recommended_version,
            latest: latest_version,
            all_versions: info,
        })
    }

    fn install_forge_pre(
        &self,
        version_id: &str,
        vanilla_version: &str,
        forge_version: &str,
    ) -> DynResult {
        let r = self.reporter.fork();

        if vanilla_version
            .parse::<crate::semver::MinecraftVersion>()
            .map(|x| x.should_forge_use_override_installiation())
            .unwrap_or(false)
        {
            // 旧版本使用覆盖方式安装，下载其覆盖用 zip 压缩文件
            // should_forge_use_client_or_universal
            // 旧版本均使用 4 组版本号编号，所以不需要特判

            let client_or_universal = vanilla_version
                .parse::<crate::semver::MinecraftVersion>()
                .map(|x| x.should_forge_use_client_or_universal())
                .unwrap_or(false);

            let suffix = if client_or_universal {
                "client" // https://maven.minecraftforge.net/net/minecraftforge/forge/1.2.5-3.4.9.171/forge-1.2.5-3.4.9.171-client.zip
            } else {
                "universal" // https://maven.minecraftforge.net/net/minecraftforge/forge/1.3.2-4.3.5.318/forge-1.3.2-4.3.5.318-universal.zip
            };

            let full_path = format!(
                "{root}/net/minecraftforge/forge/{mc}-{forge}/forge-{mc}-{forge}-{suffix}.zip",
                root = self.minecraft_library_path.as_str(),
                mc = vanilla_version,
                forge = forge_version,
                suffix = suffix,
            );

            if std::path::Path::new(&full_path).is_file() {
                return Ok(());
            }
            fs_err::create_dir_all(&full_path[..full_path.rfind('/').unwrap_or(full_path.len())])?;

            r.set_message(format!("下载 Forge 安装覆盖包 {forge_version}"));
            r.add_max_progress(1.);

            let uris = [
                match self.source {
                    DownloadSource::Default => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip"),
                    DownloadSource::BMCLAPI => format!("https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip"),
                    _ => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip")
                },
                format!("https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip"),
                format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip"),
            ];

            crate::http::download(&uris, &full_path, 0).map_err(|e| {
                anyhow::anyhow!(
                    "下载 Forge {}-{} 安装覆盖包失败：{:?}",
                    version_id,
                    forge_version,
                    e
                )
            })?;
        } else {
            let full_path = format!(
                "{root}/net/minecraftforge/forge/{mc}-{forge}/forge-{mc}-{forge}-installer.jar",
                root = self.minecraft_library_path.as_str(),
                mc = vanilla_version,
                forge = forge_version
            );
            tracing::trace!("Downloading Forge Installer {full_path}");
            if std::path::Path::new(&full_path).is_file() {
                return Ok(());
            }
            fs_err::create_dir_all(&full_path[..full_path.rfind('/').unwrap_or(full_path.len())])?;

            r.set_message(format!("下载 Forge 安装器 {forge_version}"));
            r.add_max_progress(1.);

            let build_id =
                &forge_version[forge_version.rfind('.').map(|x| x + 1).unwrap_or_default()..];

            let uris = if forge_version.split('.').count() == 3 {
                [
                    match self.source {
                        DownloadSource::Default => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                        DownloadSource::BMCLAPI => format!("https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                        _ => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar")
                    },
                    format!("https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                    format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                ]
            } else {
                [
                    match self.source {
                        DownloadSource::Default => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                        DownloadSource::BMCLAPI => format!("https://bmclapi2.bangbang93.com/forge/download/{build_id}"),
                        _ => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar")
                    },
                    format!("https://bmclapi2.bangbang93.com/forge/download/{build_id}"),
                    format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                ]
            };

            crate::http::download(&uris, &full_path, 0).map_err(|e| {
                anyhow::anyhow!(
                    "下载 Forge {}-{} 安装器失败：{:?}",
                    version_id,
                    forge_version,
                    e
                )
            })?;
        }

        r.add_progress(1.);
        Ok(())
    }

    fn install_forge_post(
        &self,
        version_name: &str,
        version_id: &str,
        forge_version: &str,
    ) -> DynResult {
        let r = self.reporter.fork();

        // 1.5.2 以前的版本都使用覆盖 JAR 的方式安装 Forge
        if version_id
            .parse::<crate::semver::MinecraftVersion>()
            .map(|x| x.should_forge_use_override_installiation())
            .unwrap_or(false)
        {
            let client_or_universal = version_id
                .parse::<crate::semver::MinecraftVersion>()
                .map(|x| x.should_forge_use_client_or_universal())
                .unwrap_or(false);

            let suffix = if client_or_universal {
                "client"
            } else {
                "universal"
            };

            let full_path = format!(
                "{root}/net/minecraftforge/forge/{mc}-{forge}/forge-{mc}-{forge}-{suffix}.zip",
                root = self.minecraft_library_path.as_str(),
                mc = version_id,
                forge = forge_version,
                suffix = suffix,
            );

            let full_override_path = format!(
                "{root}/{version_name}/{version_name}.jar",
                root = self.minecraft_version_path.as_str(),
                version_name = version_name,
            );

            let full_temp_override_path = format!(
                "{root}/{version_name}/{version_name}.tmp.jar",
                root = self.minecraft_version_path.as_str(),
                version_name = version_name,
            );

            let full_file = fs_err::OpenOptions::new().read(true).open(&full_path)?;
            let mut full_file = zip::ZipArchive::new(full_file)?;

            let override_file = fs_err::OpenOptions::new()
                .read(true)
                .open(&full_override_path)?;
            let mut override_file = zip::ZipArchive::new(override_file)?;

            let temp_override_file = fs_err::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&full_temp_override_path)?;
            let mut temp_override_file = zip::ZipWriter::new(temp_override_file);

            for index in 0..override_file.len() {
                if let Ok(entry) = override_file.by_index(index) {
                    if entry.name().starts_with("META-INF") {
                        continue;
                    } else if entry.is_file() {
                        temp_override_file.raw_copy_file(entry)?;
                    } else if entry.is_dir() {
                        // tracing::trace!("Added dir {}", entry.name());
                        temp_override_file.add_directory(
                            entry.name(),
                            zip::write::SimpleFileOptions::default(),
                        )?;
                    }
                }
            }

            for index in 0..full_file.len() {
                if let Ok(entry) = full_file.by_index(index) {
                    if entry.name().starts_with("META-INF") {
                        continue;
                    } else if entry.is_file() {
                        temp_override_file.raw_copy_file(entry)?;
                    } else if entry.is_dir() {
                        // tracing::trace!("Added dir {}", entry.name());
                        temp_override_file.add_directory(
                            entry.name(),
                            zip::write::SimpleFileOptions::default(),
                        )?;
                    }
                }
            }

            temp_override_file.finish()?.sync_all()?;

            fs_err::remove_file(&full_override_path)?;
            fs_err::rename(full_temp_override_path, full_override_path)?;

            Ok(())
        } else {
            // 新版本均使用安装器安装
            let full_path = format!(
                "{root}/net/minecraftforge/forge/{mc}-{forge}/forge-{mc}-{forge}-installer.jar",
                root = self.minecraft_library_path.as_str(),
                mc = version_id,
                forge = forge_version
            );
            let tmp_full_path = format!(
        "{root}/net/minecraftforge/forge/{mc}-{forge}/forge-{mc}-{forge}-installer.tmp.{tempid}.jar",
        root = self.minecraft_library_path.as_str(),
        mc = version_id,
        forge = forge_version,
        tempid = std::time::SystemTime::now().elapsed().unwrap_or_default().as_secs()
    );
            tracing::trace!("Writing temp forge installer from {full_path} to {tmp_full_path}");
            {
                let from_file = fs_err::OpenOptions::new().read(true).open(&full_path)?;
                let to_file = fs_err::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&tmp_full_path)?;
                tracing::trace!("Modifying");
                self.modify_forge_installer(from_file, to_file, version_name)
                    .with_context(|| {
                        format!(
                            "修改 Forge 模组安装器文件 {full_path} 到 {tmp_full_path} 时发生错误"
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
            cmd.stderr(Stdio::inherit());
            cmd.stdin(Stdio::null());

            // 使用安装器自身声明的入口类执行无头安装。
            // 旧实现依赖内置 bootstrapper 反射调用固定的 main 签名，安装器升级后
            // 会因参数类型不匹配而失败；`--installClient` 是安装器长期支持的稳定入口。
            let main_class = super::installer_main_class(&full_path)?;

            cmd.arg("-cp");
            cmd.arg(&tmp_full_path);
            cmd.arg(main_class);
            cmd.arg("--installClient");
            cmd.arg(self.minecraft_path.as_str()); // 安装位置

            r.add_progress(1.);
            r.set_message("运行 Forge 安装器安装 Forge".into());

            tracing::trace!("Start running installer {cmd:?}");

            let mut child = cmd.spawn()?;

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
            r.add_progress(1.);
            r.remove_progress();
            fs_err::remove_file(&tmp_full_path)?;
            if !status.success() {
                anyhow::bail!(
                    "执行 Forge {}-{} 安装器失败，运行器返回值：{}",
                    version_id,
                    forge_version,
                    status
                );
            }
            let installed_version_json = std::path::Path::new(&self.minecraft_version_path)
                .join(version_name)
                .join(format!("{version_name}.json"));
            if !installed_version_json.is_file() {
                anyhow::bail!(
                    "Forge {}-{} 安装器已正常退出（运行器返回值：{}），但未找到版本元数据 {}",
                    version_id,
                    forge_version,
                    status,
                    installed_version_json.display()
                );
            }
            Ok(())
        }
    }

    fn modify_forge_installer(
        &self,
        from_reader: fs_err::File,
        to_writer: fs_err::File,
        name: &str,
    ) -> DynResult {
        tracing::trace!("Opening file");
        let mut file = zip::ZipArchive::new(std::io::BufReader::new(from_reader))
            .context("打开 Forge 安装器时发生错误")?;
        tracing::trace!("Opening file");
        let mut out_file = zip::ZipWriter::new(to_writer);
        // 安装器会从 install_profile.json 与 version.json 两处读取下载地址，
        // 只改其中一处会在真实安装时漏掉另一批库。
        let replace_source = match &self.source {
            DownloadSource::BMCLAPI => "https://bmclapi2.bangbang93.com/maven".to_owned(),
            DownloadSource::Custom(base) => {
                format!("{}/maven", base.as_str().trim_end_matches('/'))
            }
            DownloadSource::Default => "https://maven.minecraftforge.net".to_owned(),
        };
        tracing::trace!("Reading file");
        for index in 0..file.len() {
            if let Ok(mut entry) = file.by_index(index) {
                // 保留清单：NeoForge 安装器会读取自身 JAR 的 Manifest，
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
                                                            .strip_prefix(
                                                                "https://maven.minecraftforge.net",
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
                                                if let Some(down_path) = down_url.strip_prefix(
                                                    "https://maven.minecraftforge.net",
                                                ) {
                                                    *down_url =
                                                        format!("{replace_source}{down_path}");
                                                    tracing::trace!(
                                                        "已修改 libraries[{i}].url 字段"
                                                    );
                                                }
                                                if let Some(down_path) = down_url.strip_prefix(
                                                    "https://files.minecraftforge.net",
                                                ) {
                                                    *down_url =
                                                        format!("{replace_source}{down_path}");
                                                    tracing::trace!(
                                                        "已修改 libraries[{i}].url 字段"
                                                    );
                                                }
                                            }
                                        }
                                        // https://files.minecraftforge.net/
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
                                                                "https://maven.minecraftforge.net",
                                                            ) {
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
                                                        "https://maven.minecraftforge.net/",
                                                    ) {
                                                        *down_url =
                                                            format!("{replace_source}{down_path}");
                                                        tracing::trace!("已修改 versionInfo.libraries[{i}].url 字段");
                                                    }
                                                    if let Some(down_path) = down_url.strip_prefix(
                                                        "https://files.minecraftforge.net",
                                                    ) {
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
                                &[
                                    "https://maven.minecraftforge.net",
                                    "https://files.minecraftforge.net",
                                ],
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
