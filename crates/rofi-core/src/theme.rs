//! Palettes and a generated Rofi theme for the hub menu.
//!
//! Screen Time keeps its own richer role set; the hub only needs a readable
//! manager window. Sharing the mode names means a user can pick the same look
//! for both.

use std::collections::HashMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use serde_json::Value;

use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Auto,
    Nord,
    Solarized,
    Gruvbox,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Auto, Mode::Nord, Mode::Solarized, Mode::Gruvbox];

    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Auto => "auto",
            Mode::Nord => "nord",
            Mode::Solarized => "solarized",
            Mode::Gruvbox => "gruvbox",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Auto => "Auto",
            Mode::Nord => "Nord",
            Mode::Solarized => "Solarized Light",
            Mode::Gruvbox => "Gruvbox Dark",
        }
    }

    /// Cycle to the next mode; used by the `Alt+4` Reset shortcut? No: the
    /// menu binds each mode directly. Kept for `rofi-hub theme next`.
    pub fn next(self) -> Self {
        match self {
            Mode::Auto => Mode::Nord,
            Mode::Nord => Mode::Solarized,
            Mode::Solarized => Mode::Gruvbox,
            Mode::Gruvbox => Mode::Auto,
        }
    }
}

impl FromStr for Mode {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" | "config" | "matugen" | "static" => Ok(Mode::Auto),
            "nord" => Ok(Mode::Nord),
            "solarized" | "solarized-light" | "light" => Ok(Mode::Solarized),
            "gruvbox" | "gruvbox-dark" | "dark" => Ok(Mode::Gruvbox),
            _ => Err(()),
        }
    }
}

/// Layout chrome for one native app. Palette and typography come from
/// [`Theme`]; the profile only controls geometry and icon visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChromeProfile {
    pub width_ch: usize,
    pub lines: usize,
    pub show_icons: bool,
    pub header_pt: u8,
    pub prompt: bool,
}

impl ChromeProfile {
    pub const fn new(width_ch: usize, lines: usize, show_icons: bool, header_pt: u8) -> Self {
        Self {
            width_ch,
            lines,
            show_icons,
            header_pt,
            prompt: true,
        }
    }

