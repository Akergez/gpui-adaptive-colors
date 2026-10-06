//! Reading a theme written for the Zed editor.
//!
//! Zed's is the theme format people actually publish in — several hundred
//! families in its extension registry — while the toolkit has one of its own
//! that almost nobody writes for. The two agree on how code is highlighted
//! (the toolkit copied that part from Zed on purpose) and on nothing else, so
//! this module is the dictionary between them: a Zed theme family goes in as
//! text, a document in the toolkit's format comes out.
//!
//! It is a translation between two vocabularies that do not line up, and it
//! takes sides where they differ:
//!
//! - Zed is an editor, and its `background` is the frame *around* the editor.
//!   What an application's window is mostly made of is the equivalent of the
//!   editor itself, so `editor.background` is the background here and the
//!   panels become the sidebar. The title bar is the sidebar's colour and not
//!   Zed's own for it: a window has two surfaces, the frame and the page.
//! - Zed has no "primary" colour, the one a default button is filled with.
//!   `text.accent` is the nearest thing a theme author chose on purpose, and
//!   the text on top of it is the background colour, which contrasts with an
//!   accent for the same reason the accent contrasts with the background.
//! - About forty of the toolkit's hundred and fifty colours are named
//!   ([`tables`]). The rest are derived by the toolkit from those, which is
//!   also what its own themes rely on.
//!
//! Everything here works on JSON values and knows nothing of the toolkit's
//! types, so a file somebody wrote by hand can be wrong in any way it likes:
//! a colour that is not a colour is dropped, never passed on to fail later.

mod relax;
mod tables;
mod theme;

use serde_json::{Map, Value};

/// Why a text is not a theme.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not a Zed theme: there is no list of themes in it")]
    NoThemes,
}

/// A theme file as text, parsed the way Zed parses one: comments and trailing
/// commas are allowed, and published themes do have them.
pub fn parse(source: &str) -> Result<Value, FormatError> {
    Ok(serde_json::from_str(&relax::relax(source))?)
}

/// Whether a parsed theme file is Zed's rather than the toolkit's: a Zed
/// theme says `appearance` and keeps its colours under `style`.
pub fn is_zed(document: &Value) -> bool {
    document["themes"].as_array().is_some_and(|themes| {
        themes
            .iter()
            .any(|theme| theme.get("style").is_some() || theme.get("appearance").is_some())
    })
}

