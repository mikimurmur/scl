//! 启动设置页面。

use iced::widget::{column, container, row, space};
use iced::{Alignment, Element, Length};
use scl_gui_widgets::icons;
use scl_gui_widgets::theme::Theme;
use scl_gui_widgets::widgets;

use crate::{
    app::Message,
    app_state::AppState,
    config::VersionIndependentMethod,
    core::DownloadSource,
    ui::{BoolField, ChoiceField, PAGE_SETTINGS_JAVA, TextField, UiMessage, common},
};

/// 创建一个 11px 的次级说明标签。
fn caption<'a>(theme: &Theme, text: impl Into<String>) -> Element<'a, Message> {
    common::hint(theme, text)
}

/// 创建一个开关设置项。
fn toggle<'a>(theme: &Theme, value: bool, field: BoolField) -> Element<'a, Message> {
    common::toggle_switch(theme, value, move |value| {
        Message::Ui(UiMessage::Bool(field, value))
    })
}

/// 一行「标签 + 输入框」。
fn advanced_field<'a>(
    theme: &Theme,
    label: &'static str,
    input: Element<'a, Message>,
) -> Element<'a, Message> {
    row![
        container(common::label(theme, label)).width(Length::Fixed(110.0)),
        space().width(Length::Fixed(8.0)),
        input,
    ]
    .align_y(Alignment::Center)
    .into()
}

/// 创建一个单选选项。
fn radio<'a>(
    theme: &Theme,
    label: &'static str,
    selected: bool,
    field: ChoiceField,
    index: usize,
) -> Element<'a, Message> {
    common::radio(
        theme,
        label,
        selected,
        Message::Ui(UiMessage::Choice(field, index)),
    )
}