    pub fn for_app(app: &str) -> Self {
        match app {
            "screen-time" => Self::new(144, 18, false, 26),
            "calendar" => Self::new(64, 12, false, 20),
            "clipboard" => Self::new(110, 16, false, 18),
            "keybinds" => Self::new(110, 16, false, 18),
            "network" => Self::new(90, 12, true, 20),
            _ => Self::new(96, 20, false, 20),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub base: String,
    /// Primary surface, used for selected rows and panels.
    pub mantle: String,
    /// Hover/pressed surface; may equal `mantle` for flat themes.
    pub surface_alt: String,
    /// Divider color; never used for body text.
    pub border: String,
    pub text: String,
    pub muted: String,
    pub accent: String,
    pub power: String,
    pub work: String,
    pub browser: String,
    pub communication: String,
    pub entertainment: String,
    pub other: String,
    pub danger: String,
    pub font_family: String,
    pub font_size: u8,
}

impl Theme {
    pub fn for_mode(mode: Mode) -> Self {
        match mode {
            // `auto` resolves matugen/static before it reaches this fallback;
            // the builtin default is Gruvbox Dark (hard contrast).
            Mode::Auto => Self::builtin(),
            Mode::Nord => Self::nord(),
            Mode::Solarized => Self::solarized(),
            Mode::Gruvbox => Self::gruvbox(),
        }
    }

    pub fn builtin() -> Self {
        let mut theme = Self::gruvbox();
        theme.name = "auto";
        theme
    }

    pub fn nord() -> Self {
        Self {
            name: "nord",
            base: "#2e3440".to_owned(),
            mantle: "#3b4252".to_owned(),
            surface_alt: "#434c5e".to_owned(),
            border: "#4c566a".to_owned(),
            text: "#eceff4".to_owned(),
            muted: "#d8dee9".to_owned(),
            accent: "#88c0d0".to_owned(),
            power: "#d8dee9".to_owned(),
            work: "#a3be8c".to_owned(),
            browser: "#8fbcdb".to_owned(),
            communication: "#ebcb8b".to_owned(),
            entertainment: "#dfacd0".to_owned(),
            other: "#e5e9f0".to_owned(),
            danger: "#bf616a".to_owned(),
            font_family: "JetBrainsMono Nerd Font".to_owned(),
            font_size: 12,
        }
    }

    pub fn solarized() -> Self {
        Self {
            name: "solarized",
            base: "#fdf6e3".to_owned(),
            mantle: "#eee8d5".to_owned(),
            surface_alt: "#e4ddc8".to_owned(),
            border: "#d5cdb6".to_owned(),
            text: "#073642".to_owned(),
            muted: "#52676e".to_owned(),
            accent: "#00689d".to_owned(),
            power: "#52676e".to_owned(),
            work: "#516100".to_owned(),
            browser: "#00689d".to_owned(),
            communication: "#8d5c00".to_owned(),
            entertainment: "#a52e6f".to_owned(),
            other: "#58636a".to_owned(),
            danger: "#b3302c".to_owned(),
            font_family: "JetBrainsMono Nerd Font".to_owned(),
            font_size: 12,
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            name: "gruvbox",
            base: "#1d2021".to_owned(),
            mantle: "#282828".to_owned(),
            surface_alt: "#32302f".to_owned(),
            border: "#3c3836".to_owned(),
            text: "#fbf1c7".to_owned(),
            muted: "#a89984".to_owned(),
            accent: "#fabd2f".to_owned(),
            power: "#d5c4a1".to_owned(),
            work: "#b8bb26".to_owned(),
            browser: "#83a598".to_owned(),
            communication: "#fe8019".to_owned(),
            entertainment: "#d3869b".to_owned(),
            other: "#928374".to_owned(),
            danger: "#fb4934".to_owned(),
            font_family: "JetBrainsMono Nerd Font".to_owned(),
            font_size: 12,
        }
    }

    /// A light fallback used when a matugen palette says the desktop is light.
    pub fn light() -> Self {
        Self {
            name: "light",
            base: "#faf8f4".to_owned(),
            mantle: "#ece8e0".to_owned(),
            surface_alt: "#e4dfd3".to_owned(),
            border: "#d6d0c3".to_owned(),
            text: "#231f20".to_owned(),
            muted: "#55505a".to_owned(),
            accent: "#25458b".to_owned(),
            power: "#3e5760".to_owned(),
            work: "#23623a".to_owned(),
            browser: "#1e5195".to_owned(),
            communication: "#834c09".to_owned(),
            entertainment: "#992755".to_owned(),
            other: "#635a63".to_owned(),
            danger: "#a3231f".to_owned(),
            font_family: "JetBrainsMono Nerd Font".to_owned(),
            font_size: 12,
        }
    }

    pub fn font(&self) -> String {
        format!("{} {}", self.font_family, self.font_size)
    }

    /// A standalone fallback theme, installed alongside the binaries.
    pub fn rasi(&self) -> String {
        self.rasi_with(ChromeProfile::for_app("hub"))
    }

    /// A standalone fallback theme with app-specific chrome geometry.
    pub fn rasi_with(&self, profile: ChromeProfile) -> String {
        let font = escape_rasi(&self.font());
        let base = &self.base;
        let mantle = &self.mantle;
        let surface_alt = &self.surface_alt;
        let border = &self.border;
        let text = &self.text;
        let muted = &self.muted;
        let accent = &self.accent;
        let show_icons = profile.show_icons;
        let width = profile.width_ch;
        let lines = profile.lines;

        let mut out = String::new();
        let _ = write!(
            out,
            r#"/* Generated by rofi-hub. Overrides welcome; the hub regenerates
   its runtime copy on every launch. */
configuration {{
    font: "{font}";
    show-icons: {show_icons};
    disable-history: true;
    hover-select: false;
    cycle: true;
    matching: "fuzzy";
}}

* {{
    font: "{font}";
    background-color: {base};
    text-color: {text};
    border-color: {border};
    selected-normal-background: {mantle};
    selected-normal-foreground: {text};
    normal-background: {base};
    normal-foreground: {text};
    foreground: {text};
    background: {base};
    spacing: 0;
    margin: 0;
    padding: 0;
}}

/* Zoned chrome: one bordered card for filter, header, and list. */
window {{
    width: calc( ( {width}ch + 64px ) min 94% );
    location: center;
    anchor: center;
    border: 1px;
    border-color: {border};
    border-radius: 12px;
    padding: 12px;
    background-color: {base};
}}

mainbox {{
    background-color: {base};
    children: [ inputbar, message, listview ];
    spacing: 8px;
    padding: 0;
}}

inputbar {{
    background-color: {mantle};
    text-color: {text};
    padding: 6px 12px;
    border: 1px;
    border-color: {border};
    border-radius: 8px;
    children: [ prompt, entry ];
    spacing: 10px;
}}

message {{
    background-color: {mantle};
    text-color: {text};
    padding: 0px 12px;
    border: 0px 0px 1px 0px;
    border-color: {border};
}}

textbox {{
    background-color: transparent;
    text-color: {text};
    padding: 0;
    border: 0;
}}

prompt {{
    background-color: transparent;
    text-color: {accent};
}}

entry {{
    background-color: transparent;
    text-color: {text};
    placeholder: "Type to filter";
    placeholder-color: {muted};
}}

listview {{
    background-color: {mantle};
    columns: 1;
    lines: {lines};
    spacing: 0;
    scrollbar: false;
    cycle: true;
    dynamic: true;
    fixed-height: false;
    padding: 4px;
    border: 1px;
    border-color: {border};
    border-radius: 8px;
}}

/* Section rows are marked active by the apps; the band separates zones. */
element normal.active, element alternate.active, element selected.active {{
    background-color: {surface_alt};
    border: 0px 0px 1px 0px;
    border-color: {border};
    border-radius: 4px;
}}

element {{
    children: [ element-icon, element-text ];
    padding: 4px 10px;
    background-color: {mantle};
    text-color: {text};
    spacing: 8px;
    border: 0px 0px 1px 0px;
    border-color: {border};
    border-radius: 0;
}}

element selected {{
    background-color: {surface_alt};
    text-color: {text};
    border: 0px 0px 1px 2px;
    border-color: {accent};
    border-radius: 4px;
}}

element selected.active {{
    background-color: {surface_alt};
    border: 0px 0px 1px 2px;
    border-color: {accent};
    border-radius: 4px;
}}

element-icon {{
    size: 1.2em;
    background-color: transparent;
}}

element-text {{
    background-color: transparent;
    text-color: inherit;
    font: "{font}";
    vertical-align: 0.5;
}}
"#
        );
        out
    }
}

impl Theme {
    /// The theme currently in effect for this user/session.
    pub fn active() -> Self {
        Self::resolve(active_mode())
    }

    /// Resolve matugen then static.env, then the builtin Gruvbox fallback.
    pub fn resolve(mode: Mode) -> Self {
        if mode == Mode::Auto {
            Self::from_system().unwrap_or_else(Self::builtin)
        } else {
            Self::for_mode(mode)
        }
    }

    /// Default user locations: `~/.config/theme/generated/palette.json` then
    /// `~/.config/theme/static.env`.
    pub fn from_system() -> Option<Self> {
        let config = paths::config_home();
        Self::from_palette_file(&config.join("theme/generated/palette.json"))
            .or_else(|| Self::from_static_env(&config.join("theme/static.env")))
    }

    /// Parse a matugen palette and make every role readable on its surfaces.
    pub fn from_palette_file(path: &Path) -> Option<Self> {
        let bytes = fs::read(path).ok()?;
        let value: Value = serde_json::from_slice(&bytes).ok()?;
        let base = json_color(&value, &["colors", "surface", "default", "color"])?;
        let mantle = json_color(&value, &["colors", "surface_container", "default", "color"])?;
        let text = json_color(&value, &["colors", "on_surface", "default", "color"])?;
        let muted = json_color(
            &value,
            &["colors", "on_surface_variant", "default", "color"],
        )?;
        let accent = json_color(&value, &["colors", "primary", "default", "color"])?;
        let power = json_color(&value, &["palettes", "secondary", "70", "color"])?;
        let base_luminance = luminance(&base)?;
        let light = contrast(0.0, base_luminance) >= contrast(1.0, base_luminance);
        let mut theme = if light {
            Self::light()
        } else {
            Self::gruvbox()
        };
        theme.name = "matugen";
        let mut backgrounds = [base_luminance, luminance(&mantle)?];
        let endpoint = if light { 0 } else { 255 };
        if contrast(f64::from(endpoint) / 255.0, backgrounds[1]) < 4.5 {
            backgrounds[1] = luminance(&theme.mantle)?;
        } else {
            theme.mantle = mantle;
        }
        theme.base = base;
        theme.text = text;
        theme.muted = muted;
        theme.accent = accent;
        theme.power = power;
        for color in [
            &mut theme.text,
            &mut theme.muted,
            &mut theme.accent,
            &mut theme.power,
            &mut theme.work,
            &mut theme.browser,
            &mut theme.other,
            &mut theme.communication,
            &mut theme.entertainment,
            &mut theme.danger,
        ] {
            let Some(lum) = luminance(color) else {
                continue;
            };
            if backgrounds.iter().any(|&bg| contrast(lum, bg) < 4.5) {
                *color = readable_color(color, backgrounds, endpoint);
            }
        }
        Some(theme)
    }

    /// Parse a static.env written by a wallpaper/theme hook.
    pub fn from_static_env(path: &Path) -> Option<Self> {
        let vars = parse_env_file(path)?;
        let base = required_color(&vars, &["THEME_STATIC_BASE"])?;
        let mantle = required_color(&vars, &["THEME_STATIC_MANTLE"])?;
        let text = required_color(&vars, &["THEME_STATIC_TEXT"])?;
        let muted = required_color(&vars, &["THEME_STATIC_SUBTEXT", "THEME_SIGNAL_MUTED"])?;
        let accent = required_color(&vars, &["THEME_STATIC_ACCENT"])?;
        let power = required_color(&vars, &["THEME_STATIC_CYAN"])?;
        let mut theme = Self::gruvbox();
        theme.name = "static";
        theme.base = base;
        theme.mantle = mantle;
        theme.text = text;
        theme.muted = muted;
        theme.accent = accent.clone();
        theme.work = accent;
        theme.power = power;
        theme.browser = optional_color(&vars, &["THEME_STATIC_BLUE"], &theme.power);
        theme.communication =
            optional_color(&vars, &["THEME_STATIC_WARNING"], &theme.communication);
        theme.entertainment = optional_color(&vars, &["THEME_STATIC_ERROR"], &theme.danger);
        theme.other = optional_color(&vars, &["THEME_STATIC_OUTLINE"], &theme.muted);
        theme.danger = optional_color(&vars, &["THEME_STATIC_ERROR"], &theme.danger);
        Some(theme)
    }

    /// Contrast ratio between two roles, useful for UI tests.
    pub fn contrast_ratio(&self, foreground: &str, background: &str) -> Option<f64> {
        contrast_ratio(foreground, background)
    }
}

/// Parse a `KEY=value` file with optional `export` prefixes and quotes.
pub fn parse_env_file(path: &Path) -> Option<HashMap<String, String>> {
    let text = fs::read_to_string(path).ok()?;
    let mut vars = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim();
        let value = if value.len() >= 2
            && ((value.starts_with('\'') && value.ends_with('\''))
                || (value.starts_with('"') && value.ends_with('"')))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };
        vars.insert(key.to_owned(), value.to_owned());
    }
    Some(vars)
}

