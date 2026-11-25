//! Optional user configuration at `~/.config/rofi-hub/config.toml`.

use std::fs;

use rofi_core::paths;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: Ui,
    pub theme: ThemeSection,
    pub launcher: Launcher,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Launcher {
    /// Number of columns in `rofi-hub launcher`; clamped to 1..=6.
    pub columns: Option<u32>,
    /// Number of visible rows in `rofi-hub launcher`; clamped to 1..=40.
    pub lines: Option<u32>,
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

    pub fn launcher_columns(&self) -> Option<u32> {
        self.launcher
            .columns
            .filter(|columns| (1..=6).contains(columns))
    }

    pub fn launcher_lines(&self) -> Option<u32> {
        self.launcher.lines.filter(|lines| (1..=40).contains(lines))
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
        assert_eq!(config.launcher_columns(), None);
        assert_eq!(config.launcher_lines(), None);
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
    fn parses_and_clamps_launcher_columns() {
        let config: Config = toml::from_str("[launcher]\ncolumns = 3\n").unwrap();
        assert_eq!(config.launcher_columns(), Some(3));

        let config: Config = toml::from_str("[launcher]\ncolumns = 0\n").unwrap();
        assert_eq!(config.launcher_columns(), None);
        assert_eq!(config.launcher_lines(), None);

        let config: Config = toml::from_str("[launcher]\ncolumns = 9\n").unwrap();
        assert_eq!(config.launcher_columns(), None);
        assert_eq!(config.launcher_lines(), None);
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

[launcher]
columns = 3
lines = 10
"#,
        )
        .unwrap();
        assert_eq!(config.prompt(), "Applications");
        assert_eq!(config.font_family(), Some("Iosevka"));
        assert_eq!(config.font_size(), Some(14));
        assert_eq!(config.theme.mode.as_deref(), Some("nord"));
        assert_eq!(config.launcher_columns(), Some(3));
        assert_eq!(config.launcher_lines(), Some(10));
    }
}
