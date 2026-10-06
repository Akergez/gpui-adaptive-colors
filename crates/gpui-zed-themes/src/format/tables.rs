//! Which of Zed's colours each of the toolkit's is taken from.
//!
//! Each row is a colour of the toolkit, under the name its theme files use,
//! and the keys of a Zed `style` to look in: the first that holds a colour
//! wins.

/// The colours that are taken as they are.
pub(super) const COLORS: &[(&str, &[&str])] = &[
    ("foreground", &["text", "editor.foreground"]),
    ("muted.foreground", &["text.muted", "text.placeholder"]),
    (
        "muted.background",
        &["surface.background", "element.background"],
    ),
    ("border", &["border", "border.variant"]),
    ("input.border", &["border", "border.variant"]),
    ("window.border", &["border", "border.variant"]),
    ("ring", &["border.focused", "text.accent"]),
    ("primary.background", &["text.accent", "link_text.hover"]),
    ("secondary.background", &["element.background"]),
    ("secondary.hover.background", &["element.hover"]),
    (
        "secondary.active.background",
        &["element.active", "element.selected"],
    ),
    ("secondary.foreground", &["text"]),
    (
        "accent.background",
        &["element.hover", "ghost_element.hover"],
    ),
    ("accent.foreground", &["text"]),
    ("link", &["link_text.hover", "text.accent"]),
    ("link.hover", &["link_text.hover", "text.accent"]),
    ("link.active", &["link_text.hover", "text.accent"]),
    ("popover.foreground", &["text"]),
    (
        "list.hover.background",
        &["ghost_element.hover", "element.hover"],
    ),
    (
        "list.active.background",
        &["ghost_element.selected", "element.selected"],
    ),
    ("sidebar.foreground", &["text"]),
    ("sidebar.border", &["border.variant", "border"]),
    (
        "sidebar.accent.background",
        &["ghost_element.selected", "element.selected"],
    ),
    ("sidebar.accent.foreground", &["text"]),
    ("sidebar.primary.background", &["text.accent"]),
    ("title_bar.border", &["border.variant", "border"]),
    ("tab.foreground", &["text.muted"]),
    ("tab.active.foreground", &["text"]),
    ("scrollbar.background", &["scrollbar.track.background"]),
    (
        "scrollbar.thumb.background",
        &["scrollbar.thumb.background"],
    ),
    (
        "scrollbar.thumb.hover.background",
        &["scrollbar.thumb.hover_background"],
    ),
    ("drop_target.background", &["drop_target.background"]),
    ("danger.background", &["error"]),
    ("success.background", &["success"]),
    ("warning.background", &["warning"]),
    ("info.background", &["info"]),
    ("base.red", &["terminal.ansi.red", "error"]),
    ("base.green", &["terminal.ansi.green", "success"]),
    ("base.blue", &["terminal.ansi.blue", "info"]),
    ("base.yellow", &["terminal.ansi.yellow", "warning"]),
    ("base.magenta", &["terminal.ansi.magenta"]),
    ("base.cyan", &["terminal.ansi.cyan"]),
];

/// The same, for what the window is filled with. A Zed theme may make these
/// translucent for a blurred window; an application's window is not one, and
/// a translucent surface on it shows whatever was drawn last underneath, so
/// they lose their alpha.
pub(super) const SURFACES: &[(&str, &[&str])] = &[
    ("background", &["editor.background", "background"]),
    (
        "popover.background",
        &["elevated_surface.background", "surface.background"],
    ),
    (
        "sidebar.background",
        &["panel.background", "surface.background"],
    ),
    (
        "title_bar.background",
        &["panel.background", "surface.background"],
    ),
    ("status_bar.background", &["status_bar.background"]),
    ("tab_bar.background", &["tab_bar.background"]),
    ("tab.background", &["tab.inactive_background"]),
    ("tab.active.background", &["tab.active_background"]),
];

/// The part of a Zed `style` the toolkit reads as it is, for code blocks.
pub(super) const HIGHLIGHT: &[&str] = &[
    "editor.background",
    "editor.foreground",
    "editor.active_line.background",
    "editor.line_number",
    "editor.active_line_number",
    "editor.invisible",
    "editor.gutter.background",
    "error",
    "error.background",
    "error.border",
    "warning",
    "warning.background",
    "warning.border",
    "info",
    "info.background",
    "info.border",
    "success",
    "success.background",
    "success.border",
    "hint",
    "hint.background",
    "hint.border",
];
