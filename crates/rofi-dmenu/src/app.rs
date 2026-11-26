//! Small helpers shared by the standalone Rofi apps.
//!
//! The apps honor `ROFI_CONFIG` and `ROFI_THEME`, and resolve an app-specific
//! `~/.config/rofi/<app>.rasi` override before the user's base theme and the
//! packaged fallback. No hub state or hub-owned environment variable is consulted.

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::{DmenuOptions, paths};

/// Resolve the Rofi config: an explicit `ROFI_CONFIG`, then the standard user
/// config. A missing or unreadable override is ignored.
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

/// Resolve the theme for one app. See [`crate::theme::resolve_theme_file`]
/// for the full precedence order.
pub fn theme_path(app: &str) -> Option<PathBuf> {
    crate::theme::resolve_theme_file(app)
}

/// A private PID file under `$XDG_RUNTIME_DIR/rofi`. The directory is created
/// with user-only permissions; apps never share Rofi's default PID file with
/// unrelated prompts.
pub fn pid_path(app: &str) -> Option<PathBuf> {
    let dir = paths::rofi_runtime_dir();
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
