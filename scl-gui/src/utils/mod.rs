//! 启动器使用的一些工具。

#[cfg(target_os = "windows")]
pub mod console;
pub mod fix_minecraft;
pub mod image;
pub mod open_url;
pub mod prelude;
pub mod progress;
pub mod thread;
pub mod update;

pub use progress::Progress;