/// Optional font overrides from a static.env file.
pub fn font_from_static(path: &Path) -> Option<(Option<String>, Option<u8>)> {
    let vars = parse_env_file(path)?;
    let family = ["THEME_MONO_FONT", "THEME_UI_FONT"]
        .iter()
        .find_map(|key| vars.get(*key))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty() && !value.chars().any(char::is_control));
    let size = vars
        .get("THEME_ROFI_FONT_SIZE")
        .and_then(|value| value.trim().parse::<u8>().ok())
        .filter(|size| (6..=72).contains(size));
    Some((family, size))
}

pub fn valid_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

/// WCAG contrast between two validated colors.
pub fn contrast_ratio(foreground: &str, background: &str) -> Option<f64> {
    let first = luminance(foreground)?;
    let second = luminance(background)?;
    Some(contrast(first, second))
}

fn contrast(first: f64, second: f64) -> f64 {
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn json_color(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    let color = current.as_str()?;
    valid_color(color).then(|| color.to_owned())
}

fn required_color(vars: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = vars.get(*key) {
            if valid_color(value) {
                return Some(value.clone());
            }
            return None;
        }
    }
    None
}

fn optional_color(vars: &HashMap<String, String>, keys: &[&str], fallback: &str) -> String {
    keys.iter()
        .find_map(|key| vars.get(*key))
        .filter(|value| valid_color(value))
        .cloned()
        .unwrap_or_else(|| fallback.to_owned())
}

