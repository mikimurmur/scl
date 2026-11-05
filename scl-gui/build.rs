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
    let os = match std::env::consts::OS {
        "windows" => "Windows",
        "macos" => "macOS",
        other => other,
    };
    println!("cargo:rustc-env=SCL_TARGET_OS={os}");
    println!("cargo:rustc-env=SCL_TARGET_ARCH={}", std::env::consts::ARCH);
}
