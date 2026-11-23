use std::{env, fs, path::PathBuf};

fn load_dotenv() {
    let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let path = PathBuf::from(manifest_dir).join("../.env");
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
        if !key.is_empty() && env::var_os(key).is_none() {
            env::set_var(key, value);
        }
    }
}

fn emit(name: &str) {
    println!("cargo:rerun-if-env-changed={name}");
    let value = env::var(name).unwrap_or_default();
    if value.is_empty() {
        println!("cargo:warning=未设置 {name}，相关功能将无法使用");
    }
    println!("cargo:rustc-env={name}={value}");
}

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_icon();
    }

    for path in ["build.rs", "src", "../.env"] {
        println!("cargo:rerun-if-changed={path}");
    }
    load_dotenv();
    emit("SCL_MICROSOFT_CLIENT_ID");

    // 源码或 .env 变化都会重跑构建脚本，从而在本 crate 实际重新编译时刷新
    // SCL_BUILD_DATE（未变更源码时沿用既有产物与日期）。
    let date = std::process::Command::new("date")
        .arg("+%Y/%m/%d")
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_default();
    println!("cargo:rustc-env=SCL_BUILD_DATE={date}");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let os = match target_os.as_str() {
        "windows" => "Windows",
        "macos" => "macOS",
        "linux" => "Linux",
        other => other,
    };
    println!("cargo:rustc-env=SCL_TARGET_OS={os}");
    println!(
        "cargo:rustc-env=SCL_TARGET_ARCH={}",
        env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default()
    );
}

/// 把 `assets/logo.ico` 嵌入 Windows 可执行文件。
///
/// 图标由 `assets/logo.svg` 生成：
/// `convert -background none assets/logo.svg -define icon:auto-resize=256,128,64,48,32,24,16 assets/logo.ico`
///
/// Druid 的 Windows 后端通过 `LoadIconW(..., MAKEINTRESOURCEW(1))` 读取
/// 资源 ID 1 作为窗口图标，`winresource` 的 `set_icon` 恰好写入该 ID。
fn embed_windows_icon() {
    let icon = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR 未设置"))
        .join("../assets/logo.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon(icon.to_str().expect("Windows 图标路径不是合法的 UTF-8"));
    if let Err(err) = resource.compile() {
        panic!("编译 Windows 图标资源失败：{err}");
    }
}
