//! Material You colours for a gpui-kit application.
//!
//! [`init`] replaces the toolkit's light and dark themes with a pair made
//! from one colour — the system's, or one the application names — and keeps
//! them in step when that colour changes. The application goes on switching
//! between light and dark exactly as before, with `Theme::change`; it is
//! the two themes behind that switch that are different.
//!
//! ```ignore
//! gpui_kit::init(cx);
//! gpui_adaptive_colors::init(Options::new(Color::from_u32(0x6750a4)), cx);
//! ```
//!
//! Components need nothing more: they read `cx.theme()` and find the scheme
//! there ([`mapping`] says which role went where). An application's own
//! drawing can ask for a role the theme has no place for through
//! [`ActiveScheme`]: `cx.scheme().tertiary_container`.
//!
//! Only colours are touched. Fonts, sizes and radii stay as the application
//! set them.

pub mod mapping;
mod roles;
mod state;

pub use adaptive_colors::{Color, KeyColors, Palette, Scheme, Seed, Variant};
pub use roles::{ActiveScheme, SchemeColors, hsla};
pub use state::{
    AdaptiveColors, Options, Source, init, refresh, set_source, wear, wear_scheme,
};