/// A Zed theme family, as a theme set in the toolkit's format.
pub fn convert(document: &Value) -> Result<Value, FormatError> {
    let themes = document["themes"]
        .as_array()
        .ok_or(FormatError::NoThemes)?;
    let themes: Vec<Value> = themes.iter().filter_map(theme::theme).collect();
    if themes.is_empty() {
        return Err(FormatError::NoThemes);
    }
    let mut set = Map::new();
    set.insert(
        "name".into(),
        document["name"].as_str().unwrap_or_default().into(),
    );
    for key in ["author", "url"] {
        if let Some(text) = document[key].as_str() {
            set.insert(key.into(), text.into());
        }
    }
    set.insert("themes".into(), themes.into());
    Ok(set.into())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A family in Zed's format, written for these tests: the keys are the
    /// ones a published theme has, the colours are nobody's.
    const SAMPLE: &str = r##"{
        "name": "Sample", "author": "Nobody",
        "themes": [
            {"name": "Sample Dark", "appearance": "dark", "style": {
                "background": "#303840ff",
                "editor.background": "#101820ff",
                "panel.background": "#202830ff",
                "title_bar.background": "#404850ff",
                "text": "#e0e4e8ff",
                "text.muted": "#a0a4a8ff",
                "text.accent": "#60a0e0ff",
                "players": [{"cursor": "#60a0e0ff", "selection": "#60a0e03d"}],
                "syntax": {"boolean": {"color": "#c09060ff", "font_style": null,
                                       "font_weight": null}}}},
            {"name": "Sample Light", "appearance": "light", "style": {
                "editor.background": "#fafafaff", "text": "#202020ff"}}
        ]}"##;

    fn converted(source: &str) -> Value {
        convert(&parse(source).unwrap()).unwrap()
    }

    fn named<'a>(set: &'a Value, name: &str) -> &'a Value {
        set["themes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|theme| theme["name"] == name)
            .unwrap()
    }

    #[test]
    fn comments_and_trailing_commas_are_read_past() {
        let document = parse(
            "{\n  // a family\n  \"name\": \"C\", /* inline */\n  \"url\": \"https://a/b//c\",\n  \
             \"note\": \"a \\\" /* not a comment */ , ]\",\n  \"themes\": [1, 2, ],\n}",
        )
        .unwrap();
        assert_eq!(document["url"], "https://a/b//c");
        assert_eq!(document["note"], "a \" /* not a comment */ , ]");
        assert_eq!(document["themes"], json!([1, 2]));
    }

    #[test]
    fn invalid_json_is_an_error() {
        assert!(matches!(parse("{"), Err(FormatError::Json(_))));
    }

    #[test]
    fn the_two_formats_are_told_apart() {
        assert!(is_zed(&parse(SAMPLE).unwrap()));
        assert!(!is_zed(
            &json!({ "themes": [{ "name": "N", "mode": "dark", "colors": {} }] })
        ));
        assert!(!is_zed(&json!({ "name": "nothing" })));
    }

    #[test]
    fn a_family_becomes_a_theme_set_with_every_theme_in_its_mode() {
        let set = converted(SAMPLE);
        assert_eq!(set["name"], "Sample");
        assert_eq!(set["author"], "Nobody");
        assert_eq!(named(&set, "Sample Dark")["mode"], "dark");
        assert_eq!(named(&set, "Sample Light")["mode"], "light");
    }

    #[test]
    fn the_editor_is_the_background_and_the_panel_is_the_sidebar() {
        let set = converted(SAMPLE);
        let colors = &named(&set, "Sample Dark")["colors"];
        assert_eq!(colors["background"], "#101820");
        assert_eq!(colors["sidebar.background"], "#202830");
        // One frame: the bar over the window is the sidebar's colour.
        assert_eq!(colors["title_bar.background"], colors["sidebar.background"]);
        assert_ne!(colors["title_bar.background"], colors["background"]);
        assert_eq!(colors["foreground"], "#e0e4e8ff");
        assert_eq!(colors["muted.foreground"], "#a0a4a8ff");
    }

    #[test]
    fn the_accent_is_the_primary_colour_and_the_background_is_written_on_it() {
        let set = converted(SAMPLE);
        let colors = &named(&set, "Sample Dark")["colors"];
        assert_eq!(colors["primary.background"], "#60a0e0ff");
        assert_eq!(colors["primary.foreground"], colors["background"]);
    }

    #[test]
    fn the_first_player_gives_the_caret_and_the_selection() {
        let set = converted(SAMPLE);
        let colors = &named(&set, "Sample Dark")["colors"];
        assert_eq!(colors["caret"], "#60a0e0ff");
        assert_eq!(colors["selection.background"], "#60a0e03d");
    }

    #[test]
    fn syntax_colours_come_through_without_the_nulls() {
        let set = converted(SAMPLE);
        let highlight = &named(&set, "Sample Dark")["highlight"];
        assert_eq!(highlight["editor.background"], "#101820ff");
        assert_eq!(
            highlight["syntax"]["boolean"],
            json!({ "color": "#c09060ff" })
        );
    }

    #[test]
    fn a_translucent_surface_loses_its_alpha() {
        let set = converted(
            r##"{"name": "Glass", "themes": [{"name": "Glass", "appearance": "dark",
                "style": {"editor.background": "#10203080", "panel.background": "#abc8"}}]}"##,
        );
        let colors = &named(&set, "Glass")["colors"];
        assert_eq!(colors["background"], "#102030");
        assert_eq!(colors["sidebar.background"], "#aabbcc");
    }

    #[test]
    fn what_is_not_a_colour_is_left_out_rather_than_passed_on() {
        let set = converted(
            r##"{"name": "Odd", "themes": [{"name": "Odd", "appearance": "light", "style": {
                "text": "tomato", "border": null, "text.muted": "#12345",
                "editor.foreground": "#ABCDEF",
                "syntax": {"comment": {"color": 7, "font_style": "oblique", "font_weight": 650},
                           "string": {"color": "nope"}}}}]}"##,
        );
        let theme = named(&set, "Odd");
        // The second source is used when the first holds nonsense.
        assert_eq!(theme["colors"]["foreground"], "#abcdef");
        assert!(theme["colors"].get("border").is_none());
        assert!(theme["colors"].get("muted.foreground").is_none());
        assert_eq!(
            theme["highlight"]["syntax"],
            json!({ "comment": { "font_style": "italic", "font_weight": 700 } }),
        );
    }

    #[test]
    fn an_appearance_that_is_not_light_is_dark() {
        let set = converted(r#"{"themes": [{"name": "A", "style": {}}]}"#);
        assert_eq!(named(&set, "A")["mode"], "dark");
        // And a theme that says nothing about code still has a place for it.
        assert_eq!(named(&set, "A")["highlight"], json!({ "syntax": {} }));
    }

    #[test]
    fn a_theme_without_a_name_or_colours_is_skipped() {
        let set = converted(
            r#"{"themes": [{"name": " ", "style": {}}, {"name": "No style"},
                           {"name": "Kept", "style": {}}]}"#,
        );
        assert_eq!(set["themes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn a_file_with_no_themes_is_not_a_theme() {
        assert!(matches!(
            convert(&json!({ "name": "x" })),
            Err(FormatError::NoThemes)
        ));
        assert!(matches!(
            convert(&json!({ "themes": [] })),
            Err(FormatError::NoThemes)
        ));
    }
}
