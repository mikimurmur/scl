# Vendored `taskbar_interface`

来源：`https://github.com/Steve-xmh/taskbar_interface.git`，commit
`222b3c9f008b1044b0023433d7d864bf4c1306fc`。

上游的 `windows` 依赖写死为 `"*"`，且 Windows 实现只兼容 `windows` 0.52
（`HWND(isize)`）。本项目升级到 `windows` 0.62.2 后需要把句柄类型改为
`HWND(*mut c_void)`，因此将整个 crate 放入 `vendor/` 并做最小修改：

- `Cargo.toml`：`windows` 从 `"*"` 固定为 `"0.62.2"`；移除没有 build.rs /
  example 使用的 build-dependencies 与 dev-dependencies。
- `Cargo.toml`：macOS 依赖从 `icrate 0.0.4` 换成 `objc2 0.5` +
  `objc2-app-kit 0.2` + `objc2-foundation 0.2`。
- `src/macos/mod.rs`：跟随 `objc2 0.5` 重写（`Id` → `Retained`，
  `MainThreadMarker`，`NSProgressIndicatorStyle::Bar` 等）。`icrate 0.0.4`
  依赖的 `objc2 0.4.1` 在 Rust 1.100 下因 trait solver 变化无法编译，
  且上游没有 0.4.x 新版本。
- `src/windows/mod.rs`：适配 `windows` 0.62 的 `HWND` 指针类型，并显式忽略
  `FlashWindowEx` 的 `BOOL` 返回值。

除上述修改外，其余平台实现与上游保持一致。上游修复该问题后可以直接删除
本目录并把依赖改回 git 地址。
