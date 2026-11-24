//! XDG path resolution and the small amount of shell-like expansion the
//! provider protocol supports.

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
/// once when it is missing, so themes and providers installed by a distro
/// package are still found from `/usr`.
pub fn data_dirs_with_system_fallback() -> Vec<PathBuf> {
    let mut dirs = data_dirs();
    let system = PathBuf::from("/usr/share");
    if !dirs.iter().any(|dir| dir == &system) {
        dirs.push(system);
    }
    dirs
}

/// Candidate theme files for one app, in precedence order after the
/// environment overrides:
///
/// 1. `$XDG_CONFIG_HOME/rofi/<app>.rasi`
/// 2. each `$XDG_DATA_DIRS/rofi/<app>.rasi`
/// 3. `/usr/share/rofi/<app>.rasi` when not already covered
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

/// Installed fallback themes for the hub menu itself.
pub fn hub_theme_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![data_home().join("rofi-hub/themes/hub.rasi")];
    candidates.extend(
        data_dirs_with_system_fallback()
            .into_iter()
            .map(|dir| dir.join("rofi-hub/themes/hub.rasi")),
    );
    candidates
}

/// Installed Screen Time theme fallbacks.
pub fn screen_time_theme_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![
        data_home().join("screen-time/rofi/screen-time.rasi"),
        config_home().join("screen-time/rofi/screen-time.rasi"),
    ];
    candidates.extend(
        data_dirs_with_system_fallback()
            .into_iter()
            .map(|dir| dir.join("screen-time/rofi/screen-time.rasi")),
    );
    candidates
}

/// Provider manifests are read from the development override first, then the
/// user data dir, then each system data dir. First match wins.
pub fn provider_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(raw) = env::var_os("ROFI_HUB_PROVIDER_DIR") {
        dirs.extend(env::split_paths(&raw));
    }

    dirs.push(data_home().join("rofi-hub/providers"));
    dirs.extend(
        data_dirs()
            .into_iter()
            .map(|dir| dir.join("rofi-hub/providers")),
    );

    dirs
}

pub fn hub_config_path() -> PathBuf {
    config_home().join("rofi-hub/config.toml")
}

/// The last theme chosen from the hub menu. Kept separate from config.toml so
/// a shortcut never rewrites a user-edited configuration file.
pub fn hub_theme_state_path() -> PathBuf {
    config_home().join("rofi-hub/theme")
}

pub fn hub_state_dir() -> PathBuf {
    state_home().join("rofi-hub")
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
    hub_state_dir()
}

pub fn hub_runtime_dir() -> PathBuf {
    runtime_dir().join("rofi-hub")
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
    fn system_fallback_is_last_resort_for_installed_themes() {
        let candidates = rofi_theme_candidates("calendar");
        let system = PathBuf::from("/usr/share/rofi/calendar.rasi");
        assert!(candidates.contains(&system));
    }
}
