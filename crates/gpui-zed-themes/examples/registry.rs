//! Asks Zed's registry what themes there are, fetches the most downloaded
//! extension and says what is in it:
//! `cargo run -p gpui-zed-themes --example registry [extension-id]`.
//!
//! Nothing is written to disk. It is the way to see whether the registry
//! still answers the way this crate expects.

use gpui_zed_themes::{Registry, read};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = Registry::new(concat!("gpui-zed-themes/", env!("CARGO_PKG_VERSION")));
    let extensions = registry.list()?;
    println!("{} extensions with themes", extensions.len());
    for extension in extensions.iter().take(5) {
        println!(
            "  {} ({}), {} downloads",
            extension.title(),
            extension.id(),
            extension.download_count()
        );
    }

    let wanted = std::env::args().nth(1);
    let Some(id) = wanted.as_deref().or(extensions.first().map(|first| first.id())) else {
        return Ok(());
    };
    let files = registry.download(id)?;
    println!("{id}: {} theme files", files.len());
    for file in &files {
        for theme in read(file)? {
            println!(
                "  {} ({}), {} of its colours used",
                theme.name,
                theme.mode.name(),
                serde_json::to_value(&theme.colors)?
                    .as_object()
                    .map_or(0, |colors| colors.values().filter(|color| !color.is_null()).count())
            );
        }
    }
    Ok(())
}
