//! The roles of the scheme in use, in the toolkit's own colour type.
//!
//! The theme has a place for about half of a scheme's roles. The rest — the
//! tertiary accent, the containers, the surface steps — are here for the
//! application's own drawing, read the way the theme is read:
//! `cx.scheme().secondary_container` beside `cx.theme().border`.

use adaptive_colors::{Color, Scheme};
use gpui_kit::component::Theme;
use gpui_kit::{App, Hsla, rgb};

use crate::state::AdaptiveColors;

/// A colour of this library as one of the toolkit's, to draw with: a swatch
/// of a seed, say.
pub fn hsla(color: Color) -> Hsla {
    let packed = u32::from_be_bytes([0, color.red(), color.green(), color.blue()]);
    rgb(packed).into()
}

macro_rules! define_scheme_colors {
    ($($role:ident),* $(,)?) => {
        /// Every role of a Material 3 scheme, named as the specification
        /// names them. The fields are public for the same reason the theme's
        /// are: this is a palette to read from, not a thing with behaviour.
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct SchemeColors {
            $(pub $role: Hsla,)*
        }

        impl From<&Scheme> for SchemeColors {
            fn from(scheme: &Scheme) -> Self {
                SchemeColors {
                    $($role: hsla(scheme.$role()),)*
                }
            }
        }
    };
}

adaptive_colors::for_each_role!(define_scheme_colors);

/// Reads the scheme in use: the light one or the dark one, whichever the
/// theme is showing.
pub trait ActiveScheme {
    fn scheme(&self) -> &SchemeColors;
}

impl ActiveScheme for App {
    fn scheme(&self) -> &SchemeColors {
        AdaptiveColors::global(self).colors(Theme::global(self).mode.is_dark())
    }
}
