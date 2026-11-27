use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use rofi_dmenu::theme::font_from_static;
use rofi_dmenu::{ChromeProfile, Theme as CoreTheme};

use crate::config::{Config, ThemeMode};

pub const DEFAULT_FAMILY: &str = "JetBrainsMono Nerd Font";
pub const DEFAULT_SIZE: u8 = 12;

pub use rofi_dmenu::theme::valid_color;

/// Screen Time keeps the Rofi chrome path next to the shared core palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub core: CoreTheme,
    pub rofi_theme: Option<PathBuf>,
}

impl Deref for Theme {
    type Target = CoreTheme;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl DerefMut for Theme {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.core
    }
}

impl Theme {
    /// Resolve the theme on every render. Never cached: a palette or font
    /// change applies on the next Rofi open, with no daemon restart.
    pub fn load() -> Self {
        Self::load_with_mode(None)
    }

    pub(crate) fn load_with_mode(mode: Option<&str>) -> Self {
        let mut config = Config::load_default().unwrap_or_default();
        // Screen Time owns its own theme choice. There is no hub state.
        if let Some(mode) = mode {
            config.theme.mode = Some(mode.to_owned());
        }
        Self::resolve(&config)
    }

    pub fn resolve(config: &Config) -> Self {
        let home = crate::config::home_dir();
        let palette = config.palette_path(&home);
        let static_env = config.static_env_path(&home);
        Self::resolve_with_paths(config, &home, &palette, &static_env)
    }

    pub fn resolve_with_paths(
        config: &Config,
        home: &Path,
        palette_path: &Path,
        static_env_path: &Path,
    ) -> Self {
        let mode = config.theme_mode();
        let mut core = match mode {
            ThemeMode::Auto | ThemeMode::Matugen => CoreTheme::from_palette_file(palette_path)
                .or_else(|| CoreTheme::from_static_env(static_env_path)),
            ThemeMode::Static => CoreTheme::from_static_env(static_env_path),
            ThemeMode::Nord => Some(CoreTheme::nord()),
            ThemeMode::Solarized => Some(CoreTheme::solarized()),
            ThemeMode::Gruvbox => Some(CoreTheme::gruvbox()),
            ThemeMode::Config => None,
        }
        .unwrap_or_else(CoreTheme::builtin);

        apply_explicit_colors(&mut core, config);

        let (mut font_family, mut font_size) = (DEFAULT_FAMILY.to_owned(), DEFAULT_SIZE);
        if let Some((family, size)) = font_from_static(static_env_path) {
            if let Some(family) = family {
                font_family = family;
            }
            if let Some(size) = size {
                font_size = size.max(DEFAULT_SIZE);
            }
        }
        if let Some(family) = &config.font.family {
            let family = family.trim();
            if !family.is_empty() && !family.chars().any(char::is_control) {
                font_family = family.to_owned();
            }
        }
        if let Some(size) = config.font.size
            && (6..=72).contains(&size)
        {
            font_size = size;
        }
        core.font_family = font_family;
        core.font_size = font_size;

        Self {
            core,
            rofi_theme: chrome_theme_path(config, home),
        }
    }

    pub fn builtin() -> Self {
        Self {
            core: CoreTheme::builtin(),
            rofi_theme: None,
        }
    }

    pub fn font(&self) -> String {
        self.core.font()
    }

    /// A complete fallback Rofi theme generated from the resolved roles. The
    /// wrapper writes this to a runtime file and passes it with `-theme`; the
    /// standalone installed theme is used only if generation fails.
    pub fn rofi_rasi(&self) -> String {
        let mut out = String::new();
        if let Some(path) = &self.rofi_theme {
            out.push_str(&format!(
                "@theme \"{}\"\n",
                escape_rasi(&path.display().to_string())
            ));
        }
        let mut profile = ChromeProfile::for_app("screen-time");
        profile.width_ch = crate::render::grid_columns(self);
        out.push_str(&self.core.rasi_with(profile));
        out.push_str(&format!(
            r#"
mainbox {{ children: [ message, listview ]; }}
inputbar {{ enabled: false; }}
element {{ children: [ element-text ]; }}
textbox {{
    font: "{} {}";
    markup: true;
    vertical-align: 0.5;
}}
"#,
            escape_rasi(&self.font_family),
            profile.header_pt,
        ));
        out
    }
}

