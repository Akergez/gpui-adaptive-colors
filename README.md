# adaptive-colors

Material You colours for Rust applications: a whole colour scheme worked out
from one colour, and that colour taken from the system.

Three crates.

| Crate | What it is |
|---|---|
| `adaptive-colors` | No toolkit. A seed in, the roles of a Material 3 scheme out, light or dark; and, behind the `system` feature, the colour the platform has chosen. |
| `gpui-adaptive-colors` | The same for a [gpui-kit](https://gpui-kit.com) application: the scheme as the toolkit's theme, kept in step with the system. Pinned to one gpui-kit version (`0.7.0`). |
| `gpui-zed-themes` | Themes made for the Zed editor, for someone who would rather wear one than a scheme: read, fetched from Zed's registry, kept on disk, browsed. Does not depend on the other two. |

## In a gpui-kit application

```rust
use gpui_adaptive_colors::{Color, Options};

gpui_kit::init(cx);
gpui_adaptive_colors::init(Options::new(Color::from_u32(0x2fb380)), cx);
```

That is all a component needs: it reads `cx.theme()` as before and finds the
scheme there. The colour given is the fallback for a system with none.

- Light or dark is still `Theme::change(mode, None, cx)`. The library puts a
  light and a dark theme behind the two modes; it does not pick between them.
- `set_source(color, cx)` names a colour instead of following the system;
  `set_source(Source::System, cx)` goes back.
- `cx.scheme()` (the `ActiveScheme` trait) has every role, for drawing of your
  own that the theme has no colour for: `cx.scheme().secondary_container`.
- Only colours change. Fonts, sizes and radii stay as you set them.

The system is asked off the main thread, so its answer arrives after the first
frame. To start in the right colours, keep the last answer and hand it back:

```rust
let remembered = stored.and_then(|text| text.parse::<Seed>().ok());
gpui_adaptive_colors::init(Options::new(fallback).remembered(remembered), cx);

cx.observe_global::<AdaptiveColors>(|cx| {
    let said = AdaptiveColors::global(cx).system_seed().map(|seed| seed.to_string());
    // write `said` wherever settings are kept
})
.detach();
```

Which role goes to which colour of the theme is one table,
`gpui-adaptive-colors/src/mapping.rs`, with the reasons beside it.

## Where the system's colour comes from

| Platform | Source | Changes |
|---|---|---|
| Linux, FreeBSD | The desktop's accent colour, from the settings portal (`org.freedesktop.appearance`, `accent-color`). | Followed as they happen. |
| Android 12+ | The palettes the system derives from the wallpaper (`system_accent1_*` … `system_neutral2_*`), read through JNI. No Java is needed. | Not announced: call `refresh(cx)` when the application comes back to the front. |
| Windows, macOS, older Android | Not asked yet. | The fallback colour is used. |

On Android the virtual machine and the activity are found through
[`ndk-context`](https://crates.io/crates/ndk-context), which `android-activity`
fills in. The Android code compiles for `aarch64-linux-android` and has **not
been run on a device yet**.

To see what your system says:

```sh
cargo run -p adaptive-colors --features system --example system
```

## Themes from the Zed editor

Zed's is the theme format people publish in: several hundred families in its
extension registry. `gpui-zed-themes` is four parts, each usable without the
ones after it.

```rust
use gpui_zed_themes::{Browser, BrowserEvent, Registry, Store};

let registry = Registry::new("my-app/1.0");       // api.zed.dev
let store = Store::new(data_dir.join("themes"));  // one directory per extension

// Without a view: both calls block, so not on the main thread.
let files = registry.download("catppuccin")?;
store.install("catppuccin", &files)?;
let themes = store.themes();                      // Vec<ThemeConfig>

// Or the view: a search box over the registry, with Install and Remove.
let browser = cx.new(|cx| Browser::new(registry, store, window, cx));
browser.update(cx, |browser, cx| browser.load(cx));
cx.subscribe(&browser, |_, _, _: &BrowserEvent, cx| { /* read the store again */ });
```

- `format` translates a Zed theme file into the toolkit's format; the tables
  of which colour comes from where are `format/tables.rs`.
- A theme is kept as it was published and translated on every read, so a
  better translation reaches themes already installed.
- Only the text of `themes/*.json` is taken out of an extension, in memory,
  under a size limit. No theme is compiled in.
- Which theme is worn is the application's business. With
  `gpui-adaptive-colors` it is `wear(theme, cx)` to put a theme in place of
  the scheme for the mode the theme is for, and `wear_scheme(mode, cx)` to
  take it off again.

To see whether the registry still answers the way the crate expects:

```sh
cargo run -p gpui-zed-themes --example registry
```

## Without a toolkit

```rust
use adaptive_colors::{Color, Scheme, Seed};

let seed = Seed::Color(Color::from_u32(0x6750a4));
let scheme = Scheme::dark(&seed);
scheme.primary();              // every role is a method
scheme.custom(brand_green);    // a colour of your own, fitted to the scheme
```

The arithmetic is Google's `material-color-utilities`, through the
[`material-colors`](https://crates.io/crates/material-colors) crate (0.4: 0.5
needs a newer compiler than the 1.95 this builds with).

## Tests

```sh
cargo test --workspace
```
