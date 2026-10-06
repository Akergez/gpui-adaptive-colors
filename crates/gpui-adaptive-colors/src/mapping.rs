//! Which role of a Material 3 scheme each colour of a gpui-kit theme takes.
//!
//! The two vocabularies do not line up one to one. The theme's comes from
//! shadcn, where "secondary" is a quiet filled button and "accent" the
//! background of a hovered row; Material's "secondary" and "tertiary" are
//! accent hues. So the table goes by what a colour is used for, not by what
//! it is called.
//!
//! Only part of the theme is named here. The theme derives the rest from
//! what it is given — a danger button from `danger`, a table from the list —
//! and those derivations are left to it wherever they land on something
//! Material would also choose. What is named is what they would get wrong:
//! every colour that would otherwise fall back to the default theme's greys
//! and blues, and the hover and press states, which Material makes by
//! laying the content colour over the container at 8% and 12%.
//!
//! The colours go in under the names a theme file uses (`primary.background`)
//! because those are the theme's stable interface, and its base colours can
//! be set no other way.

use adaptive_colors::{Color, Palette, Scheme};
use gpui_kit::component::{ThemeConfig, ThemeConfigColors, ThemeMode};
use serde_json::{Map, Value};

/// Material's state layers.
const HOVER: f32 = 0.08;
const PRESS: f32 = 0.12;

/// The colours a scheme has no role for and an interface still needs, as
/// they are before being fitted to the scheme: Tailwind's 600s, which the
/// default theme uses for the same purposes.
const RED: Color = Color::from_u32(0xdc2626);
const GREEN: Color = Color::from_u32(0x16a34a);
const BLUE: Color = Color::from_u32(0x2563eb);
const YELLOW: Color = Color::from_u32(0xca8a04);
const CYAN: Color = Color::from_u32(0x0891b2);
const MAGENTA: Color = Color::from_u32(0x9333ea);

