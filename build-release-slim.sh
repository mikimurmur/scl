#!/usr/bin/env bash
# 瘦身 release 构建：功能与默认 release 完全一致，只改变代码生成和符号表。
#
# 相对默认 release（本机约 19.98 MB，未 strip）：
#   - lto=fat            跨 crate 内联 + 全局死代码消除
#   - codegen-units=1    单个代码单元，利于内联与去重
#   - opt-level=z        以体积为优化目标
#   - panic=abort        去掉展开表（.gcc_except_table、大部分 .eh_frame）
#   - strip=symbols      去掉 .symtab/.strtab
#
# 代价：
#   - panic=abort 后进程内 panic 直接终止，catch_unwind 不再能恢复；
#     panic 钩子仍会写日志/弹窗，但不会展开其它线程的栈。
#   - strip=symbols 后 backtrace 只剩地址，没有函数名（栈展开本身仍可用）。
#
# 用法：bash build-release-slim.sh
# 产物：target/release/SharpCraftLauncher
set -euo pipefail
cd "$(dirname "$0")"

export CARGO_PROFILE_RELEASE_LTO=fat
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_OPT_LEVEL=z
export CARGO_PROFILE_RELEASE_PANIC=abort
export CARGO_PROFILE_RELEASE_STRIP=symbols

cargo build -p sharp_craft_launcher --release "$@"

bin="${CARGO_TARGET_DIR:-target}/release/SharpCraftLauncher"
printf '%s: %s bytes\n' "$bin" "$(wc -c < "$bin")"
