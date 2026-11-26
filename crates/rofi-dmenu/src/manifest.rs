//! Provider manifests: the contract between the hub and a sub-app.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{Error, Result, paths};

/// A sub-app the hub can launch.
///
/// The manifest is intentionally declarative. The hub never evaluates a shell
/// string: `exec` is an argument vector.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProviderManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(alias = "command")]
    pub exec: Vec<String>,
    #[serde(default)]
    pub version: Option<String>,
    /// Where the manifest was loaded from; not part of the file format.
    #[serde(skip)]
    pub path: Option<std::path::PathBuf>,
}

impl ProviderManifest {
    pub fn from_path(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).map_err(|source| Error::ManifestRead {
            path: path.to_path_buf(),
            source,
        })?;
        let mut manifest: Self = toml::from_str(&text).map_err(|source| Error::ManifestParse {
            path: path.to_path_buf(),
            source,
        })?;
        manifest.path = Some(path.to_path_buf());
        manifest.validate(path)?;
        Ok(manifest)
    }

    fn validate(&self, path: &Path) -> Result<()> {
        if !valid_id(&self.id) {
            return Err(Error::InvalidManifest {
                path: path.to_path_buf(),
                reason: format!(
                    "id `{}` must use lowercase letters, digits, '-' or '_'",
                    self.id
                ),
            });
        }
        if self.name.trim().is_empty() {
            return Err(Error::InvalidManifest {
                path: path.to_path_buf(),
                reason: "name is required".to_owned(),
            });
        }
        if self.exec.is_empty() || self.exec[0].trim().is_empty() {
            return Err(Error::InvalidManifest {
                path: path.to_path_buf(),
                reason: "exec must contain at least one program".to_owned(),
            });
        }
        if self.exec.iter().any(|arg| arg.contains('\0')) {
            return Err(Error::InvalidManifest {
                path: path.to_path_buf(),
                reason: "exec arguments must not contain NUL".to_owned(),
            });
        }
        Ok(())
    }

    pub fn category(&self) -> &str {
        self.category.as_deref().unwrap_or("Apps")
    }
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && !id.starts_with('-')
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
}

/// Every manifest found in the search path, plus recoverable load errors.
#[derive(Debug, Default)]
pub struct ManifestSet {
    providers: Vec<ProviderManifest>,
    warnings: Vec<String>,
}

impl ManifestSet {
    /// Load manifests from the standard XDG search path.
    pub fn discover() -> Self {
        Self::from_dirs(&paths::provider_dirs())
    }

    /// Load manifests from explicit directories. Earlier directories win when
    /// two providers share an id.
    pub fn from_dirs(dirs: &[PathBuf]) -> Self {
        let mut set = Self::default();
        let mut seen = HashSet::new();

        for dir in dirs {
            let entries = match fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    set.warnings
                        .push(format!("cannot read {}: {error}", dir.display()));
                    continue;
                }
            };

            let mut paths: Vec<PathBuf> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
                .collect();
            paths.sort();

            for path in paths {
                match ProviderManifest::from_path(&path) {
                    Ok(manifest) => {
                        if !seen.insert(manifest.id.clone()) {
                            set.warnings.push(format!(
                                "ignoring duplicate provider `{}` from {}",
                                manifest.id,
                                path.display()
                            ));
                        } else {
                            set.providers.push(manifest);
                        }
                    }
                    Err(error) => set.warnings.push(error.to_string()),
                }
            }
        }

        set.providers.sort_by(|a, b| {
            a.category()
                .to_lowercase()
                .cmp(&b.category().to_lowercase())
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.id.cmp(&b.id))
        });
        set
    }

    pub fn providers(&self) -> &[ProviderManifest] {
        &self.providers
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    pub fn find(&self, id: &str) -> Option<&ProviderManifest> {
        self.providers.iter().find(|provider| provider.id == id)
    }

    pub fn into_parts(self) -> (Vec<ProviderManifest>, Vec<String>) {
        (self.providers, self.warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(dir: &Path, name: &str, text: &str) {
        fs::write(dir.join(name), text).unwrap();
    }

    #[test]
    fn valid_ids_are_strict() {
        assert!(valid_id("screen-time"));
        assert!(valid_id("clipboard_2"));
        assert!(!valid_id("Screen-Time"));
        assert!(!valid_id("-leading"));
        assert!(!valid_id("with/slash"));
    }

    #[test]
    fn loads_and_sorts_manifests() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "b.toml",
            r#"
id = "beta"
name = "Beta"
category = "Tools"
exec = ["beta-ui"]
"#,
        );
        write(
            dir.path(),
            "a.toml",
            r#"
id = "alpha"
name = "Alpha"
category = "Tools"
description = "First"
exec = ["alpha", "--open"]
"#,
        );

        let set = ManifestSet::from_dirs(&[dir.path().to_path_buf()]);
        assert_eq!(set.providers().len(), 2);
        assert_eq!(set.providers()[0].id, "alpha");
        assert_eq!(set.providers()[1].id, "beta");
        assert!(set.warnings().is_empty());
        assert_eq!(set.find("alpha").unwrap().exec.len(), 2);
    }

    #[test]
    fn skips_bad_manifests_without_failing_the_hub() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "ok.toml",
            r#"
id = "ok"
name = "OK"
exec = ["ok"]
"#,
        );
        write(
            dir.path(),
            "bad.toml",
            "id = 'Bad Id'\nname = 'x'\nexec = ['x']\n",
        );
        write(dir.path(), "broken.toml", "not = [valid");

        let set = ManifestSet::from_dirs(&[dir.path().to_path_buf()]);
        assert_eq!(set.providers().len(), 1);
        assert_eq!(set.warnings().len(), 2);
    }

    #[test]
    fn earlier_directories_win_duplicates() {
        let first = tempdir().unwrap();
        let second = tempdir().unwrap();
        write(
            first.path(),
            "x.toml",
            "id = 'same'\nname = 'First'\nexec = ['first']\n",
        );
        write(
            second.path(),
            "x.toml",
            "id = 'same'\nname = 'Second'\nexec = ['second']\n",
        );

        let set =
            ManifestSet::from_dirs(&[first.path().to_path_buf(), second.path().to_path_buf()]);
        assert_eq!(set.providers().len(), 1);
        assert_eq!(set.providers()[0].name, "First");
        assert_eq!(set.warnings().len(), 1);
    }
}