fn apply_explicit_colors(colors: &mut CoreTheme, config: &Config) {
    let supplied = config.explicit_colors();
    let values: Vec<&str> = supplied.iter().filter_map(|value| *value).collect();
    if values.is_empty() {
        return;
    }
    if values.iter().any(|value| !valid_color(value)) {
        // Never half-apply an explicit palette.
        return;
    }
    let mut values = values.into_iter();
    let mut next = || values.next();
    if supplied[0].is_some()
        && let Some(value) = next()
    {
        colors.base = value.to_owned();
    }
    if supplied[1].is_some()
        && let Some(value) = next()
    {
        colors.mantle = value.to_owned();
    }
    if supplied[2].is_some()
        && let Some(value) = next()
    {
        colors.text = value.to_owned();
    }
    if supplied[3].is_some()
        && let Some(value) = next()
    {
        colors.muted = value.to_owned();
    }
    if supplied[4].is_some()
        && let Some(value) = next()
    {
        colors.accent = value.to_owned();
        colors.work.clone_from(&colors.accent);
    }
    if supplied[5].is_some()
        && let Some(value) = next()
    {
        colors.power = value.to_owned();
    }
    if supplied[6].is_some()
        && let Some(value) = next()
    {
        colors.other = value.to_owned();
    }
    // `other` follows `muted` unless explicitly set.
    if supplied[3].is_some() && supplied[6].is_none() {
        colors.other = colors.muted.clone();
    }
}

fn chrome_theme_path(config: &Config, home: &Path) -> Option<PathBuf> {
    if config.theme.rofi_theme.is_some() {
        let path = config.rofi_theme_path(home);
        if path.is_file() {
            return Some(path);
        }
    }
    // The user/theme-module base theme is layered under the Screen Time
    // layout. Screen Time does not read any hub theme state.
    let base = home.join(".config/rofi/theme.rasi");
    base.is_file().then_some(base)
}

fn escape_rasi(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write(path: &Path, value: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, value).unwrap();
    }

    fn palette(accent: &str) -> String {
        format!(
            r##"{{
              "colors": {{
                "surface": {{"default": {{"color": "#101010"}}}},
                "surface_container": {{"default": {{"color": "#202020"}}}},
                "on_surface": {{"default": {{"color": "#f0f0f0"}}}},
                "on_surface_variant": {{"default": {{"color": "#909090"}}}},
                "primary": {{"default": {{"color": "{accent}"}}}}
              }},
              "palettes": {{"secondary": {{"70": {{"color": "#00cccc"}}}}}}
            }}"##
        )
    }

    fn static_env() -> &'static str {
        r#"
