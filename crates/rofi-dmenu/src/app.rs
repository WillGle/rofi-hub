//! Small helpers shared by the standalone Rofi apps.
//!
//! The apps honor `ROFI_CONFIG` and `ROFI_THEME`, and resolve an app-specific
//! `~/.config/rofi/<app>.rasi` override before the user's base theme and the
//! packaged fallback. Apps that offer `Ctrl+1–4` also own a persistent theme
//! lock under `$XDG_STATE_HOME/rofi-apps/<app>/theme-choice`. No hub state or
//! hub-owned environment variable is consulted.

use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::{DmenuOptions, Mode, Theme, paths};

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

// The palette order matches Screen Time. Auto restores the normal theme resolution.
const THEME_SHORTCUTS: [Mode; 4] = [Mode::Nord, Mode::Solarized, Mode::Gruvbox, Mode::Auto];

pub fn bind_theme_shortcuts(options: &mut DmenuOptions, first: u8) {
    for index in 0..THEME_SHORTCUTS.len() {
        options.extra_args.extend([
            format!("-kb-custom-{}", usize::from(first) + index + 1),
            format!("Control+{}", index + 1),
        ]);
    }
}

/// App-owned theme lock state. This is deliberately separate from the menu,
/// Screen Time, the installer, and the shared runtime theme cache.
pub fn theme_choice_path(app: &str) -> PathBuf {
    paths::state_home()
        .join("rofi-apps")
        .join(app)
        .join("theme-choice")
}

/// The persisted palette mode for one app. `Auto` is represented by no file,
/// because it means "use the configured theme".
pub fn saved_theme_mode(app: &str) -> Option<Mode> {
    let text = fs::read_to_string(theme_choice_path(app)).ok()?;
    text.trim()
        .parse::<Mode>()
        .ok()
        .filter(|mode| *mode != Mode::Auto)
}

/// Persist or clear the app's theme lock. `Auto` removes the lock so the
/// configured theme and `Ctrl+4` behave the same on the next launch.
pub fn save_theme_mode(app: &str, mode: Mode) -> io::Result<()> {
    let path = theme_choice_path(app);
    if mode == Mode::Auto {
        return match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        };
    }
    let Some(dir) = path.parent() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "theme choice path has no parent",
        ));
    };
    fs::create_dir_all(dir)?;
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    let temp = dir.join(format!(".theme-choice.{}", std::process::id()));
    fs::write(&temp, format!("{}\n", mode.as_str()))?;
    let _ = fs::set_permissions(&temp, fs::Permissions::from_mode(0o600));
    fs::rename(&temp, &path)
}

/// Resolve a palette and its Rofi theme file. Non-auto modes layer the palette
/// over the app's configured theme so fonts, geometry, and spacing stay intact.
pub fn theme_for_mode(app: &str, mode: Mode) -> (Theme, Option<PathBuf>) {
    let theme = Theme::resolve(mode);
    let base = theme_path(app);
    let path = if mode == Mode::Auto {
        base
    } else {
        let cache_name = format!("{app}-{}", mode.as_str());
        base.as_deref()
            .and_then(|base| crate::theme::write_palette_theme(&cache_name, base, &theme))
            .or_else(|| crate::theme::generate_theme(app, &theme))
    };
    (theme, path)
}

/// Initial state for an app that supports a persistent theme lock.
pub fn initial_theme(app: &str) -> (Theme, Option<PathBuf>) {
    saved_theme_mode(app)
        .map(|mode| theme_for_mode(app, mode))
        .unwrap_or_else(|| (Theme::active(), theme_path(app)))
}

/// Apply and persist a `Ctrl+1–4` choice. A save failure must not prevent the
/// in-session palette change.
pub fn session_theme(app: &str, binding: u8, first: u8) -> Option<(Theme, Option<PathBuf>)> {
    let mode = *THEME_SHORTCUTS.get(usize::from(binding.checked_sub(first)?))?;
    let _ = save_theme_mode(app, mode);
    Some(theme_for_mode(app, mode))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_files_are_private_to_the_app() {
        let path = pid_path("clipboard").expect("runtime dir");
        assert!(path.ends_with("rofi-clipboard.pid"));
    }

    #[test]
    fn theme_lock_state_is_app_owned() {
        assert!(theme_choice_path("calendar").ends_with("rofi-apps/calendar/theme-choice"));
        assert_eq!(
            theme_choice_path("network"),
            paths::state_home()
                .join("rofi-apps")
                .join("network")
                .join("theme-choice")
        );
    }

    #[test]
    fn shortcut_order_maps_to_the_documented_palettes() {
        assert_eq!(THEME_SHORTCUTS[0], Mode::Nord);
        assert_eq!(THEME_SHORTCUTS[1], Mode::Solarized);
        assert_eq!(THEME_SHORTCUTS[2], Mode::Gruvbox);
        assert_eq!(THEME_SHORTCUTS[3], Mode::Auto);
    }
}
