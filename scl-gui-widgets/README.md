## SCL GUI Widgets

SCL 启动器的 [Iced](https://iced.rs) 组件库，组件风格大量参考了 WinUI 3 和 Fluent Design 设计。

组件库基于 Iced 0.14，默认使用 tiny-skia 渲染后端（不依赖 wgpu），提供主题、SVG 图标、按钮、列表项、导航、开关、进度条和滑动页面切换等组件。

### 示例

```sh
cargo run -p scl-gui-widgets --example widget_test
cargo run -p scl-gui-widgets --example launcher
```
