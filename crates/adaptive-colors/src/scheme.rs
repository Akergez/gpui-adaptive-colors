//! A Material 3 colour scheme, worked out from a seed.
//!
//! A scheme is five tonal palettes — one hue and chroma each, at every
//! lightness — and a rule saying which lightness of which palette each role
//! takes in a light and in a dark interface. The rule is Google's
//! (`material-color-utilities`, through the `material-colors` crate); what is
//! here is the way in and the way out.
//!
//! The way in is a [`Seed`]: either one colour, from which the palettes are
//! derived, or the five palettes' own key colours when the system has
//! already derived them (Android does). The way out is [`Scheme`], with every
//! role worked out once and kept: finding a colour of a given tone is a
//! search, and a theme is read on every frame.
//!
//! The roles are named as the Material 3 specification names them
//! (<https://m3.material.io/styles/color/roles>), which is the documentation
//! of what each is for.

use std::fmt;
use std::str::FromStr;

use material_colors::dynamic_color::{DynamicScheme, Variant as DynamicVariant};
use material_colors::palette::TonalPalette;

use crate::Color;

/// Calls `$callback!` with the name of every role of a [`Scheme`], separated
/// by commas. It is how a crate for a toolkit declares its own struct with
/// the same roles in its own colour type without keeping a second list.
#[macro_export]
macro_rules! for_each_role {
    ($callback:ident) => {
        $callback! {
            primary, on_primary, primary_container, on_primary_container,
            inverse_primary,
            primary_fixed, primary_fixed_dim, on_primary_fixed, on_primary_fixed_variant,
            secondary, on_secondary, secondary_container, on_secondary_container,
            secondary_fixed, secondary_fixed_dim, on_secondary_fixed, on_secondary_fixed_variant,
            tertiary, on_tertiary, tertiary_container, on_tertiary_container,
            tertiary_fixed, tertiary_fixed_dim, on_tertiary_fixed, on_tertiary_fixed_variant,
            error, on_error, error_container, on_error_container,
            surface_dim, surface, surface_tint, surface_bright,
            surface_container_lowest, surface_container_low, surface_container,
            surface_container_high, surface_container_highest,
            on_surface, on_surface_variant, surface_variant,
            outline, outline_variant,
            inverse_surface, inverse_on_surface,
            background, on_background,
            shadow, scrim
        }
    };
}

/// How the palettes are derived from one colour. [`Variant::TonalSpot`] is
/// what Android does unless asked otherwise: a calm scheme in which only the
/// accents carry the colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    #[default]
    TonalSpot,
    /// Nearly grey, with a hint of the seed.
    Neutral,
    /// The seed at full strength.
    Vibrant,
    /// Hues deliberately moved away from the seed's.
    Expressive,
    /// A primary as close to the seed as a readable scheme allows.
    Fidelity,
    /// As [`Variant::Fidelity`], with accents drawn from the seed's
    /// neighbours.
    Content,
    Monochrome,
    Rainbow,
    FruitSalad,
}

impl From<Variant> for DynamicVariant {
    fn from(variant: Variant) -> Self {
        match variant {
            Variant::TonalSpot => DynamicVariant::TonalSpot,
            Variant::Neutral => DynamicVariant::Neutral,
            Variant::Vibrant => DynamicVariant::Vibrant,
            Variant::Expressive => DynamicVariant::Expressive,
            Variant::Fidelity => DynamicVariant::Fidelity,
            Variant::Content => DynamicVariant::Content,
            Variant::Monochrome => DynamicVariant::Monochrome,
            Variant::Rainbow => DynamicVariant::Rainbow,
            Variant::FruitSalad => DynamicVariant::FruitSalad,
        }
    }
}

/// One colour from each of a scheme's five palettes, from which the palette
/// is rebuilt: its hue and its chroma are what define it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyColors {
    primary: Color,
    secondary: Color,
    tertiary: Color,
    neutral: Color,
    neutral_variant: Color,
}

impl KeyColors {
    pub const fn new(
        primary: Color,
        secondary: Color,
        tertiary: Color,
        neutral: Color,
        neutral_variant: Color,
    ) -> Self {
        KeyColors {
            primary,
            secondary,
            tertiary,
            neutral,
            neutral_variant,
        }
    }

    fn all(&self) -> [Color; 5] {
        [
            self.primary,
            self.secondary,
            self.tertiary,
            self.neutral,
            self.neutral_variant,
        ]
    }
}