fn rgb(color: &str) -> Option<[u8; 3]> {
    let bytes = color.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return None;
    }
    let channel = |index: usize| {
        let high = hex_digit(bytes[index])?;
        let low = hex_digit(bytes[index + 1])?;
        Some(high << 4 | low)
    };
    Some([channel(1)?, channel(3)?, channel(5)?])
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn luminance(color: &str) -> Option<f64> {
    let [r, g, b] = rgb(color)?.map(|channel| {
        let c = f64::from(channel) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    Some(0.2126 * r + 0.7152 * g + 0.0722 * b)
}

fn readable_color(color: &str, backgrounds: [f64; 2], endpoint: u8) -> String {
    let Some(rgb) = rgb(color) else {
        return color.to_owned();
    };
    for step in 1..=20 {
        let [r, g, b] = rgb.map(|c| (u16::from(c) * (20 - step) + u16::from(endpoint) * step) / 20);
        let candidate = format!("#{r:02x}{g:02x}{b:02x}");
        let Some(lum) = luminance(&candidate) else {
            continue;
        };
        if backgrounds.iter().all(|&bg| contrast(lum, bg) >= 4.5) {
            return candidate;
        }
    }
    color.to_owned()
}

/// Read the saved global theme mode, if any.
pub fn saved_mode() -> Option<Mode> {
    fs::read_to_string(paths::hub_theme_state_path())
        .ok()
        .and_then(|value| value.trim().parse().ok())
}

/// Persist the global theme mode shared by every native app.
pub fn save_mode(mode: Mode) -> std::io::Result<()> {
    let path = paths::hub_theme_state_path();
    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "theme state path has no parent directory",
        )
    })?;
    fs::create_dir_all(dir)?;
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    let temp = dir.join(format!(".theme.{}", std::process::id()));
    fs::write(&temp, format!("{}\n", mode.as_str()))?;
    let _ = fs::set_permissions(&temp, fs::Permissions::from_mode(0o600));
    fs::rename(&temp, &path)?;
    Ok(())
}