/// The colours of a theme for `scheme`, each under the name a theme file
/// gives it. An alpha, where there is one, is the fourth byte of the hex.
pub fn theme_colors(scheme: &Scheme) -> Vec<(&'static str, String)> {
    let mut table = Table(Vec::new());
    let mut set = |key: &'static str, color: Color| table.set(key, color);

    let surface = scheme.surface();
    let text = scheme.on_surface();

    set("background", surface);
    set("foreground", text);
    set("border", scheme.outline_variant());
    // Not the stronger `outline` Material draws a text field with: the theme
    // makes the hovered, pressed and selected fills of an outlined button
    // out of this colour, and from `outline` they come out as slabs of grey.
    set("input.border", scheme.outline_variant());
    set("ring", scheme.primary());
    set("caret", scheme.primary());
    set("muted.background", scheme.surface_container_high());
    set("muted.foreground", scheme.on_surface_variant());
    set("accent.background", scheme.surface_container_highest());
    set("accent.foreground", text);

    // A filled, a tonal and a destructive button, and the three statuses.
    let (success, warning, info) = (
        scheme.custom(GREEN),
        scheme.custom(YELLOW),
        scheme.custom(CYAN),
    );
    // Each is a container, what is written on it, and the container hovered
    // and pressed.
    let mut filled = |keys: [&'static str; 4], container: Color, content: Color| {
        let [background, foreground, hover, active] = keys;
        set(background, container);
        set(foreground, content);
        set(hover, content.over(container, HOVER));
        set(active, content.over(container, PRESS));
    };
    filled(
        [
            "primary.background",
            "primary.foreground",
            "primary.hover.background",
            "primary.active.background",
        ],
        scheme.primary(),
        scheme.on_primary(),
    );
    filled(
        [
            "secondary.background",
            "secondary.foreground",
            "secondary.hover.background",
            "secondary.active.background",
        ],
        scheme.secondary_container(),
        scheme.on_secondary_container(),
    );
    filled(
        [
            "danger.background",
            "danger.foreground",
            "danger.hover.background",
            "danger.active.background",
        ],
        scheme.error(),
        scheme.on_error(),
    );
    filled(
        [
            "success.background",
            "success.foreground",
            "success.hover.background",
            "success.active.background",
        ],
        success.color(),
        success.on_color(),
    );
    filled(
        [
            "warning.background",
            "warning.foreground",
            "warning.hover.background",
            "warning.active.background",
        ],
        warning.color(),
        warning.on_color(),
    );
    filled(
        [
            "info.background",
            "info.foreground",
            "info.hover.background",
            "info.active.background",
        ],
        info.color(),
        info.on_color(),
    );
    // The plain button: the surface, outlined.
    filled(
        [
            "button.background",
            "button.foreground",
            "button.hover.background",
            "button.active.background",
        ],
        surface,
        text,
    );

    // What floats, and what frames the content.
    set("popover.background", scheme.surface_container());
    set("popover.foreground", text);
    set("group_box.background", scheme.surface_container_low());
    set("group_box.foreground", text);
    set("title_bar.background", scheme.surface_container_low());
    set("title_bar.border", scheme.outline_variant());
    set("sidebar.background", scheme.surface_container_low());
    set("sidebar.foreground", text);
    set("sidebar.border", scheme.outline_variant());
    set("sidebar.accent.background", scheme.secondary_container());
    set(
        "sidebar.accent.foreground",
        scheme.on_secondary_container(),
    );
    set("sidebar.primary.background", scheme.primary());
    set("sidebar.primary.foreground", scheme.on_primary());

    // Rows. A table takes all of this from the list.
    set("list.hover.background", text.over(surface, HOVER));
    set("list.even.background", scheme.surface_container_low());
    set("list.head.background", scheme.surface_container());
    set("list.active.border", scheme.primary());

    // Controls.
    set("switch.background", scheme.outline_variant());
    set("slider.background", scheme.primary());
    set("progress.bar.background", scheme.primary());
    set("skeleton.background", scheme.surface_container_highest());
    set(
        "tab_bar.segmented.background",
        scheme.surface_container_high(),
    );
    set("link", scheme.primary());
    set("scrollbar.thumb.hover.background", scheme.outline());

    // Charts: one hue from light to dark, as the default theme has it, with
    // the scheme's own primary in the run.
    let run = if scheme.is_dark() {
        [90, 80, 60, 40, 30]
    } else {
        [80, 60, 40, 30, 20]
    };
    let charts = ["chart.1", "chart.2", "chart.3", "chart.4", "chart.5"];
    for (key, tone) in charts.into_iter().zip(run) {
        set(key, scheme.tone(Palette::Primary, tone));
    }
    set("chart.bullish", success.color());
    set("chart.bearish", scheme.error());

    // The colours the theme falls back on for anything it is not told.
    set("base.red", scheme.custom(RED).color());
    set("base.green", success.color());
    set("base.blue", scheme.custom(BLUE).color());
    set("base.yellow", warning.color());
    set("base.cyan", info.color());
    set("base.magenta", scheme.custom(MAGENTA).color());

    // What is drawn over something else and has to let it show.
    table.translucent("list.active.background", scheme.primary(), 0x1f);
    table.translucent("selection.background", scheme.primary(), 0x4d);
    table.translucent("scrollbar.background", surface, 0x00);
    table.translucent("scrollbar.thumb.background", scheme.outline(), 0xb3);
    table.translucent("chart.grid", scheme.outline_variant(), 0x99);
    table.0
}