THEME_MONO_FONT='Static Mono'
THEME_ROFI_FONT_SIZE=13
THEME_STATIC_BASE='#111111'
THEME_STATIC_MANTLE='#222222'
THEME_STATIC_TEXT='#eeeeee'
THEME_STATIC_SUBTEXT='#999999'
THEME_STATIC_ACCENT='#3333ff'
THEME_STATIC_CYAN='#33cccc'
"#
    }

    #[test]
    fn matugen_wins_over_static() {
        let dir = tempdir().unwrap();
        let palette_path = dir.path().join("palette.json");
        let static_path = dir.path().join("static.env");
        write(&palette_path, &palette("#123456"));
        write(&static_path, static_env());
        let config = Config::default();
        let theme = Theme::resolve_with_paths(&config, dir.path(), &palette_path, &static_path);
        assert_eq!(theme.base, "#101010");
        assert_ne!(theme.accent, "#123456");
        assert_eq!(theme.power, "#00cccc");
        for color in [
            &theme.work,
            &theme.browser,
            &theme.communication,
            &theme.entertainment,
            &theme.muted,
            &theme.other,
            &theme.accent,
        ] {
            assert!(
                theme
                    .contrast_ratio(color, &theme.base)
                    .is_some_and(|ratio| ratio >= 4.5),
                "{color}"
            );
            assert!(
                theme
                    .contrast_ratio(color, &theme.mantle)
                    .is_some_and(|ratio| ratio >= 4.5),
                "{color}"
            );
        }
        assert_eq!(theme.font_family, "Static Mono");
        assert_eq!(theme.font_size, 13);
    }

    #[test]
    fn config_colors_win_and_invalid_config_is_not_half_applied() {
        let dir = tempdir().unwrap();
        let palette_path = dir.path().join("palette.json");
        let static_path = dir.path().join("static.env");
        write(&palette_path, &palette("#123456"));
        write(&static_path, static_env());
        let mut config: Config = toml::from_str(
            r##"
            [theme]
            accent = "#abcdef"
            power = "#fedcba"
            [font]
            family = "Config Mono"
            size = 16
            "##,
        )
        .unwrap();
        let theme = Theme::resolve_with_paths(&config, dir.path(), &palette_path, &static_path);
        assert_eq!(theme.accent, "#abcdef");
        assert_eq!(theme.power, "#fedcba");
        assert_eq!(theme.base, "#101010");
        assert_eq!(theme.font_family, "Config Mono");
        assert_eq!(theme.font_size, 16);

        config.theme.accent = Some("not-a-color".to_owned());
        let theme = Theme::resolve_with_paths(&config, dir.path(), &palette_path, &static_path);
        assert_ne!(theme.accent, "#123456");
    }

    #[test]
    fn corrupt_palette_falls_back_to_static_then_builtin() {
        let dir = tempdir().unwrap();
        let palette_path = dir.path().join("palette.json");
        let static_path = dir.path().join("static.env");
        write(&palette_path, &palette("broken"));
        write(&static_path, static_env());
        let config = Config::default();
        let theme = Theme::resolve_with_paths(&config, dir.path(), &palette_path, &static_path);
        assert_eq!(theme.base, "#111111");
        assert_eq!(theme.accent, "#3333ff");

        let theme = Theme::resolve_with_paths(
            &config,
            dir.path(),
            &dir.path().join("missing.json"),
            &dir.path().join("missing.env"),
        );
        assert_eq!(theme.base, "#1d2021");
        assert_eq!(theme.accent, "#fabd2f");
    }

    #[test]
    fn auto_palette_without_secondary_keeps_the_desktop_accent() {
        let dir = tempdir().unwrap();
        let palette_path = dir.path().join("palette.json");
        let static_path = dir.path().join("static.env");
        write(
            &palette_path,
            r##"{
              "colors": {
                "surface": {"default": {"color": "#101410"}},
                "surface_container": {"default": {"color": "#1c211b"}},
                "surface_container_high": {"default": {"color": "#272b26"}},
                "on_surface": {"default": {"color": "#e0e4db"}},
                "on_surface_variant": {"default": {"color": "#c2c9bd"}},
                "primary": {"default": {"color": "#9fd49c"}},
                "outline_variant": {"default": {"color": "#424940"}}
              }
            }"##,
        );
        write(&static_path, static_env());
        let theme =
            Theme::resolve_with_paths(&Config::default(), dir.path(), &palette_path, &static_path);
        assert_eq!(theme.base, "#101410");
        assert_eq!(theme.surface_alt, "#272b26");
        assert_eq!(theme.border, "#424940");
        assert_eq!(theme.accent, "#9fd49c");
        assert_ne!(theme.accent, "#3333ff");
        assert_ne!(theme.accent, "#fabd2f");
    }

    #[test]
    fn stale_mode_uses_static_even_when_palette_exists() {
        let dir = tempdir().unwrap();
        let palette_path = dir.path().join("palette.json");
        let static_path = dir.path().join("static.env");
        write(&palette_path, &palette("#123456"));
        write(&static_path, static_env());
        let config: Config = toml::from_str("[theme]\nmode = \"static\"\n").unwrap();
        let theme = Theme::resolve_with_paths(&config, dir.path(), &palette_path, &static_path);
        assert_eq!(theme.accent, "#3333ff");
    }

    #[test]
    fn generated_theme_contains_font_roles_and_import() {
        let dir = tempdir().unwrap();
        let rofi = dir.path().join("user.rasi");
        write(&rofi, "* { }\n");
        let config: Config = toml::from_str(&format!(
            "[theme]\nrofi_theme = \"{}\"\n[font]\nfamily = \"Test Mono\"\nsize = 14\n",
            rofi.display()
        ))
        .unwrap();
        let theme = Theme::resolve_with_paths(
            &config,
            dir.path(),
            &dir.path().join("missing.json"),
            &dir.path().join("missing.env"),
        );
        let rasi = theme.rofi_rasi();
        assert!(rasi.contains(&format!("@theme \"{}\"", rofi.display())));
        assert!(rasi.contains("font: \"Test Mono 14\""));
        assert_eq!(theme.accent, "#fabd2f");
        assert_eq!(theme.power, "#d5c4a1");
    }

    #[test]
    fn ui_font_is_secondary_fallback() {
        let dir = tempdir().unwrap();
        let palette_path = dir.path().join("missing.json");
        let static_path = dir.path().join("static.env");
        write(
            &static_path,
            &static_env().replace("THEME_MONO_FONT='Static Mono'", "THEME_UI_FONT='UI Mono'"),
        );
        let theme =
            Theme::resolve_with_paths(&Config::default(), dir.path(), &palette_path, &static_path);
        assert_eq!(theme.font_family, "UI Mono");
        assert_eq!(theme.font_size, 13);
    }

    #[test]
    fn palette_colors_must_be_hex() {
        assert!(valid_color("#abcdef"));
        assert!(!valid_color("#abcde"));
        assert!(!valid_color("abcdef"));
        assert!(!valid_color("#gggggg"));
    }

    #[test]
    fn gruvbox_has_readable_roles() {
        let theme = CoreTheme::gruvbox();
        for (foreground, background) in [
            (&theme.text, &theme.base),
            (&theme.text, &theme.mantle),
            (&theme.muted, &theme.base),
            (&theme.accent, &theme.base),
            (&theme.danger, &theme.base),
        ] {
            assert!(
                theme
                    .contrast_ratio(foreground, background)
                    .is_some_and(|ratio| ratio >= 4.5),
                "{foreground} on {background}"
            );
        }
        for color in [
            &theme.work,
            &theme.browser,
            &theme.communication,
            &theme.entertainment,
        ] {
            assert!(
                theme
                    .contrast_ratio(color, &theme.base)
                    .is_some_and(|ratio| ratio >= 3.0),
                "{color}"
            );
        }
    }

    #[test]
    fn explicit_base_only_changes_base() {
        let mut config = Config::default();
        config.theme.base = Some("#101010".to_owned());
        let mut colors = CoreTheme::builtin();
        let original = colors.clone();
        apply_explicit_colors(&mut colors, &config);
        assert_eq!(colors.base, "#101010");
        assert_eq!(colors.mantle, original.mantle);
        assert_eq!(colors.text, original.text);
        assert_eq!(colors.muted, original.muted);
        assert_eq!(colors.accent, original.accent);
        assert_eq!(colors.power, original.power);
        assert_eq!(colors.other, original.other);
        assert_eq!(colors.work, original.work);
    }

    #[test]
    fn explicit_other_does_not_follow_muted_unless_supplied() {
        let mut config = Config::default();
        config.theme.other = Some("#112233".to_owned());
        let mut colors = CoreTheme::builtin();
        let original = colors.clone();
        apply_explicit_colors(&mut colors, &config);
        assert_eq!(colors.other, "#112233");
        assert_eq!(colors.muted, original.muted);
        assert_eq!(colors.accent, original.accent);
    }

    #[test]
    fn explicit_muted_without_other_updates_both() {
        let mut config = Config::default();
        config.theme.muted = Some("#445566".to_owned());
        let mut colors = CoreTheme::builtin();
        apply_explicit_colors(&mut colors, &config);
        assert_eq!(colors.muted, "#445566");
        assert_eq!(colors.other, "#445566");
        assert_ne!(colors.accent, "#445566");
    }

    #[test]
    fn explicit_accent_without_power_updates_work_only() {
        let mut config = Config::default();
        config.theme.accent = Some("#778899".to_owned());
        let mut colors = CoreTheme::builtin();
        let original = colors.clone();
        apply_explicit_colors(&mut colors, &config);
        assert_eq!(colors.accent, "#778899");
        assert_eq!(colors.work, "#778899");
        assert_eq!(colors.power, original.power);
        assert_eq!(colors.muted, original.muted);
    }

    #[test]
    fn explicit_mixed_valid_and_invalid_palette_changes_nothing() {
        let mut config = Config::default();
        config.theme.base = Some("#123456".to_owned());
        config.theme.accent = Some("not-a-color".to_owned());
        let mut colors = CoreTheme::builtin();
        let original = colors.clone();
        apply_explicit_colors(&mut colors, &config);
        assert_eq!(colors, original);
    }
}