/// The theme colors currently in effect.
///
/// The hub passes `ROFI_HUB_THEME` to every provider. Direct launches fall
/// back to the saved hub choice, then to `auto`. Generated Palettes only
/// affect the Rofi window; rows use these shared role colors.
pub fn active() -> Theme {
    Theme::active()
}

/// The mode behind [`active`]. Direct app launches need the mode as well as
/// the palette so a saved Nord/Gruvbox choice generates the matching theme
/// instead of falling back to the static auto theme.
pub fn active_mode() -> Mode {
    env::var("ROFI_HUB_THEME")
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .or_else(|| {
            fs::read_to_string(paths::hub_theme_state_path())
                .ok()
                .and_then(|value| value.trim().parse().ok())
        })
        .unwrap_or(Mode::Auto)
}

/// Resolve `auto` to a user Rofi theme when one exists, otherwise generate
/// the builtin palette. Returns the path passed to Rofi plus the effective
/// theme for row coloring.
pub fn prepare(mode: Mode, theme: &Theme, app: &str) -> Option<std::path::PathBuf> {
    if mode == Mode::Auto
        && let Some(raw) = std::env::var_os("ROFI_THEME")
    {
        let path = std::path::PathBuf::from(raw);
        if path.is_file() {
            return Some(path);
        }
    }
    if mode == Mode::Auto
        && let Some(path) = paths::hub_theme_candidates()
            .into_iter()
            .find(|candidate| candidate.is_file())
    {
        return Some(path);
    }

    let dir = paths::hub_runtime_dir();
    std::fs::create_dir_all(&dir).ok()?;
    set_private_dir(&dir);
    // Each app has its own geometry (width, visible lines, icons), so the
    // generated file must be app-scoped instead of a single shared theme.rasi.
    let safe_app: String = app
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let safe_app = if safe_app.is_empty() {
        "app"
    } else {
        &safe_app
    };
    let path = dir.join(format!("theme-{safe_app}.rasi"));
    let temp = dir.join(format!(".theme-{safe_app}.{}", std::process::id()));
    if std::fs::write(&temp, theme.rasi_with(ChromeProfile::for_app(app))).is_err() {
        return None;
    }
    set_private_file(&temp);
    std::fs::rename(&temp, &path).ok()?;
    Some(path)
}

