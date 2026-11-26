//! XDG path resolution and the small amount of shell-like expansion the
//! provider protocol supports.
//!
//! The library never reads or writes hub-owned state. Theme files, provider
//! manifests, and runtime assets live in user-owned XDG locations.

use std::env;
use std::path::{Path, PathBuf};

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

pub fn config_home() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

pub fn data_home() -> PathBuf {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".local/share")))
        .unwrap_or_else(|| PathBuf::from(".local/share"))
}

pub fn state_home() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".local/state")))
        .unwrap_or_else(|| PathBuf::from(".local/state"))
}

pub fn data_dirs() -> Vec<PathBuf> {
    let raw = env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    env::split_paths(&raw).collect()
}

/// Data directories used by system-wide fallbacks.
///
/// `XDG_DATA_DIRS` overrides the search path, but the FHS location must keep
/// working even when a session sets a narrower list. `/usr/share` is appended
/// once when it is missing, so packaged layouts are still found from `/usr`.
pub fn data_dirs_with_system_fallback() -> Vec<PathBuf> {
    let mut dirs = data_dirs();
    let system = PathBuf::from("/usr/share");
    if !dirs.iter().any(|dir| dir == &system) {
        dirs.push(system);
    }
    dirs
}

/// Candidate app overrides and packaged layouts, in precedence order:
///
/// 1. `$XDG_CONFIG_HOME/rofi/<app>.rasi` (full app override)
/// 2. each `$XDG_DATA_DIRS/rofi/<app>.rasi` (packaged fallback)
pub fn rofi_theme_candidates(app: &str) -> Vec<PathBuf> {
    rofi_theme_candidates_from(app, &config_home(), &data_dirs_with_system_fallback())
}

pub fn rofi_theme_candidates_from(
    app: &str,
    config_home: &Path,
    data_dirs: &[PathBuf],
) -> Vec<PathBuf> {
    let name = format!("{app}.rasi");
    let mut candidates = vec![config_home.join("rofi").join(&name)];
    candidates.extend(data_dirs.iter().map(|dir| dir.join("rofi").join(&name)));
    candidates
}

/// The user/theme-module base theme. App layouts layer on top of this file.
pub fn base_theme_path() -> PathBuf {
    config_home().join("rofi/theme.rasi")
}

/// The full global theme override accepted by Rofi itself.
pub fn global_theme_override() -> Option<PathBuf> {
    let raw = env::var_os("ROFI_THEME")?;
    let value = raw.to_string_lossy();
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let path = expand_tilde(value);
    path.is_file().then_some(path)
}

/// Provider manifests are read from the user data dir first, then each
/// system data dir. Both the current `rofi-apps` registry and the legacy
/// `rofi-hub` location are consulted during the migration window.
pub fn provider_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    let home_base = data_home();
    dirs.push(home_base.join("rofi-apps/providers"));
    dirs.push(home_base.join("rofi-hub/providers"));
    for base in data_dirs() {
        dirs.push(base.join("rofi-apps/providers"));
        dirs.push(base.join("rofi-hub/providers"));
    }

    dirs
}

/// Directory for generated runtime assets. Falls back to the state dir when
/// the session has no runtime directory.
pub fn runtime_dir() -> PathBuf {
    if let Some(path) = env::var_os("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(path);
        if !path.as_os_str().is_empty() {
            return path;
        }
    }
    state_home()
}

/// Runtime scratch directory shared by the apps (`$XDG_RUNTIME_DIR/rofi`).
pub fn rofi_runtime_dir() -> PathBuf {
    runtime_dir().join("rofi")
}

/// Expand only a leading `~` or `~/`. The provider protocol is intentionally
/// not a shell, so this is the only expansion performed.
pub fn expand_tilde(value: &str) -> PathBuf {
    if value == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(value));
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return home_dir()
            .map(|home| home.join(rest))
            .unwrap_or_else(|| PathBuf::from(value));
    }
    PathBuf::from(value)
}

pub fn is_readable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilde_expansion_is_limited_to_the_prefix() {
        let expanded = expand_tilde("~/apps/hub");
        assert!(expanded.ends_with("apps/hub"));
        assert_eq!(expand_tilde("/tmp/~tilde"), PathBuf::from("/tmp/~tilde"));
    }

    #[test]
    fn theme_candidates_prefer_user_config_then_data_dirs() {
        let candidates = rofi_theme_candidates_from(
            "clipboard",
            Path::new("/home/user/.config"),
            &[
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ],
        );
        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/home/user/.config/rofi/clipboard.rasi"),
                PathBuf::from("/usr/local/share/rofi/clipboard.rasi"),
                PathBuf::from("/usr/share/rofi/clipboard.rasi"),
            ]
        );
    }

    #[test]
    fn base_theme_is_user_owned() {
        assert!(base_theme_path().ends_with("rofi/theme.rasi"));
    }

    #[test]
    fn provider_dirs_include_legacy_and_current_registry() {
        let dirs = provider_dirs();
        assert!(
            dirs.iter()
                .any(|path| path.ends_with("rofi-apps/providers"))
        );
        assert!(dirs.iter().any(|path| path.ends_with("rofi-hub/providers")));
    }
}
