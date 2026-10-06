//! One theme of a family, translated.

use serde_json::{Map, Value, json};

use super::tables::{COLORS, HIGHLIGHT, SURFACES};

/// A colour as `#rrggbb` or `#rrggbbaa`, or nothing for anything else.
fn color(value: &Value) -> Option<String> {
    let hex = value.as_str()?.trim().strip_prefix('#')?;
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let hex = hex.to_ascii_lowercase();
    match hex.len() {
        3 | 4 => Some(format!(
            "#{}",
            hex.chars()
                .flat_map(|digit| [digit, digit])
                .collect::<String>()
        )),
        6 | 8 => Some(format!("#{hex}")),
        _ => None,
    }
}

/// A colour from [`color`], without its alpha.
fn opaque(color: &str) -> String {
    color.chars().take(7).collect()
}

/// How one kind of token is drawn. Zed's `oblique` has no counterpart and is
/// the nearest thing to italic; a weight is whatever number the author typed.
fn syntax_style(style: &Value) -> Option<Value> {
    let style = style.as_object()?;
    let mut out = Map::new();
    if let Some(found) = style.get("color").and_then(color) {
        out.insert("color".into(), found.into());
    }
    match style.get("font_style").and_then(Value::as_str) {
        Some("italic" | "oblique") => out.insert("font_style".into(), "italic".into()),
        Some("normal") => out.insert("font_style".into(), "normal".into()),
        _ => None,
    };
    if let Some(weight) = style.get("font_weight").and_then(Value::as_f64) {
        let weight = ((weight / 100.0).round() as i64).clamp(1, 9) * 100;
        out.insert("font_weight".into(), weight.into());
    }
    (!out.is_empty()).then(|| out.into())
}

/// One theme of a family; nothing for one without a name or colours, which
/// could be neither listed nor drawn.
pub(super) fn theme(theme: &Value) -> Option<Value> {
    let name = theme["name"]
        .as_str()
        .filter(|name| !name.trim().is_empty())?;
    let style = theme["style"].as_object()?;
    // Zed itself treats anything that is not "light" as dark.
    let mode = if theme["appearance"].as_str() == Some("light") {
        "light"
    } else {
        "dark"
    };

    let first = |keys: &[&str]| keys.iter().find_map(|key| color(style.get(*key)?));
    let mut colors = Map::new();
    for (target, sources) in COLORS {
        if let Some(found) = first(sources) {
            colors.insert((*target).into(), found.into());
        }
    }
    for (target, sources) in SURFACES {
        if let Some(found) = first(sources) {
            colors.insert((*target).into(), opaque(&found).into());
        }
    }
    if let Some(background) = colors.get("background").cloned() {
        colors.insert("primary.foreground".into(), background.clone());
        colors.insert("sidebar.primary.foreground".into(), background);
    }
    // The first "player" is the person at the keyboard: their cursor and
    // their selection are the theme's caret and selection.
    let player = style.get("players").and_then(|players| players.get(0));
    let of_player = |key: &str| player.and_then(|player| color(player.get(key)?));
    if let Some(caret) = of_player("cursor").or_else(|| first(&["text.accent"])) {
        colors.insert("caret".into(), caret.into());
    }
    if let Some(selection) = of_player("selection") {
        colors.insert("selection.background".into(), selection.into());
    }

    let mut highlight = Map::new();
    for key in HIGHLIGHT {
        if let Some(found) = style.get(*key).and_then(color) {
            highlight.insert((*key).into(), found.into());
        }
    }
    // The toolkit will not read a highlight without a `syntax` in it, so a
    // theme that has none gets an empty one rather than being refused whole.
    let syntax: Map<String, Value> = style
        .get("syntax")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(scope, style)| Some((scope.clone(), syntax_style(style)?)))
        .collect();
    highlight.insert("syntax".into(), syntax.into());

    Some(json!({
        "name": name.trim(),
        "mode": mode,
        "colors": colors,
        "highlight": highlight,
    }))
}
