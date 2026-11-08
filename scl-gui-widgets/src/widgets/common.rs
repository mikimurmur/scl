//! 一些通用的东西

use druid::{
    piet::{GradientStop, PaintBrush},
    Env, LinearGradient, PaintCtx, RenderContext, Size, UnitPoint,
};

use crate::theme::color as theme;

pub fn print_common_button(
    (is_accent, is_flat, is_active, is_hot, is_disabled): (bool, bool, bool, bool, bool),
    ctx: &mut PaintCtx,
    size: Size,
    env: &Env,
) {
    let border_width = 1.;
    let rounded_rect = size.to_rect().to_rounded_rect(4.);
    let border_rounded_rect = size
        .to_rect()
        .inflate(border_width / -2.0, border_width / -2.0)
        .to_rounded_rect(4. - border_width / 2.0);

    let bg = common_button_background(
        (
            is_accent,
            is_flat,
            is_active,
            is_hot,
            ctx.is_disabled() || is_disabled,
        ),
        env,
    );
    ctx.fill(rounded_rect, &PaintBrush::Color(bg));
    if !is_flat {
        let gradient_stops = vec![
            GradientStop {
                pos: 0.9067,
                color: druid::Color::Rgba32(0xFFFFFF14),
            },
            GradientStop {
                pos: 1.,
                color: druid::Color::Rgba32(0x00000066),
            },
        ];
        let border_gradient = PaintBrush::Linear(LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            gradient_stops,
        ));
        ctx.stroke(border_rounded_rect, &border_gradient, border_width);
    }
}

/// 通用按钮在当前主题与状态下的背景色。
///
/// 拆成纯函数便于单测断言深色/浅色主题下按钮底色确实随主题变化。
fn common_button_background(
    (is_accent, is_flat, is_active, is_hot, is_disabled): (bool, bool, bool, bool, bool),
    env: &Env,
) -> druid::Color {
    if is_disabled {
        env.get(theme::base::LOW)
    } else if is_accent {
        if is_active {
            env.get(theme::accent::ACCENT_DARK_1)
        } else if is_hot {
            env.get(theme::accent::ACCENT_LIGHT_1)
        } else {
            env.get(theme::accent::ACCENT)
        }
    } else if is_flat {
        if is_active {
            env.get(theme::base::LOW)
        } else if is_hot {
            env.get(theme::list::LIST_LOW)
        } else {
            druid::Color::TRANSPARENT
        }
    } else if is_active {
        env.get(theme::base::MEDIUM_LOW)
    } else if is_hot {
        env.get(theme::list::LIST_LOW)
    } else {
        env.get(theme::base::LOW)
    }
}

#[cfg(test)]
mod tests {
    use druid::{Color, Env};

    use crate::theme::color as theme;

    use super::common_button_background;

    fn over(foreground: Color, background: Color) -> Color {
        let (r, g, b, a) = foreground.as_rgba();
        let (br, bg_g, bb, _) = background.as_rgba();
        Color::rgba(
            r * a + br * (1. - a),
            g * a + bg_g * (1. - a),
            b * a + bb * (1. - a),
            1.,
        )
    }

    fn luminance(color: Color) -> f64 {
        let (r, g, b, _) = color.as_rgba();
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    fn theme_env(theme: theme::Theme) -> Env {
        let mut env = Env::empty();
        theme::set_color_to_env(&mut env, theme);
        env
    }

    #[test]
    fn normal_button_colors_change_with_theme() {
        let light = theme_env(theme::Theme::Light);
        let dark = theme_env(theme::Theme::Dark);
        let normal = (false, false, false, false, false);
        let hot = (false, false, false, true, false);
        let active = (false, false, true, false, false);

        let light_bg = common_button_background(normal, &light);
        let dark_bg = common_button_background(normal, &dark);
        assert_ne!(light_bg, dark_bg);

        // 深色主题：按钮底色叠加在深色窗口背景上仍是深色，文字是浅色。
        let dark_painted = over(dark_bg, dark.get(druid::theme::WINDOW_BACKGROUND_COLOR));
        assert!(luminance(dark_painted) < 0.35);
        assert!(luminance(dark.get(theme::typography::FONT_COLOR)) > 0.8);

        // 浅色主题：底色是浅色，文字是深色。
        let light_painted = over(light_bg, light.get(druid::theme::WINDOW_BACKGROUND_COLOR));
        assert!(luminance(light_painted) > 0.5);
        assert!(luminance(light.get(theme::typography::FONT_COLOR)) < 0.3);

        // hover/pressed 用不同主题键，与常态有可见差异。
        assert_ne!(dark_bg, common_button_background(hot, &dark));
        assert_ne!(
            common_button_background(hot, &dark),
            common_button_background(active, &dark)
        );
    }
}
