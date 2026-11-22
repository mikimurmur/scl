# scl-gui-iced

SCL 图形组件库的 [Iced](https://iced.rs) 迁移实验，使用 **tiny-skia** 软件渲染后端。

这个 crate 与旧的 `scl-gui-widgets` 并行存在：旧组件库继续基于 Druid 为现有启动器服务，
`scl-gui-iced` 则用 Iced 的声明式 UI 重新实现同一套 WinUI 3 / Fluent Design 风格，
用于验证把 UI 层迁移到 Iced + tiny-skia 的可行性。

## 运行示例

```sh
# 组件总览：输入框、开关、滑块、按钮、进度条、列表项、标题栏
cargo run -p scl-gui-iced --example widget_test

# 启动器外壳预览：主页、版本、下载、设置四个页面
cargo run -p scl-gui-iced --example launcher

# 测试与静态检查
cargo test -p scl-gui-iced
cargo clippy -p scl-gui-iced --all-targets -- -D warnings
```

示例窗口使用无边框模式，标题栏由组件库自绘；拖动、最小化、关闭都通过
Iced 的窗口任务完成。

## 已迁移内容

| 模块 | 内容 |
| --- | --- |
| `theme` | 亮色/暗色主题、SCL 品牌橙（`#F74C00`）、字体与字号 |
| `icons` | 从旧 `scl-gui` 图标集挑选的常用 SVG 图标，支持主题着色 |
| `widgets::button` | 普通/强调/危险/扁平按钮、启动按钮、设置行 |
| `widgets::icon_button` | 图标按钮与扁平图标按钮 |
| `widgets::text_input` | 文本输入框与密码输入框 |
| `widgets::toggle_switch` | 开关 |
| `widgets::slider` | 滑块 |
| `widgets::progress` | 确定进度条与不确定进度条 |
| `widgets::list_item` | 列表项（图标、副标题、选中态、右侧组件） |
| `widgets::navigation` | 仿 Pivot 的横向导航栏 |
| `widgets::page_switcher` | 页面栈与返回逻辑 |
| `widgets::window` | 自绘标题栏与窗口外壳 |
| `widgets::container` | 卡片、页面与主题化滚动容器 |

## 与 Druid 版本的差异

- **状态驱动页面切换**：Iced 没有 Druid 那种保留子组件树的 `PageSwitcher`。
  `PageSwitcher<P>` 只维护页面栈，页面内容由应用根据当前页面重新构建。
- **图标改为 SVG**：旧组件库直接保存 SVG 路径并用 Piet 绘制；Iced 的 `svg`
  组件需要完整 SVG 文档，因此运行时把路径包装成 SVG，再通过 `svg::Style::color` 着色。
- **标题栏透明色**：主题里的 `main.title_bar` 支持半透明/透明；透明时会退回到
  窗口背景色，并根据实际背景计算前景色。
- **动画由订阅驱动**：示例使用 `iced::window::frames()` 推进不确定进度条，
  因为 `thread-pool` 后端没有 `iced::time::every`。
- **品牌色**：亮色与暗色主题都使用 SCL 橙色作为主色，不再沿用 Fluent 蓝。

## 尚未迁移

以下旧组件/能力还没有对应实现，需要后续按实际页面需求补齐：

- `list_select`、`radio`、`menu_item`、`download_module_item`、`large_list_item`
- 背景图片与窗口透明/模糊效果
- 配置、日志、账户、下载等业务逻辑（示例只做界面外壳）
- 旧 `scl-gui` 的启动流程与 e2e 测试

## 依赖

```toml
iced = { version = "0.14", default-features = false, features = [
    "tiny-skia",
    "svg",
    "image",
    "crisp",
    "thread-pool",
    "x11",
    "wayland",
] }
```

Rust 版本：1.88（edition 2024）。
