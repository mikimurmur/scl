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
        println!("cargo:warning=未设置 {name}，相关接口将无法使用");
    }
    println!("cargo:rustc-env={name}={value}");
}

fn main() {
    for path in ["build.rs", "src", "assets", "../assets", "../.env"] {
        println!("cargo:rerun-if-changed={path}");
    }
    load_dotenv();
    emit("CURSEFORGE_API_KEY");
    emit("MICROSOFT_LEGACY_CLIENT_ID");
}
