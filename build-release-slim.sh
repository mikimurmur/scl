#!/usr/bin/env bash
# release 瘦身构建入口。
#
# LTO / opt-level / panic / strip 等配置统一维护在根 Cargo.toml 的
# [profile.release]，本脚本只负责构建并打印产物体积，不再重复设置参数。
#
# 产物：target/release/SharpCraftLauncher
set -euo pipefail
cd "$(dirname "$0")"

cargo build -p sharp_craft_launcher --release "$@"

bin="${CARGO_TARGET_DIR:-target}/release/SharpCraftLauncher"
printf '%s: %s bytes\n' "$bin" "$(wc -c < "$bin")"
