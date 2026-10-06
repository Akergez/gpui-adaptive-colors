//! Zed's extension registry, as a place to get themes from.
//!
//! There is no repository of Zed themes to clone: the registry is a list of
//! several hundred other people's repositories, and what the editor itself
//! talks to is an API in front of them — one request for the list, one for a
//! packaged extension. This module makes those two requests and nothing else.
//! It hands back the theme files as text, without their names; what they mean
//! is [`crate::format`]'s business and where they are kept is
//! [`crate::Store`]'s.
//!
//! Both requests block. The crate knows no executor, so a caller with a main
//! thread to keep free makes them from somewhere else, as [`crate::Browser`]
//! does on GPUI's background executor.
//!
//! The API is the editor's own and promises nothing to anybody else, so every
//! failure here is an ordinary one to be shown and survived.
//!
//! What comes back was written by strangers. The archive is unpacked in
//! memory and only the text of `themes/*.json` is taken out of it: no path in
//! it is ever used as a path here. Both the download and each file have a
//! size they may not exceed — a theme is a few dozen kilobytes.

use std::io::Read;
use std::time::Duration;

use serde::Deserialize;

/// Where the registry is, unless [`Registry::at`] says otherwise.
const API: &str = "https://api.zed.dev";
const MAX_ARCHIVE: u64 = 8 * 1024 * 1024;
const MAX_THEME: u64 = 2 * 1024 * 1024;
const MAX_THEMES: usize = 64;

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("{0}")]
    Http(#[from] reqwest::Error),
    #[error("the registry's answer could not be read: {0}")]
    Answer(#[from] serde_json::Error),
    #[error("the extension is larger than a theme has any reason to be")]
    TooLarge,
    #[error("the extension could not be unpacked: {0}")]
    Archive(#[from] std::io::Error),
    #[error("there are no themes in this extension")]
    NoThemes,
    #[error("\"{0}\" is not an extension name")]
    BadId(String),
}

/// One entry of the registry, as much of it as a list row shows.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Extension {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
    #[serde(default)]
    download_count: u64,
}

impl Extension {
    /// Its name in the registry, which is also what it is installed under.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// What to call it: its name, or its id where it has none.
    pub fn title(&self) -> &str {
        if self.name.is_empty() {
            &self.id
        } else {
            &self.name
        }
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The first author without the address: registry entries are written
    /// as `Name <mail>`.
    pub fn author(&self) -> Option<&str> {
        let author = self.authors.first()?;
        let name = author.split('<').next().unwrap_or(author).trim();
        (!name.is_empty()).then_some(name)
    }

    pub fn download_count(&self) -> u64 {
        self.download_count
    }

    /// Whether `needle`, already lowercased, is somewhere a person would
    /// look for it.
    pub fn matches(&self, needle: &str) -> bool {
        needle.is_empty()
            || self.name.to_lowercase().contains(needle)
            || self.id.contains(needle)
            || self
                .description
                .as_deref()
                .is_some_and(|text| text.to_lowercase().contains(needle))
    }
}

/// An extension name is also a directory name here, so it may only be made
/// of what the registry itself allows in one.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
}

/// How to reach the registry. Cheap to copy around: nothing is opened until
/// a request is made.
#[derive(Debug, Clone)]
pub struct Registry {
    api: String,
    user_agent: String,
}

impl Registry {
    /// `user_agent` is how the application names itself to the registry:
    /// `name/version`.
    pub fn new(user_agent: impl Into<String>) -> Self {
        Registry {
            api: API.to_string(),
            user_agent: user_agent.into(),
        }
    }

    /// Asks somewhere else than `api.zed.dev`: a stand-in, for tests.
    pub fn at(mut self, api: impl Into<String>) -> Self {
        self.api = api.into().trim_end_matches('/').to_string();
        self
    }

    fn http(&self) -> Result<reqwest::blocking::Client, RegistryError> {
        Ok(reqwest::blocking::Client::builder()
            .user_agent(&self.user_agent)
            .timeout(Duration::from_secs(30))
            .build()?)
    }

    /// Every extension that provides themes, most downloaded first — the
    /// order the registry answers in, and the only measure of quality there
    /// is.
    pub fn list(&self) -> Result<Vec<Extension>, RegistryError> {
        let answer = self
            .http()?
            .get(format!("{}/extensions", self.api))
            // What the editor sends: without a schema version the registry
            // answers with extensions in a packaging this cannot read.
            .query(&[("max_schema_version", "1"), ("provides", "themes")])
            .send()?
            .error_for_status()?
            .bytes()?;
        listed(&answer)
    }

    /// The theme files of one extension, latest version.
    pub fn download(&self, id: &str) -> Result<Vec<String>, RegistryError> {
        if !valid_id(id) {
            return Err(RegistryError::BadId(id.to_string()));
        }
        let response = self
            .http()?
            .get(format!("{}/extensions/{id}/download", self.api))
            .send()?
            .error_for_status()?;
        // The length a server claims is not the length it sends, so the body
        // is counted as it arrives: one byte past the limit is enough to know.
        let mut archive = Vec::new();
        response
            .take(MAX_ARCHIVE + 1)
            .read_to_end(&mut archive)?;
        if archive.len() as u64 > MAX_ARCHIVE {
            return Err(RegistryError::TooLarge);
        }
        unpack(&archive)
    }
}

/// What of the registry's answer can be installed: an entry whose name could
/// not be a directory is left out rather than offered and refused.
fn listed(answer: &[u8]) -> Result<Vec<Extension>, RegistryError> {
    #[derive(Deserialize)]
    struct Page {
        data: Vec<Extension>,
    }
    let page: Page = serde_json::from_slice(answer)?;
    Ok(page
        .data
        .into_iter()
        .filter(|extension| valid_id(&extension.id))
        .collect())
}

/// The `themes/*.json` of a gzipped tarball.
fn unpack(archive: &[u8]) -> Result<Vec<String>, RegistryError> {
    let mut files = Vec::new();
    let mut tarball = tar::Archive::new(flate2::read::GzDecoder::new(archive));
    for entry in tarball.entries()? {
        let entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry.path()?.into_owned();
        let in_themes = path
            .parent()
            .and_then(|parent| parent.file_name())
            .is_some_and(|directory| directory == "themes");
        let is_json = path
            .extension()
            .is_some_and(|extension| extension == "json");
        if !in_themes || !is_json {
            continue;
        }
        if entry.size() > MAX_THEME || files.len() == MAX_THEMES {
            return Err(RegistryError::TooLarge);
        }
        let mut text = String::new();
        // `take`, because the size in a header is a claim like any other.
        entry.take(MAX_THEME).read_to_string(&mut text)?;
        files.push(text);
    }
    if files.is_empty() {
        return Err(RegistryError::NoThemes);
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tarball(entries: &[(&str, &str)]) -> Vec<u8> {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut builder = tar::Builder::new(encoder);
        for (path, text) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(text.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, path, text.as_bytes())
                .unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn only_the_theme_files_come_out_of_an_extension() {
        let archive = tarball(&[
            ("./extension.toml", "id = \"x\""),
            ("./themes/night.json", "{\"a\": 1}"),
            ("./themes/README.md", "hello"),
            ("./src/other.json", "{}"),
            ("./themes/day.json", "{\"b\": 2}"),
        ]);
        let files = unpack(&archive).unwrap();
        assert_eq!(files, ["{\"a\": 1}", "{\"b\": 2}"]);
    }

    #[test]
    fn an_extension_without_themes_is_an_error() {
        let archive = tarball(&[("./extension.toml", "id = \"x\"")]);
        assert!(matches!(unpack(&archive), Err(RegistryError::NoThemes)));
    }

    #[test]
    fn what_is_not_an_archive_is_an_error_and_not_a_panic() {
        assert!(matches!(
            unpack(b"<html>not found</html>"),
            Err(RegistryError::Archive(_))
        ));
    }

    #[test]
    fn an_extension_name_cannot_leave_its_directory() {
        assert!(valid_id("catppuccin"));
        assert!(valid_id("tokyo-night_2"));
        for id in ["", "..", "a/b", "../x", "a.b", "Caps", "a b", "/etc"] {
            assert!(!valid_id(id), "{id:?}");
        }
        let registry = Registry::new("test/0");
        assert!(matches!(
            registry.download("../x"),
            Err(RegistryError::BadId(_))
        ));
    }

    #[test]
    fn the_registry_answer_is_read_and_unusable_names_are_dropped() {
        let extensions = listed(
            br#"{"data": [
                {"id": "catppuccin", "name": "Catppuccin", "version": "0.2.27",
                 "description": "Soothing pastel theme", "authors": ["Catppuccin <r@c.com>"],
                 "provides": ["themes"], "download_count": 1163124},
                {"id": "../evil", "name": "Evil"},
                {"id": "bare"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(extensions.len(), 2);
        assert_eq!(extensions[0].title(), "Catppuccin");
        assert_eq!(extensions[0].author(), Some("Catppuccin"));
        assert_eq!(extensions[0].download_count(), 1163124);
        assert_eq!(extensions[1].title(), "bare");
        assert_eq!(extensions[1].author(), None);
    }

    #[test]
    fn an_answer_that_is_not_the_registry_s_is_an_error() {
        assert!(matches!(
            listed(b"<html>"),
            Err(RegistryError::Answer(_))
        ));
    }

    #[test]
    fn a_search_looks_at_the_name_the_id_and_the_description() {
        let extension = Extension {
            id: "tokyo-night".into(),
            name: "Tokyo Night Themes".into(),
            description: Some("A clean, dark theme".into()),
            authors: vec![],
            download_count: 0,
        };
        assert!(extension.matches(""));
        assert!(extension.matches("tokyo night"));
        assert!(extension.matches("tokyo-night"));
        assert!(extension.matches("clean"));
        assert!(!extension.matches("gruvbox"));
    }

    #[test]
    fn another_address_is_asked_without_a_doubled_slash() {
        let registry = Registry::new("test/0").at("http://127.0.0.1:1/zed/");
        assert_eq!(registry.api, "http://127.0.0.1:1/zed");
    }
}
