# Vendored `taskbar_interface`

来源：`https://github.com/Steve-xmh/taskbar_interface.git`，commit
`222b3c9f008b1044b0023433d7d864bf4c1306fc`。

上游的 `windows` 依赖写死为 `"*"`，且 Windows 实现只兼容 `windows` 0.52
（`HWND(isize)`）。本项目升级到 `windows` 0.62.2 后需要把句柄类型改为
`HWND(*mut c_void)`，因此将整个 crate 放入 `vendor/` 并做最小修改：

- `Cargo.toml`：`windows` 从 `"*"` 固定为 `"0.62.2"`；移除没有 build.rs /
  example 使用的 build-dependencies 与 dev-dependencies。
- `src/windows/mod.rs`：适配 `windows` 0.62 的 `HWND` 指针类型，并显式忽略
  `FlashWindowEx` 的 `BOOL` 返回值。

除上述修改外，其余平台实现与上游保持一致。上游修复该问题后可以直接删除
本目录并把依赖改回 git 地址。
