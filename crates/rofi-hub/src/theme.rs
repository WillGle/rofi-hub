//! Theme resolution and persistence for the hub menu.
//!
//! Precedence: `--theme`, `ROFI_HUB_THEME`, the last menu choice, `config.toml`,
//! then `auto`.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rofi_core::{Mode, Theme, paths};

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Cli,
    Env,
    Saved,
    Config,
    Default,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Cli => "--theme",
            Source::Env => "ROFI_HUB_THEME",
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
    } else if let Ok(value) = std::env::var("ROFI_HUB_THEME") {
        if value.trim().is_empty() {
            (None, Source::Default)
        } else {
            (Some(value), Source::Env)
        }
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

    let mut theme = Theme::for_mode(mode);
    if let Some(font) = config.font_family() {
        theme.font_family = font.to_owned();
    }
    if let Some(size) = config.font_size() {
        theme.font_size = size;
    }

    let path = match &custom_path {
        Some(path) => Some(path.clone()),
        None => rofi_core::theme::prepare(mode, &theme),
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
    let text = fs::read_to_string(paths::hub_theme_state_path()).ok()?;
    text.trim().parse().ok()
}

/// Parse a mode name (not a path) for `rofi-hub theme set`.
pub fn parse_mode(value: &str) -> Result<Mode, String> {
    value
        .trim()
        .parse()
        .map_err(|_| format!("unknown theme `{}`", value.trim()))
}

/// Persist the menu choice without touching `config.toml`.
pub fn save_mode(mode: Mode) -> Result<()> {
    let path = paths::hub_theme_state_path();
    let dir = path
        .parent()
        .context("theme state path has no parent directory")?;
    fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));

    let temp = dir.join(format!(".theme.{}", std::process::id()));
    {
        let mut file =
            fs::File::create(&temp).with_context(|| format!("cannot write {}", temp.display()))?;
        let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
        writeln!(file, "{}", mode.as_str())?;
        file.sync_all().ok();
    }
    fs::rename(&temp, &path).with_context(|| format!("cannot replace {}", path.display()))?;
    Ok(())
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
    fn state_path_is_under_config_home() {
        assert!(paths::hub_theme_state_path().ends_with("rofi-hub/theme"));
    }
}
