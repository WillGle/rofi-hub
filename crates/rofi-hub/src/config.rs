//! Optional user configuration at `~/.config/rofi-hub/config.toml`.

use std::fs;

use rofi_core::paths;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: Ui,
    pub theme: ThemeSection,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Ui {
    /// Prompt shown above the list.
    pub prompt: Option<String>,
    /// Font family for the generated theme.
    pub font: Option<String>,
    /// Font size for the generated theme (clamped to 6..=72).
    pub font_size: Option<u8>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ThemeSection {
    /// `auto`, `nord`, `solarized`, `gruvbox`, or a path to a `.rasi` file.
    pub mode: Option<String>,
}

impl Config {
    /// Load the config without ever failing the launcher. A malformed file is
    /// reported to stderr and the built-in defaults are used.
    pub fn load() -> (Self, Vec<String>) {
        let path = paths::hub_config_path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return (Self::default(), Vec::new());
            }
            Err(error) => {
                return (
                    Self::default(),
                    vec![format!("cannot read {}: {error}", path.display())],
                );
            }
        };
        match toml::from_str(&text) {
            Ok(config) => (config, Vec::new()),
            Err(error) => (
                Self::default(),
                vec![format!("ignoring {}: {error}", path.display())],
            ),
        }
    }

    pub fn prompt(&self) -> &str {
        self.ui
            .prompt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("Rofi Hub")
    }

    pub fn font_family(&self) -> Option<&str> {
        self.ui
            .font
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty() && !value.chars().any(char::is_control))
    }

    pub fn font_size(&self) -> Option<u8> {
        self.ui.font_size.filter(|size| (6..=72).contains(size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe() {
        let config = Config::default();
        assert_eq!(config.prompt(), "Rofi Hub");
        assert_eq!(config.font_family(), None);
        assert_eq!(config.font_size(), None);
    }

    #[test]
    fn rejects_clamped_font_and_blank_prompt() {
        let config: Config = toml::from_str(
            r#"
[ui]
prompt = "   "
font = "  "
font_size = 200
"#,
        )
        .unwrap();
        assert_eq!(config.prompt(), "Rofi Hub");
        assert_eq!(config.font_family(), None);
        assert_eq!(config.font_size(), None);
    }

    #[test]
    fn parses_a_full_file() {
        let config: Config = toml::from_str(
            r#"
[ui]
prompt = "Applications"
font = "Iosevka"
font_size = 14

[theme]
mode = "nord"
"#,
        )
        .unwrap();
        assert_eq!(config.prompt(), "Applications");
        assert_eq!(config.font_family(), Some("Iosevka"));
        assert_eq!(config.font_size(), Some(14));
        assert_eq!(config.theme.mode.as_deref(), Some("nord"));
    }
}
