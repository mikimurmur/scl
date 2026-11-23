//! 游戏启动中页面。

use chrono::Datelike;
use iced::widget::{column, container, space};
use iced::{Element, Length};

use crate::{app::Message, app_state::AppState, ui::common};

/// 小萧生日（10 月 18 日）当天显示的贴士
const BIRTHDAY_TIPS: [&str; 3] = [
    "今天是小萧的第 {YEAR} 岁生日噢！",
    "蛋糕是个谎言！\n但生日蛋糕不是 :D",
    "Happy Birthday SteveXMH!",
];

/// SCL 诞生日（1 月 9 日）当天显示的贴士
const ANNIVERSARY_TIPS: [&str; 3] = [
    "今天是 SCL 的 {YEAR} 周年了！\n非常感谢你来使用我的启动器！",
    "一眨眼，SCL 已经有 {YEAR} 个年头了呢（感叹）",
    "今天是小萧和 SCL 耗上的第 {YEAR} 年（大雾）",
];

/// 平日常规贴士
const GENERAL_TIPS: [&str; 33] = [
    "这个提示功能是在 2021-05-23 里被加入的噢！",
    "SCL 启动器第一次出现是在小萧的教室电脑里，那时是 2021 年的一月呢~",
    "在爱发电支持之后，你可以手动修改配置文件来修改主题颜色，图标形状或者其他东西哦！",
    "SCL 启动器是用 Rust 编程语言编写的哦！",
    "用 Rust 开发带 GUI 的 MC 启动器 SCL 可是第一个哦！",
    "启动器目前可以选择三个下载源，你有没有试试呢？",
    "小萧曾为了写 UI 试图做一个 DirectUI 库出来……",
    "萧萧我啊，最喜欢▇▇了",
    "你所热爱的，就是你的▇▇",
    "高三党，现在！立刻！给我去学习！",
    "SCL 的设计工作都是在 Figma 里完成的哦！",
    "UI 好看吗？几乎全部控件都是小萧自己写的哦！",
    "Rust 来了，快__！",
    "Minecraft！启动！",
    "原来你也玩 MC！",
    "SCL 的大量设计均来自 WinUI 3 哦！",
    "如果你使用了第三方登录，点击头像下方的提示文字就会跳转到第三方验证服务的主页哦！",
    "版本列表里的图标是可以点进去的，此时可以对这个版本做更详细的修改哦！",
    "你什么都不知道",
    "我不知道",
    "反正我不知道",
    "我不想知道",
    "你不知道",
    "小萧我啊，想成为五边形游戏开发者呢……\n就是要变成那样很漫长呢……",
    "非常感谢你来使用我的启动器！",
    "你知道吗的文字其实是在 SCL 群里征集的，所以快来加群整点骚话吧！",
    "你知道吗的文字不超过 50 个字（其实从来没有超过 50 字的）",
    "如果启动器出了错误，可以用命令行启动启动器以获得更详细的日志输出哦！",
    "当这句话出现的时候，那你就该知道一件事了",
    "非常好启动器来自史蒂夫萧制作与爱",
    "在征集你知道吗的时候，作者只会一直在发方块狗狗甩水表情包",
    "使用 Rust 编写 GUI 程序其实非常辛苦，又小又好用的库少之又少……",
    "SCL 的启动核心在 Github 上完全开源可用哦！搜索 Steve-xmh/scl 即可找到！",
];

pub fn random_tip() -> String {
    let now = chrono::Local::now();
    let (tips, year) = if now.month() == 1 && now.day() == 9 {
        (&ANNIVERSARY_TIPS[..], Some(now.year() - 2021))
    } else if now.month() == 10 && now.day() == 18 {
        (&BIRTHDAY_TIPS[..], Some(now.year() - 2003))
    } else {
        (&GENERAL_TIPS[..], None)
    };
    let seed = chrono::Utc::now().timestamp_millis() as u64;
    let index = fastrand::Rng::with_seed(seed).usize(..tips.len());
    let tip = match year {
        Some(year) => tips[index].replace("{YEAR}", &year.to_string()),
        None => tips[index].to_owned(),
    };
    format!("你知道吗？\n\n{tip}")
}

/// 创建游戏启动中页面
pub fn view(state: &AppState) -> Element<'_, Message> {
    let theme = state.theme;

    let tip_card = container(
        iced::widget::text(state.starting_tip.clone().unwrap_or_default())
            .size(13.0)
            .font(theme.typography.caption_alt.font)
            .color(theme.font_color),
    )
    .width(Length::Fill)
    .padding(8)
    .style(move |_| iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
        border: iced::Border {
            radius: 4.0.into(),
            width: 2.0,
            color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.2),
        },
        ..iced::widget::container::Style::default()
    });

    column![
        space().height(Length::Fill),
        container(common::indeterminate(&theme, state.animation_phase, 4.0))
            .width(Length::Fixed(120.0)),
        space().height(Length::Fixed(8.0)),
        common::label(&theme, "启动游戏中"),
        space().height(Length::Fixed(16.0)),
        space().height(Length::Fill),
        tip_card,
        space().height(Length::Fixed(8.0)),
    ]
    .spacing(0)
    .align_x(iced::Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(8)
    .into()
}
