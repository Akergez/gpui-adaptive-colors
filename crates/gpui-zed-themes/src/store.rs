//! The themes a person has installed, as a directory.
//!
//! One subdirectory for each extension installed from the registry, named
//! after it, and beside those any file somebody put there by hand. A file may
//! be in Zed's format or the toolkit's. Zed's files are kept as they were
//! published and translated every time they are read ([`crate::format`]), so
//! a better translation in a later version reaches the themes already
//! installed.
//!
//! The toolkit has a theme registry of its own that can watch a directory,
//! and it is not used: it reads only its own format, and everything that
//! changes this directory goes through here anyway.
//!
//! Everything here touches the disk. Reading is a handful of small files and
//! is fine before the first frame; installing and removing belong off the
//! main thread.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gpui_kit::component::{ThemeConfig, ThemeSet};

use crate::format::{self, FormatError};
use crate::registry::valid_id;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{0}")]
    Format(#[from] FormatError),
    #[error("not a theme the toolkit can read: {0}")]
    Shape(#[from] serde_json::Error),
    #[error("{0}")]
    Disk(#[from] std::io::Error),
    #[error("none of the files in this extension is a theme this can read")]
    NothingUsable,
    #[error("\"{0}\" is not an extension name")]
    BadId(String),
}

/// The themes in a file of either format.
pub fn read(text: &str) -> Result<Vec<ThemeConfig>, StoreError> {
    let document = format::parse(text)?;
    let document = if format::is_zed(&document) {
        format::convert(&document)?
    } else {
        document
    };
    Ok(serde_json::from_value::<ThemeSet>(document)?.themes)
}

/// A directory of installed themes.
#[derive(Debug, Clone)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// The directory need not exist yet: an empty store is one that was
    /// never installed into.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Store { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The theme files: the directory's own, then those of each
    /// subdirectory, in name order so that the same directory always gives
    /// the same list.
    fn files(&self) -> Vec<PathBuf> {
        let listed = |dir: &Path| -> Vec<PathBuf> {
            let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .collect();
            paths.sort();
            paths
        };
        let is_theme =
            |path: &PathBuf| path.is_file() && path.extension().is_some_and(|e| e == "json");
        let top = listed(&self.dir);
        let mut found: Vec<PathBuf> = top.iter().filter(|path| is_theme(path)).cloned().collect();
        for directory in top.iter().filter(|path| path.is_dir()) {
            found.extend(listed(directory).into_iter().filter(is_theme));
        }
        found
    }

    /// Every theme there is. A file that cannot be read is said so in the
    /// log and skipped: one bad file must not cost the others. Two themes
    /// may share a name; which of them a name means is the caller's to
    /// decide.
    pub fn themes(&self) -> Vec<ThemeConfig> {
        let mut themes = Vec::new();
        for path in self.files() {
            let found = std::fs::read_to_string(&path)
                .map_err(StoreError::from)
                .and_then(|text| read(&text));
            match found {
                Ok(found) => themes.extend(found),
                Err(error) => {
                    tracing::warn!(path = %path.display(), %error, "ignored a theme file")
                }
            }
        }
        themes
    }

    /// The extensions installed from the registry, by their names there.
    pub fn installed(&self) -> BTreeSet<String> {
        std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| valid_id(name))
            .collect()
    }

    /// Keeps the theme files of an extension, replacing what was there under
    /// the same name. Answers how many themes that added.
    pub fn install(&self, id: &str, files: &[String]) -> Result<usize, StoreError> {
        if !valid_id(id) {
            return Err(StoreError::BadId(id.to_string()));
        }
        // Only what can be read is kept, and the count is taken before
        // anything is written: an extension with nothing usable leaves no
        // empty directory behind to be listed as installed.
        let usable: Vec<(&String, usize)> = files
            .iter()
            .filter_map(|file| Some((file, read(file).ok()?.len())))
            .collect();
        if usable.is_empty() {
            return Err(StoreError::NothingUsable);
        }
        let target = self.dir.join(id);
        match std::fs::remove_dir_all(&target) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.into()),
            _ => {}
        }
        std::fs::create_dir_all(&target)?;
        for (index, (file, _)) in usable.iter().enumerate() {
            std::fs::write(target.join(format!("{index:02}.json")), file)?;
        }
        Ok(usable.iter().map(|(_, themes)| themes).sum())
    }

    /// Forgets an installed extension.
    pub fn remove(&self, id: &str) -> Result<(), StoreError> {
        if !valid_id(id) {
            return Err(StoreError::BadId(id.to_string()));
        }
        Ok(std::fs::remove_dir_all(self.dir.join(id))?)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::component::ThemeMode;

    use super::*;

    const NATIVE: &str = r##"{"name": "Mine", "themes": [
        {"name": "Mine Dark", "mode": "dark", "colors": {"background": "#101010"}}]}"##;
    const ZED: &str = r##"{"name": "Theirs", "themes": [
        {"name": "Theirs Light", "appearance": "light", "style": {"text": "#202020"}},
        {"name": "Theirs Dark", "appearance": "dark", "style": {"text": "#e0e0e0"}}]}"##;

    fn scratch(name: &str) -> Store {
        let dir = std::env::temp_dir().join(format!(
            "gpui-zed-themes-test-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Store::new(dir)
    }

    #[test]
    fn a_file_in_either_format_is_read() {
        let native = read(NATIVE).unwrap();
        assert_eq!(native[0].name, "Mine Dark");
        assert_eq!(native[0].mode, ThemeMode::Dark);
        let zed = read(ZED).unwrap();
        assert_eq!(zed.len(), 2);
        assert_eq!(zed[0].mode, ThemeMode::Light);
        assert_eq!(zed[0].colors.foreground.as_deref(), Some("#202020"));
    }

    #[test]
    fn what_is_not_a_theme_is_an_error() {
        assert!(matches!(read("nonsense"), Err(StoreError::Format(_))));
        assert!(matches!(
            read(r#"{"themes": [{"style": {}}]}"#),
            Err(StoreError::Format(_))
        ));
        assert!(matches!(
            read(r#"{"themes": 4}"#),
            Err(StoreError::Shape(_))
        ));
    }

    #[test]
    fn a_store_that_was_never_installed_into_is_empty() {
        let store = Store::new(std::env::temp_dir().join("gpui-zed-themes-test-never-made"));
        assert!(store.themes().is_empty());
        assert!(store.installed().is_empty());
    }

    #[test]
    fn an_installed_extension_is_listed_read_and_removed() {
        let store = scratch("install");
        std::fs::write(store.dir().join("by-hand.json"), NATIVE).unwrap();
        std::fs::write(store.dir().join("notes.txt"), "not a theme").unwrap();

        let added = store
            .install("theirs", &["broken {".to_string(), ZED.to_string()])
            .unwrap();
        assert_eq!(added, 2);
        assert_eq!(store.installed(), BTreeSet::from(["theirs".to_string()]));
        let names: Vec<_> = store.themes().into_iter().map(|theme| theme.name).collect();
        assert_eq!(names, ["Mine Dark", "Theirs Light", "Theirs Dark"]);

        // Installing again replaces, it does not add up.
        store.install("theirs", &[ZED.to_string()]).unwrap();
        assert_eq!(store.themes().len(), 3);

        store.remove("theirs").unwrap();
        assert!(store.installed().is_empty());
        assert_eq!(store.themes().len(), 1);
        let _ = std::fs::remove_dir_all(store.dir());
    }

    #[test]
    fn an_extension_with_nothing_usable_leaves_nothing_behind() {
        let store = scratch("unusable");
        assert!(matches!(
            store.install("empty", &["broken {".to_string()]),
            Err(StoreError::NothingUsable)
        ));
        assert!(store.installed().is_empty());
        let _ = std::fs::remove_dir_all(store.dir());
    }

    #[test]
    fn a_name_that_is_a_path_is_refused_before_the_disk_is_touched() {
        let store = scratch("paths");
        assert!(matches!(
            store.install("../out", &[ZED.to_string()]),
            Err(StoreError::BadId(_))
        ));
        assert!(matches!(store.remove(".."), Err(StoreError::BadId(_))));
        assert!(store.dir().exists());
        let _ = std::fs::remove_dir_all(store.dir());
    }
}