/// What a scheme is made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Seed {
    /// One colour; the [`Variant`] says how the palettes follow from it.
    Color(Color),
    /// Palettes somebody has already derived. The variant does not apply:
    /// that choice was theirs.
    KeyColors(KeyColors),
}

impl Seed {
    /// The one colour that best stands for the seed, for a swatch or for
    /// harmonizing other colours with the scheme.
    pub fn color(&self) -> Color {
        match self {
            Seed::Color(color) => *color,
            Seed::KeyColors(keys) => keys.primary,
        }
    }
}

impl From<Color> for Seed {
    fn from(color: Color) -> Self {
        Seed::Color(color)
    }
}

/// `#rrggbb`, or the five key colours separated by commas: a form to keep a
/// seed in a settings file between runs.
impl fmt::Display for Seed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Seed::Color(color) => color.fmt(formatter),
            Seed::KeyColors(keys) => {
                let [first, rest @ ..] = keys.all();
                first.fmt(formatter)?;
                rest.iter()
                    .try_for_each(|color| write!(formatter, ",{color}"))
            }
        }
    }
}

/// What [`Seed::from_str`] says of a string that is neither form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotASeed;

impl fmt::Display for NotASeed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("not a seed: expected one #rrggbb or five separated by commas")
    }
}

impl std::error::Error for NotASeed {}

impl FromStr for Seed {
    type Err = NotASeed;

    fn from_str(stored: &str) -> Result<Self, Self::Err> {
        let colors: Vec<Color> = stored
            .split(',')
            .map(|part| Color::parse_hex(part.trim()).ok_or(NotASeed))
            .collect::<Result<_, _>>()?;
        match colors[..] {
            [color] => Ok(Seed::Color(color)),
            [primary, secondary, tertiary, neutral, neutral_variant] => Ok(Seed::KeyColors(
                KeyColors::new(primary, secondary, tertiary, neutral, neutral_variant),
            )),
            _ => Err(NotASeed),
        }
    }
}

/// One of a scheme's palettes, for [`Scheme::tone`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Palette {
    Primary,
    Secondary,
    Tertiary,
    Neutral,
    NeutralVariant,
    Error,
}

/// What defines a tonal palette. Kept instead of the palette itself, which
/// is cheap to rebuild and is only asked for while a theme is being made.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Chord {
    hue: f64,
    chroma: f64,
}

impl Chord {
    fn of(palette: &TonalPalette) -> Self {
        Chord {
            hue: palette.hue(),
            chroma: palette.chroma(),
        }
    }

    fn of_color(color: Color) -> Self {
        Chord {
            hue: color.hue(),
            chroma: color.chroma(),
        }
    }

    fn palette(self) -> TonalPalette {
        TonalPalette::of(self.hue, self.chroma)
    }
}

/// A colour that is not part of the scheme, fitted to it: its hue pulled
/// towards the seed's, and four tones of it that pair up the way a scheme's
/// own roles do — `on_color` reads on `color`, `on_container` on `container`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CustomColor {
    color: Color,
    on_color: Color,
    container: Color,
    on_container: Color,
}

impl CustomColor {
    pub const fn color(&self) -> Color {
        self.color
    }

    pub const fn on_color(&self) -> Color {
        self.on_color
    }

    pub const fn container(&self) -> Color {
        self.container
    }

    pub const fn on_container(&self) -> Color {
        self.on_container
    }
}

macro_rules! define_scheme {
    ($($role:ident),* $(,)?) => {
        /// Every role of a Material 3 scheme, for one seed and one of light
        /// or dark.
        #[derive(Debug, Clone, PartialEq)]
        pub struct Scheme {
            dark: bool,
            source: Color,
            palettes: [Chord; 6],
            $($role: Color,)*
        }

        impl Scheme {
            $(
                pub const fn $role(&self) -> Color {
                    self.$role
                }
            )*

            /// Every role with its name, in the specification's order.
            pub fn roles(&self) -> impl Iterator<Item = (&'static str, Color)> {
                [$((stringify!($role), self.$role)),*].into_iter()
            }

            fn from_dynamic(dynamic: DynamicScheme) -> Self {
                let palettes = [
                    Chord::of(&dynamic.primary_palette),
                    Chord::of(&dynamic.secondary_palette),
                    Chord::of(&dynamic.tertiary_palette),
                    Chord::of(&dynamic.neutral_palette),
                    Chord::of(&dynamic.neutral_variant_palette),
                    Chord::of(&dynamic.error_palette),
                ];
                let (dark, source) = (dynamic.is_dark, dynamic.source_color_argb.into());
                let roles = material_colors::scheme::Scheme::from(dynamic);
                Scheme {
                    dark,
                    source,
                    palettes,
                    $($role: roles.$role.into(),)*
                }
            }
        }
    };
}