/// 创建启动设置页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let java_item = common::large_list_item(
        &theme,
        icons::JAVA,
        "Java 运行时管理",
        "管理你电脑内的 Java 运行时环境",
        common::command(crate::app::PUSH_PAGE, PAGE_SETTINGS_JAVA),
    );

    let status = common::memory_status();
    let used = status.max.saturating_sub(status.free);
    let game_memory = if state.config.auto_mem_size {
        (status.free as f64 * 0.75).min(4000.0)
    } else {
        state.config.mem_size as f64
    };

    let memory_info = row![
        iced::widget::text(format!(
            "当前占用：{}",
            common::format_memory_text(used as f64)
        ))
        .size(13.0)
        .font(theme.typography.caption_alt.font)
        .color(theme.base.medium),
        space().width(Length::Fill),
        iced::widget::text(format!(
            "游戏内存：{}",
            common::format_memory_text(game_memory)
        ))
        .size(13.0)
        .font(theme.typography.caption_alt.font)
        .color(theme.base.medium),
    ]
    .align_y(Alignment::Center);

    let memory_input = if state.config.auto_mem_size {
        common::text_input_readonly(&theme, "内存大小", &state.setting_launch_java_memory_size)
    } else {
        common::text_input(
            &theme,
            "内存大小",
            &state.setting_launch_java_memory_size,
            move |value| Message::Ui(UiMessage::Text(TextField::GlobalMemory, value)),
        )
    };

    let manual_row = row![
        common::radio(
            &theme,
            "手动调节内存",
            !state.config.auto_mem_size,
            Message::Ui(UiMessage::Bool(BoolField::AutoMemory, false))
        ),
        space().width(Length::Fixed(12.0)),
        container(memory_input).width(Length::Fill),
        space().width(Length::Fixed(4.0)),
        common::label(&theme, "MB"),
    ]
    .align_y(Alignment::Center);

    let version_independent = column![
        radio(
            &theme,
            "不使用版本隔离",
            state.config.version_independent_method == VersionIndependentMethod::NoIndependent,
            ChoiceField::VersionIndependentMethod,
            0
        ),
        radio(
            &theme,
            "隔离非正式版",
            state.config.version_independent_method
                == VersionIndependentMethod::IndependentForNonRelease,
            ChoiceField::VersionIndependentMethod,
            1
        ),
        radio(
            &theme,
            "隔离模组加载器版",
            state.config.version_independent_method
                == VersionIndependentMethod::IndependentForModLoader,
            ChoiceField::VersionIndependentMethod,
            2
        ),
        radio(
            &theme,
            "隔离非正式版和模组加载器版",
            state.config.version_independent_method
                == VersionIndependentMethod::IndependentForModLoaderAndNonRelease,
            ChoiceField::VersionIndependentMethod,
            3
        ),
        radio(
            &theme,
            "隔离全部版本",
            state.config.version_independent_method == VersionIndependentMethod::IndependentAll,
            ChoiceField::VersionIndependentMethod,
            4
        ),
    ]
    .spacing(2);

    let download_source = column![
        radio(
            &theme,
            "默认（官方）下载源",
            state.config.download_source == DownloadSource::Default,
            ChoiceField::DownloadSource,
            0
        ),
        radio(
            &theme,
            "BMCLAPI 下载源",
            state.config.download_source == DownloadSource::BmclApi,
            ChoiceField::DownloadSource,
            1
        ),
        radio(
            &theme,
            "MCBBS 下载源",
            state.config.download_source == DownloadSource::Mcbbs,
            ChoiceField::DownloadSource,
            2
        ),
    ]
    .spacing(2);

    let concurrency = iced::widget::text(if state.config.download_parallel_amount == 0 {
        "下载并发量：不限（高资源消耗！）".to_owned()
    } else {
        format!("下载并发量：{} 个", state.config.download_parallel_amount)
    })
    .size(13.0)
    .font(theme.typography.caption_alt.font)
    .color(theme.font_color);

    let concurrency_slider = widgets::slider(
        &theme,
        0.0..=256.0,
        state.config.download_parallel_amount as f32,
        |value| Message::Ui(UiMessage::SetParallel(value.round() as usize)),
    );

    let advanced = column![
        advanced_field(
            &theme,
            "JVM 参数",
            common::text_input(
                &theme,
                "",
                &state.config.jvm_args,
                move |value| Message::Ui(UiMessage::Text(TextField::JvmArgs, value))
            )
        ),
        space().height(Length::Fixed(8.0)),
        advanced_field(
            &theme,
            "游戏参数",
            common::text_input(&theme, "", &state.config.game_args, move |value| {
                Message::Ui(UiMessage::Text(TextField::GameArgs, value))
            })
        ),
        space().height(Length::Fixed(8.0)),
        advanced_field(
            &theme,
            "包装执行程序路径",
            common::text_input(&theme, "", &state.config.wrapper_path, move |value| {
                Message::Ui(UiMessage::Text(TextField::WrapperPath, value))
            })
        ),
        space().height(Length::Fixed(8.0)),
        advanced_field(
            &theme,
            "包装执行程序参数",
            common::text_input(&theme, "", &state.config.wrapper_args, move |value| {
                Message::Ui(UiMessage::Text(TextField::WrapperArgs, value))
            })
        ),
    ]
    .spacing(0);

    let content = column![
        space().height(Length::Fixed(8.0)),
        common::section(&theme, "游戏运行内存"),
        memory_info,
        space().height(Length::Fixed(8.0)),
        common::memory_bar(&theme, if state.config.auto_mem_size {
            None
        } else {
            Some(state.config.mem_size)
        }),
        space().height(Length::Fixed(18.0)),
        common::radio(
            &theme,
            "自动调节内存",
            state.config.auto_mem_size,
            Message::Ui(UiMessage::Bool(BoolField::AutoMemory, true))
        ),
        manual_row,
        space().height(Length::Fixed(8.0)),
        common::label(&theme, "启动游戏时等待窗口出现后关闭启动器"),
        caption(
            &theme,
            "如果你的游戏启动出现启动器窗口没有关闭的问题可以取消勾选此项，如果关闭将无法使用检查文件完整性的功能"
        ),
        toggle(&theme, state.config.wait_until_launched, BoolField::WaitUntilLaunched),
        common::label(&theme, "启动后不关闭启动器"),
        toggle(
            &theme,
            state.config.dont_close_window_when_launched,
            BoolField::DontCloseWindow
        ),
        common::label(&theme, "启动后隐藏启动器"),
        toggle(
            &theme,
            state.config.hide_window_when_launched,
            BoolField::HideWindow
        ),
        common::label(&theme, "自动尝试检查文件完整性"),
        caption(&theme, "当启动游戏失败时自动尝试检查文件完整性并尝试重启"),
        toggle(
            &theme,
            state.config.auto_check_when_first_start_failed,
            BoolField::AutoRecheck
        ),
        space().height(Length::Fixed(8.0)),
        common::label(&theme, "版本隔离方式"),
        caption(
            &theme,
            "如需针对指定版本单独设置，请在版本页面里选择指定版本后点击高级设置进行设置"
        ),
        version_independent,
        space().height(Length::Fixed(8.0)),
        common::label(&theme, "优先下载源"),
        caption(&theme, "如果优先下载源下载失败了，则会依次尝试其他下载源"),
        download_source,
        space().height(Length::Fixed(8.0)),
        concurrency,
        concurrency_slider,
        caption(
            &theme,
            "虽然 SCL 可以调节并发量甚至不限并发量，但是 64 已经足够绝大部分电脑设备跑满带宽，所以如无必要无需调节并发数。"
        ),
        space().height(Length::Fixed(8.0)),
        common::label(&theme, "高级设置"),
        caption(
            &theme,
            "以下 JVM 参数和游戏参数均会直接附加在已有参数后面\n而版本独立设置将会覆盖下面的设置"
        ),
        space().height(Length::Fixed(8.0)),
        advanced,
    ]
    .spacing(4);

    let page = column![java_item, container(content).padding(12),].spacing(0);

    common::scroll(&theme, page)
}
