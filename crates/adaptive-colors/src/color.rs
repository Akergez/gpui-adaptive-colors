//! An opaque sRGB colour.
//!
//! The crate has a colour type of its own so that neither side of it leaks
//! into the other: the arithmetic underneath has one, every toolkit has
//! another, and a caller should need neither to hold a colour. It has no
//! alpha because a scheme has none; what is drawn translucent is drawn so by
//! whoever draws it.

use std::fmt;
use std::str::FromStr;

use material_colors::color::Argb;
use material_colors::hct::Hct;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

impl Color {
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Color { red, green, blue }
    }

    /// From `0xRRGGBB`. Anything above the low three bytes is ignored, so an
    /// ARGB integer from a platform can be handed over as it is.
    pub const fn from_u32(value: u32) -> Self {
        Color {
            red: (value >> 16) as u8,
            green: (value >> 8) as u8,
            blue: value as u8,
        }
    }

    /// From `#rrggbb`, with or without the `#`.
    pub fn parse_hex(hex: &str) -> Option<Self> {
        let digits = hex.strip_prefix('#').unwrap_or(hex);
        if digits.len() != 6 || !digits.bytes().all(|digit| digit.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(digits, 16).ok().map(Self::from_u32)
    }

    /// `#rrggbb`, lower case.
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }

    pub const fn red(self) -> u8 {
        self.red
    }

    pub const fn green(self) -> u8 {
        self.green
    }

    pub const fn blue(self) -> u8 {
        self.blue
    }

    /// Hue in degrees, as Material measures it (HCT), which is not the hue
    /// of HSL: equal steps of it look equal.
    pub fn hue(self) -> f64 {
        Hct::new(self.into()).get_hue()
    }

    /// How colourful it is, on HCT's open-ended scale: grey is 0, and the
    /// most a screen can show is somewhere past 100 depending on the hue.
    pub fn chroma(self) -> f64 {
        Hct::new(self.into()).get_chroma()
    }

    /// Lightness from 0 (black) to 100 (white), the L* of L*a*b*. Two colours
    /// 50 apart in tone have a contrast of at least 4.5:1 whatever their hues.
    pub fn tone(self) -> f64 {
        Argb::from(self).as_lstar()
    }

    /// The contrast ratio with `other`, from 1 to 21.
    pub fn contrast(self, other: Color) -> f64 {
        material_colors::contrast::ratio_of_tones(self.tone(), other.tone())
    }

    /// This colour laid over `base` at `opacity`, as one opaque colour. It is
    /// how Material shows hover and press: the content colour at 8% or 12%
    /// over the container.
    pub fn over(self, base: Color, opacity: f32) -> Color {
        let opacity = opacity.clamp(0.0, 1.0);
        let channel = |top: u8, bottom: u8| {
            (f32::from(top) * opacity + f32::from(bottom) * (1.0 - opacity)).round() as u8
        };
        Color {
            red: channel(self.red, base.red),
            green: channel(self.green, base.green),
            blue: channel(self.blue, base.blue),
        }
    }

    /// This colour with its hue pulled part of the way towards `source`'s,
    /// so that a fixed colour — the green of "success" — sits with a scheme
    /// instead of beside it. It stays recognisably itself: the hue moves by
    /// half the difference and never more than 15°.
    pub fn harmonized(self, source: Color) -> Color {
        material_colors::blend::harmonize(self.into(), source.into()).into()
    }
}

impl fmt::Display for Color {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

/// What [`Color::from_str`] says of a string that is not `#rrggbb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotAColor;

impl fmt::Display for NotAColor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("not a colour: expected #rrggbb")
    }
}

impl std::error::Error for NotAColor {}

impl FromStr for Color {
    type Err = NotAColor;

    fn from_str(hex: &str) -> Result<Self, Self::Err> {
        Color::parse_hex(hex).ok_or(NotAColor)
    }
}

impl From<Color> for Argb {
    fn from(color: Color) -> Self {
        Argb::new(u8::MAX, color.red, color.green, color.blue)
    }
}

impl From<Argb> for Color {
    fn from(argb: Argb) -> Self {
        Color::rgb(argb.red, argb.green, argb.blue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_survives_being_written_as_hex() {
        let color = Color::rgb(0x67, 0x50, 0xa4);
        assert_eq!(color.to_hex(), "#6750a4");
        assert_eq!(Color::parse_hex("#6750A4"), Some(color));
        assert_eq!("6750a4".parse(), Ok(color));
    }

    #[test]
    fn what_is_not_six_hex_digits_is_not_a_colour() {
        for text in ["", "#fff", "#6750a4ff", "#67 0a4", "+6750a4", "system"] {
            assert_eq!(Color::parse_hex(text), None, "{text:?}");
        }
    }

    #[test]
    fn an_argb_integer_from_a_platform_loses_only_its_alpha() {
        assert_eq!(Color::from_u32(0xff6750a4), Color::rgb(0x67, 0x50, 0xa4));
    }

    #[test]
    fn a_layer_at_no_opacity_is_the_base_and_at_full_opacity_itself() {
        let (top, base) = (Color::rgb(255, 255, 255), Color::rgb(0, 0, 0));
        assert_eq!(top.over(base, 0.0), base);
        assert_eq!(top.over(base, 1.0), top);
        assert_eq!(top.over(base, 0.5), Color::rgb(128, 128, 128));
    }

    #[test]
    fn a_harmonized_colour_moves_towards_the_source_and_keeps_its_tone() {
        let (green, source) = (Color::rgb(0x16, 0xa3, 0x4a), Color::rgb(0x67, 0x50, 0xa4));
        let harmonized = green.harmonized(source);
        let distance = |a: f64, b: f64| ((a - b).abs()).min(360.0 - (a - b).abs());

        assert!(distance(harmonized.hue(), source.hue()) < distance(green.hue(), source.hue()));
        assert!(distance(harmonized.hue(), green.hue()) <= 15.5);
        assert!((harmonized.tone() - green.tone()).abs() < 1.0);
    }
}