crate::for_each_role!(define_scheme);

impl Scheme {
    /// `contrast` runs from -1 to 1: 0 is the standard scheme, 0.5 and 1 are
    /// what Android calls medium and high contrast, and below zero is less
    /// contrast than standard.
    pub fn new(seed: &Seed, variant: Variant, contrast: f64, dark: bool) -> Self {
        let contrast = Some(contrast.clamp(-1.0, 1.0));
        let dynamic = match seed {
            Seed::Color(color) => {
                DynamicScheme::by_variant((*color).into(), &variant.into(), dark, contrast)
            }
            Seed::KeyColors(keys) => {
                let [primary, secondary, tertiary, neutral, neutral_variant] =
                    keys.all().map(|key| Chord::of_color(key).palette());
                DynamicScheme::new(
                    keys.primary.into(),
                    None,
                    // Only for how roles pick their tones, where this is the
                    // variant with no special cases.
                    DynamicVariant::TonalSpot,
                    dark,
                    contrast,
                    primary,
                    secondary,
                    tertiary,
                    neutral,
                    neutral_variant,
                    None,
                )
            }
        };
        Self::from_dynamic(dynamic)
    }

    /// The standard light scheme of a seed.
    pub fn light(seed: &Seed) -> Self {
        Self::new(seed, Variant::default(), 0.0, false)
    }

    /// The standard dark scheme of a seed.
    pub fn dark(seed: &Seed) -> Self {
        Self::new(seed, Variant::default(), 0.0, true)
    }

    pub const fn is_dark(&self) -> bool {
        self.dark
    }

    /// The colour the scheme was made from.
    pub const fn source(&self) -> Color {
        self.source
    }

    /// A palette's colour at a tone from 0 (black) to 100 (white), for what
    /// the roles do not cover — a run of chart colours, say.
    pub fn tone(&self, palette: Palette, tone: u8) -> Color {
        let chord = self.palettes[palette as usize];
        chord.palette().tone(i32::from(tone.min(100))).into()
    }

