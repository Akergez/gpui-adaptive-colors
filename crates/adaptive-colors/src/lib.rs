//! Material You colours without a toolkit.
//!
//! Two halves. [`Scheme`] is arithmetic: one colour in, the fifty-odd roles
//! of a Material 3 scheme out, for a light or a dark interface. `system`
//! (behind the feature of that name) asks the platform which colour that
//! should be.
//!
//! Nothing here knows what draws the result. A crate for one toolkit maps
//! the roles onto that toolkit's theme; `gpui-adaptive-colors` is such a
//! crate.

mod color;
mod scheme;
#[cfg(feature = "system")]
pub mod system;

pub use color::Color;
pub use scheme::{CustomColor, KeyColors, Palette, Scheme, Seed, Variant};