/// The table being filled in.
struct Table(Vec<(&'static str, String)>);

impl Table {
    fn set(&mut self, key: &'static str, color: Color) {
        self.0.push((key, color.to_hex()));
    }

    /// `alpha` out of 255, written as the hex's fourth byte.
    fn translucent(&mut self, key: &'static str, color: Color, alpha: u8) {
        self.0.push((key, format!("{}{alpha:02x}", color.to_hex())));
    }
}

/// A theme for `scheme`: its colours and its mode, and nothing else. Fonts,
/// sizes and radii are not a scheme's to decide, and a theme applied from
/// this leaves them as they were.
pub fn theme_config(scheme: &Scheme) -> ThemeConfig {
    let colors: Map<String, Value> = theme_colors(scheme)
        .into_iter()
        .map(|(key, color)| (key.to_string(), Value::String(color)))
        .collect();
    let (name, mode) = if scheme.is_dark() {
        ("Adaptive Dark", ThemeMode::Dark)
    } else {
        ("Adaptive Light", ThemeMode::Light)
    };
    ThemeConfig {
        name: name.into(),
        mode,
        colors: serde_json::from_value::<ThemeConfigColors>(Value::Object(colors))
            .expect("every colour of a theme is a string"),
        ..ThemeConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::rc::Rc;

    use adaptive_colors::Seed;
    use gpui_kit::component::{Theme, try_parse_color};
    use gpui_kit::{Hsla, Rgba};

    use super::*;

    fn schemes() -> [Scheme; 2] {
        let seed = Seed::Color(Color::from_u32(0x6750a4));
        [Scheme::light(&seed), Scheme::dark(&seed)]
    }

    fn applied(scheme: &Scheme) -> Theme {
        let mut theme = Theme::default();
        theme.apply_config(&Rc::new(theme_config(scheme)));
        theme
    }

    fn color(hsla: Hsla) -> Color {
        let rgba = Rgba::from(hsla);
        let byte = |channel: f32| (channel * 255.0).round() as u8;
        Color::rgb(byte(rgba.r), byte(rgba.g), byte(rgba.b))
    }

    #[test]
    fn every_name_is_one_the_theme_knows_and_is_given_once() {
        for scheme in schemes() {
            let given = theme_colors(&scheme);
            let names: HashSet<_> = given.iter().map(|(name, _)| *name).collect();
            assert_eq!(names.len(), given.len());

            // A name the theme does not know is dropped on the way in, so
            // what comes back out is what it knew.
            let read_back = serde_json::to_value(theme_config(&scheme).colors).unwrap();
            for (name, value) in given {
                assert_eq!(read_back[name], Value::String(value), "{name}");
            }
        }
    }

    #[test]
    fn every_colour_is_one_the_theme_can_read() {
        for scheme in schemes() {
            for (name, value) in theme_colors(&scheme) {
                assert!(try_parse_color(&value).is_ok(), "{name}: {value}");
            }
        }
    }

    #[test]
    fn the_theme_takes_its_mode_and_main_colours_from_the_scheme() {
        for scheme in schemes() {
            let theme = applied(&scheme);
            assert_eq!(theme.mode.is_dark(), scheme.is_dark());
            assert_eq!(color(theme.background), scheme.surface());
            assert_eq!(color(theme.foreground), scheme.on_surface());
            assert_eq!(color(theme.primary), scheme.primary());
            assert_eq!(color(theme.primary_foreground), scheme.on_primary());
            assert_eq!(color(theme.danger), scheme.error());
            assert_eq!(color(theme.sidebar_accent), scheme.secondary_container());
        }
    }

    #[test]
    fn what_the_theme_derives_follows_the_scheme_too() {
        for scheme in schemes() {
            let theme = applied(&scheme);
            // Not named in the table: taken by the theme from what is.
            assert_eq!(theme.button_primary, theme.primary);
            assert_eq!(theme.table_head, theme.list_head);
            assert_eq!(theme.table_hover, theme.list_hover);
            assert_eq!(theme.window_border, theme.border);
            assert_eq!(theme.status_bar, theme.title_bar);
        }
    }

    #[test]
    fn text_reads_on_what_it_is_drawn_on() {
        for scheme in schemes() {
            let theme = applied(&scheme);
            let pairs = [
                ("text", theme.foreground, theme.background),
                ("muted text", theme.muted_foreground, theme.background),
                ("muted text on muted", theme.muted_foreground, theme.muted),
                ("primary", theme.primary_foreground, theme.primary),
                ("primary hover", theme.primary_foreground, theme.primary_hover),
                ("secondary", theme.secondary_foreground, theme.secondary),
                ("danger", theme.danger_foreground, theme.danger),
                ("success", theme.success_foreground, theme.success),
                ("warning", theme.warning_foreground, theme.warning),
                ("info", theme.info_foreground, theme.info),
                ("sidebar", theme.sidebar_foreground, theme.sidebar),
                (
                    "sidebar accent",
                    theme.sidebar_accent_foreground,
                    theme.sidebar_accent,
                ),
                ("popover", theme.popover_foreground, theme.popover),
                ("hovered row", theme.foreground, theme.list_hover),
            ];
            for (what, text, surface) in pairs {
                let contrast = color(text).contrast(color(surface));
                assert!(
                    contrast >= 4.5,
                    "{what}: {contrast:.2} in {}",
                    if scheme.is_dark() { "dark" } else { "light" }
                );
            }
        }
    }

    #[test]
    fn a_hovered_control_differs_from_one_at_rest() {
        for scheme in schemes() {
            let theme = applied(&scheme);
            assert_ne!(theme.primary_hover, theme.primary);
            assert_ne!(theme.primary_active, theme.primary_hover);
            assert_ne!(theme.button_hover, theme.button);
            assert_ne!(theme.colors.list_hover, theme.colors.list);
        }
    }

    #[test]
    fn a_theme_from_a_scheme_leaves_fonts_sizes_and_corners_alone() {
        let config = theme_config(&schemes()[0]);
        assert_eq!(config.font_size, None);
        assert_eq!(config.font_family, None);
        assert_eq!(config.mono_font_size, None);
        assert_eq!(config.radius, None);
        assert_eq!(config.shadow, None);
    }
}