    /// Fits a colour from outside the scheme to it; see [`CustomColor`].
    pub fn custom(&self, color: Color) -> CustomColor {
        let harmonized = color.harmonized(self.source);
        // The floor on chroma is the specification's: a custom colour that
        // came in pale still has to read as a colour.
        let palette = Chord {
            hue: harmonized.hue(),
            chroma: harmonized.chroma().max(48.0),
        }
        .palette();
        let tones = if self.dark {
            [80, 20, 30, 90]
        } else {
            [40, 100, 90, 10]
        };
        let [color, on_color, container, on_container] =
            tones.map(|tone| Color::from(palette.tone(tone)));
        CustomColor {
            color,
            on_color,
            container,
            on_container,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEEDS: [u32; 5] = [0x6750a4, 0x0b57d0, 0x386a20, 0xff9cb1, 0x808080];

    fn schemes() -> impl Iterator<Item = Scheme> {
        SEEDS.into_iter().flat_map(|seed| {
            let seed = Seed::Color(Color::from_u32(seed));
            [Scheme::light(&seed), Scheme::dark(&seed)]
        })
    }

    #[test]
    fn text_reads_on_the_surface_it_is_meant_for() {
        for scheme in schemes() {
            let pairs = [
                (scheme.on_primary(), scheme.primary()),
                (scheme.on_primary_container(), scheme.primary_container()),
                (scheme.on_secondary(), scheme.secondary()),
                (scheme.on_secondary_container(), scheme.secondary_container()),
                (scheme.on_tertiary_container(), scheme.tertiary_container()),
                (scheme.on_error(), scheme.error()),
                (scheme.on_surface(), scheme.surface()),
                (scheme.on_surface(), scheme.surface_container_highest()),
                (scheme.on_surface_variant(), scheme.surface_container_highest()),
            ];
            for (text, surface) in pairs {
                assert!(
                    text.contrast(surface) >= 4.5,
                    "{text} on {surface} from {}",
                    scheme.source()
                );
            }
        }
    }

    #[test]
    fn a_dark_scheme_is_dark_and_a_light_one_light() {
        for scheme in schemes() {
            let (surface, text) = (scheme.surface().tone(), scheme.on_surface().tone());
            if scheme.is_dark() {
                assert!(surface < 20.0 && text > 80.0);
            } else {
                assert!(surface > 90.0 && text < 20.0);
            }
        }
    }

    #[test]
    fn the_containers_step_away_from_the_surface_in_order() {
        for scheme in schemes() {
            let steps = [
                scheme.surface_container_low(),
                scheme.surface_container(),
                scheme.surface_container_high(),
                scheme.surface_container_highest(),
            ]
            .map(|step| (step.tone() - scheme.surface().tone()).abs());
            assert!(steps.is_sorted(), "{steps:?}");
        }
    }

    #[test]
    fn the_primary_keeps_the_hue_of_its_seed() {
        let seed = Color::from_u32(0x386a20);
        for scheme in [Scheme::light(&seed.into()), Scheme::dark(&seed.into())] {
            assert!((scheme.primary().hue() - seed.hue()).abs() < 6.0);
        }
    }

    #[test]
    fn key_colours_give_palettes_of_their_own_hues() {
        // Nothing derives the tertiary from the primary here: a system that
        // chose an unrelated one gets it.
        let grey = Color::from_u32(0x777777);
        let keys = KeyColors::new(
            Color::from_u32(0x6750a4),
            Color::from_u32(0x625b71),
            Color::from_u32(0x1b6b3a),
            grey,
            grey,
        );
        let scheme = Scheme::light(&Seed::KeyColors(keys));

        assert!((scheme.tertiary().hue() - Color::from_u32(0x1b6b3a).hue()).abs() < 6.0);
        assert!(scheme.surface().chroma() < 2.0);
        assert_eq!(scheme.source(), Color::from_u32(0x6750a4));
    }

    #[test]
    fn more_contrast_takes_accents_and_outlines_further_from_the_surface() {
        let seed = Seed::Color(Color::from_u32(0x6750a4));
        let at = |level| Scheme::new(&seed, Variant::TonalSpot, level, false);
        let (standard, high) = (at(0.0), at(1.0));
        let from_surface = |scheme: &Scheme, role: Color| role.contrast(scheme.surface());

        assert!(from_surface(&high, high.primary()) > from_surface(&standard, standard.primary()));
        assert!(from_surface(&high, high.outline()) > from_surface(&standard, standard.outline()));
        // Out of range is the nearest end of the range, not an error.
        assert_eq!(at(7.0), high);
    }

    #[test]
    fn a_palette_runs_from_black_to_white() {
        let scheme = Scheme::light(&Seed::Color(Color::from_u32(0x6750a4)));
        assert_eq!(scheme.tone(Palette::Primary, 0), Color::rgb(0, 0, 0));
        assert_eq!(scheme.tone(Palette::Primary, 100), Color::rgb(255, 255, 255));
        assert_eq!(scheme.tone(Palette::Primary, 200), Color::rgb(255, 255, 255));
        assert!(
            scheme.tone(Palette::Neutral, 50).chroma() < scheme.tone(Palette::Primary, 50).chroma()
        );
    }

    #[test]
    fn a_custom_colour_pairs_up_like_a_role_in_both_modes() {
        let green = Color::from_u32(0x16a34a);
        for scheme in schemes() {
            let custom = scheme.custom(green);
            assert!(custom.on_color().contrast(custom.color()) >= 4.5);
            assert!(custom.on_container().contrast(custom.container()) >= 4.5);
            assert!(custom.color().contrast(scheme.surface()) >= 3.0);
        }
    }

    #[test]
    fn every_role_is_listed_once() {
        let scheme = Scheme::light(&Seed::Color(Color::from_u32(0x6750a4)));
        let mut names: Vec<_> = scheme.roles().map(|(name, _)| name).collect();
        assert_eq!(names.len(), 49);
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 49);
    }

    #[test]
    fn a_seed_survives_being_stored() {
        let color = Color::from_u32(0x6750a4);
        let keys = KeyColors::new(
            color,
            Color::from_u32(0x625b71),
            Color::from_u32(0x7d5260),
            Color::from_u32(0x79747e),
            Color::from_u32(0x79747f),
        );
        for seed in [Seed::Color(color), Seed::KeyColors(keys)] {
            assert_eq!(seed.to_string().parse(), Ok(seed));
        }
        assert_eq!(Seed::Color(color).to_string(), "#6750a4");
    }

    #[test]
    fn a_seed_is_one_colour_or_five() {
        for stored in ["", "system", "#6750a4,#625b71", "#6750a4,", "#6750a4,#zzzzzz"] {
            assert_eq!(stored.parse::<Seed>(), Err(NotASeed), "{stored:?}");
        }
    }
}