fn set_private_dir(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
}

fn set_private_file(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

fn escape_rasi(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mode_aliases() {
        assert_eq!("auto".parse(), Ok(Mode::Auto));
        assert_eq!("Nord".parse(), Ok(Mode::Nord));
        assert_eq!("solarized-light".parse(), Ok(Mode::Solarized));
        assert_eq!("gruvbox-dark".parse(), Ok(Mode::Gruvbox));
        assert!("sepia".parse::<Mode>().is_err());
    }

    #[test]
    fn palettes_stay_readable() {
        for mode in Mode::ALL {
            let theme = Theme::for_mode(mode);
            for color in [
                &theme.base,
                &theme.mantle,
                &theme.text,
                &theme.muted,
                &theme.accent,
            ] {
                assert!(color.starts_with('#') && color.len() == 7, "{color}");
            }
            assert!(theme.text != theme.base);
        }
    }

    #[test]
    fn palettes_match_screen_time_roles() {
        let nord = Theme::nord();
        assert_eq!(
            (nord.muted.as_str(), nord.accent.as_str()),
            ("#d8dee9", "#88c0d0")
        );
        let solarized = Theme::solarized();
        assert_eq!(
            (solarized.muted.as_str(), solarized.accent.as_str()),
            ("#52676e", "#00689d")
        );
        let gruvbox = Theme::gruvbox();
        assert_eq!(
            (
                gruvbox.base.as_str(),
                gruvbox.mantle.as_str(),
                gruvbox.text.as_str(),
                gruvbox.muted.as_str(),
                gruvbox.accent.as_str(),
            ),
            ("#1d2021", "#282828", "#fbf1c7", "#a89984", "#fabd2f")
        );
    }

    #[test]
    fn generated_theme_has_selection_and_entry_rules() {
        let rasi = Theme::nord().rasi();
        assert!(rasi.contains("element selected"));
        assert!(rasi.contains("inputbar"));
        assert!(!rasi.contains("Nord"));
        assert!(rasi.contains("font: \"JetBrainsMono Nerd Font 12\""));
    }

    #[test]
    fn generated_theme_has_zoned_chrome() {
        let rasi = Theme::nord().rasi();
        // Shared visual tokens with crates/screen-time/src/theme.rs.
        assert!(rasi.contains("padding: 12px;"));
        assert!(rasi.contains("width: calc( ( 96ch + 64px ) min 94% );"));
        assert!(rasi.contains("border-radius: 12px;"));
        assert!(rasi.contains("border: 0px 0px 1px 2px;"));
        assert!(rasi.contains("border-color: #88c0d0;"));
        assert!(rasi.contains("hover-select: false;"));
        assert!(rasi.contains("lines: 20;"));
        assert!(rasi.contains("padding: 4px 10px;"));
        assert!(rasi.contains("border-radius: 8px;"));
    }

    #[test]
    fn static_theme_files_match_generator() {
        let hub = include_str!("../../../themes/hub.rasi");
        assert_eq!(
            Theme::gruvbox().rasi_with(ChromeProfile::for_app("hub")),
            hub
        );
        let screen_time = include_str!("../../../crates/screen-time/share/rofi/screen-time.rasi");
        assert_eq!(
            Theme::gruvbox().rasi_with(ChromeProfile::for_app("screen-time")),
            screen_time
        );
    }

    #[test]
    fn quotes_are_escaped_in_font_rules() {
        let mut theme = Theme::builtin();
        theme.font_family = "My \"Font\"".to_owned();
        assert!(theme.rasi().contains(r#"font: "My \"Font\" 12""#));
    }
}
