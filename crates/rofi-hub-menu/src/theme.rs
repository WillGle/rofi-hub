//! Theme resolution and persistence for the optional menu.
//!
//! Precedence: `--theme`, the last menu choice, `config.toml`, then `auto`.
//! The menu owns this state; apps never read it and the menu never injects it
//! into app processes.

use std::path::PathBuf;

use anyhow::{Context, Result};
use rofi_dmenu::{Mode, Theme, paths};

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Cli,
    Saved,
    Config,
    Default,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Cli => "--theme",
            Source::Saved => "saved choice",
            Source::Config => "config.toml",
            Source::Default => "default",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Choice {
    pub mode: Mode,
    pub theme: Theme,
    /// Theme file passed to Rofi, when one could be prepared.
    pub path: Option<PathBuf>,
    pub source: Source,
    /// True when the user pointed at their own `.rasi` file.
    pub custom_path: bool,
}

impl Choice {
    pub fn label(&self) -> String {
        if self.custom_path {
            self.path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| self.mode.label().to_owned())
        } else {
            self.mode.label().to_owned()
        }
    }
}

enum Raw {
    Mode(Mode),
    Path(PathBuf),
}

/// The optional menu owns its own theme state.
pub fn state_path() -> PathBuf {
    paths::config_home().join("rofi-hub-menu/theme")
}

fn classify(value: &str) -> Result<Raw, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("theme value is empty".to_owned());
    }
    let looks_like_path = value.contains('/') || value.starts_with('~') || value.ends_with(".rasi");
    if looks_like_path {
        return Ok(Raw::Path(paths::expand_tilde(value)));
    }
    value
        .parse::<Mode>()
        .map(Raw::Mode)
        .map_err(|_| format!("unknown theme `{value}`"))
}

pub fn resolve(cli: Option<&str>, config: &Config) -> Result<Choice> {
    let (raw, source) = if let Some(value) = cli {
        (Some(value.to_owned()), Source::Cli)
    } else if let Some(mode) = saved_mode() {
        (Some(mode.as_str().to_owned()), Source::Saved)
    } else if let Some(value) = &config.theme.mode {
        (Some(value.clone()), Source::Config)
    } else {
        (None, Source::Default)
    };

    let (mode, custom_path) = match raw.as_deref() {
        Some(value) => match classify(value) {
            Ok(Raw::Mode(mode)) => (mode, None),
            Ok(Raw::Path(path)) => {
                if !path.is_file() {
                    if source == Source::Cli {
                        anyhow::bail!("theme file not found: {}", path.display());
                    }
                    (Mode::Auto, None)
                } else {
                    (Mode::Auto, Some(path))
                }
            }
            Err(error) if source == Source::Cli => anyhow::bail!("{error}"),
            Err(_) => (Mode::Auto, None),
        },
        None => (Mode::Auto, None),
    };

    let mut theme = Theme::resolve(mode);
    if let Some(font) = config.font_family() {
        theme.font_family = font.to_owned();
    }
    if let Some(size) = config.font_size() {
        theme.font_size = size;
    }

    let path = match &custom_path {
        Some(path) if mode != Mode::Auto => {
            rofi_dmenu::theme::write_palette_theme("hub", path, &theme).or(Some(path.clone()))
        }
        Some(path) => Some(path.clone()),
        None => {
            let base = rofi_dmenu::theme::resolve_theme_file_with("hub-menu", &theme);
            match (mode, base) {
                (Mode::Auto, base) => base,
                (_, Some(base)) => {
                    rofi_dmenu::theme::write_palette_theme("hub", &base, &theme).or(Some(base))
                }
                (_, None) => rofi_dmenu::theme::generate_theme("hub", &theme),
            }
        }
    };

    Ok(Choice {
        mode,
        theme,
        path,
        source,
        custom_path: custom_path.is_some(),
    })
}

pub fn saved_mode() -> Option<Mode> {
    std::fs::read_to_string(state_path())
        .ok()
        .and_then(|value| value.trim().parse().ok())
}

/// Parse a mode name (not a path) for `rofi-hub-menu theme set`.
pub fn parse_mode(value: &str) -> Result<Mode, String> {
    value
        .trim()
        .parse()
        .map_err(|_| format!("unknown theme `{}`", value.trim()))
}

/// Persist the menu choice without touching `config.toml`. `auto` is the
/// absence of a menu override, so it removes the state file and lets the
/// configured `[theme]` value resolve again.
pub fn save_mode(mode: Mode) -> Result<()> {
    let path = state_path();
    if mode == Mode::Auto {
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).with_context(|| format!("cannot remove {}", path.display())),
        };
    }
    let dir = path.parent().context("menu theme path has no parent")?;
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    let temp = dir.join(format!(".theme.{}", std::process::id()));
    std::fs::write(&temp, format!("{}\n", mode.as_str()))?;
    let _ = std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600));
    std::fs::rename(&temp, &path).context("cannot save the menu theme mode")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_modes_and_paths() {
        assert!(matches!(classify("nord"), Ok(Raw::Mode(Mode::Nord))));
        assert!(matches!(classify("/tmp/theme.rasi"), Ok(Raw::Path(_))));
        assert!(classify("sepia").is_err());
        assert!(classify(" ").is_err());
    }

    #[test]
    fn explicit_missing_path_is_reported_for_cli() {
        let config = Config::default();
        let result = resolve(Some("/definitely/missing/theme.rasi"), &config);
        assert!(result.is_err());
    }

    #[test]
    fn modes_resolve_without_a_path_override() {
        let config = Config::default();
        let choice = resolve(Some("gruvbox"), &config).unwrap();
        assert_eq!(choice.mode, Mode::Gruvbox);
        assert!(!choice.custom_path);
        assert_eq!(choice.source, Source::Cli);
    }

    #[test]
    fn config_font_is_applied() {
        let config: Config = toml::from_str(
            r#"
[ui]
font = "Iosevka"
font_size = 15
"#,
        )
        .unwrap();
        let choice = resolve(Some("nord"), &config).unwrap();
        assert_eq!(choice.theme.font_family, "Iosevka");
        assert_eq!(choice.theme.font_size, 15);
    }

    #[test]
    fn state_path_is_menu_owned() {
        assert!(state_path().ends_with("rofi-hub-menu/theme"));
    }
}
