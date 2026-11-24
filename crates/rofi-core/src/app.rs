//! Small helpers shared by the built-in Rofi apps.
//!
//! The apps honor the same environment as the hub: `ROFI_CONFIG`,
//! `ROFI_THEME`, and the `ROFI_HUB_THEME_FILE` passed to a provider. An
//! app-specific `~/.config/rofi/<app>.rasi` remains the fallback for direct
//! launches from a keybind.

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::{DmenuOptions, paths};

/// Resolve the Rofi config: an explicit `ROFI_CONFIG`, then the standard user
/// config. A missing or unreadable override is ignored, matching the hub.
pub fn config_path() -> Option<PathBuf> {
    if let Some(raw) = env::var_os("ROFI_CONFIG") {
        let path = paths::expand_tilde(&raw.to_string_lossy());
        if path.is_file() {
            return Some(path);
        }
    }

    let path = paths::config_home().join("rofi/config.rasi");
    path.is_file().then_some(path)
}

/// Resolve the theme for one app. Precedence:
///
/// 1. `ROFI_THEME`
/// 2. `ROFI_HUB_THEME_FILE` (the hub's generated theme for this launch)
/// 3. `$XDG_CONFIG_HOME/rofi/<app>.rasi`
/// 4. each `$XDG_DATA_DIRS/rofi/<app>.rasi`, then `/usr/share/rofi/<app>.rasi`
///
/// Step 4 is what makes a distro package's fallback theme work without a
/// per-user copy.
pub fn theme_path(app: &str) -> Option<PathBuf> {
    for variable in ["ROFI_THEME", "ROFI_HUB_THEME_FILE"] {
        let Some(raw) = env::var_os(variable) else {
            continue;
        };
        let path = paths::expand_tilde(&raw.to_string_lossy());
        if path.is_file() {
            return Some(path);
        }
    }

    if let Some(path) = paths::rofi_theme_candidates(app)
        .into_iter()
        .find(|path| path.is_file())
    {
        return Some(path);
    }

    // Direct launches (keybinds, Waybar, a terminal) should still get the
    // same flat chrome as the hub and Screen Time. `prepare` honors
    // ROFI_THEME, the user's rofi/theme.rasi, and the packaged
    // rofi-hub/themes/hub.rasi before generating a runtime copy.
    let mode = crate::theme::active_mode();
    let palette = crate::Theme::for_mode(mode);
    crate::theme::prepare(mode, &palette)
}

/// A private PID file under the hub runtime directory. The directory is
/// created with user-only permissions; apps never share Rofi's default PID
/// file with unrelated prompts.
pub fn pid_path(app: &str) -> Option<PathBuf> {
    let dir = paths::hub_runtime_dir();
    fs::create_dir_all(&dir).ok()?;
    let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    Some(dir.join(format!("rofi-{app}.pid")))
}

/// Build dmenu options with the shared config, theme, and private PID file.
pub fn dmenu_options(app: &str, prompt: impl Into<String>) -> DmenuOptions {
    let mut options = DmenuOptions::new(prompt);
    options.config = config_path();
    options.theme = theme_path(app);
    options.pid = pid_path(app);
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_files_are_private_to_the_app() {
        let path = pid_path("clipboard").expect("runtime dir");
        assert!(path.ends_with("rofi-clipboard.pid"));
    }
}
