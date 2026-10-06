//! Themes written for the Zed editor, in a gpui-kit application.
//!
//! Four parts, each usable without the ones after it:
//!
//! - [`format`] translates a Zed theme file into the toolkit's theme format.
//!   Arithmetic on JSON; nothing else.
//! - [`Registry`] asks Zed's extension registry what themes there are and
//!   fetches one.
//! - [`Store`] keeps what was fetched in a directory and reads it back as
//!   the toolkit's `ThemeConfig`s.
//! - [`Browser`] is a view over the three: search the registry, install,
//!   remove.
//!
//! What it leaves to the application is which theme is worn. A theme from
//! the store is an ordinary `ThemeConfig`: apply it as any other, or hand it
//! to `gpui-adaptive-colors` to wear in place of a scheme.
//!
//! No theme is compiled in. Everything there is, a person installed, under
//! its own licence.

mod browser;
pub mod format;
mod registry;
mod store;

pub use browser::{Browser, BrowserEvent};
pub use registry::{Extension, Registry, RegistryError, valid_id};
pub use store::{Store, StoreError, read};
