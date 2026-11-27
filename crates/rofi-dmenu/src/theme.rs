//! Palettes and a generated Rofi theme for the hub menu.
//!
//! Screen Time keeps its own richer role set; the hub only needs a readable
//! manager window. Sharing the mode names means a user can pick the same look
//! for both.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
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
/// [`Theme`]; the profile only controls geometry, widget order, and icon
/// visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChromeProfile {
    pub width_ch: usize,
    pub lines: usize,
    pub show_icons: bool,
    pub header_pt: u8,
    pub prompt: bool,
    /// Number of listview columns. Calendar uses three compact action
    /// buttons in one row; the other apps use a single column.
    pub columns: usize,
    /// Entry placeholder for the app's primary input box.
    pub placeholder: &'static str,
    /// `true` puts the message above the input (calendar sheet, network
    /// status); `false` keeps Rofi's default input-then-message order.
    pub message_first: bool,
    /// Calendar's button row has no card, border, or full-width selection.
    pub compact_list: bool,
    /// Center message text (month title and calendar grid).
    pub centered_message: bool,
}

impl ChromeProfile {
    pub const fn new(width_ch: usize, lines: usize, show_icons: bool, header_pt: u8) -> Self {
        Self {
            width_ch,
            lines,
            show_icons,
            header_pt,
            prompt: true,
            columns: 1,
            placeholder: "Type to filter",
            message_first: false,
            compact_list: false,
            centered_message: false,
        }
    }

    pub fn for_app(app: &str) -> Self {
        match app {
            "screen-time" => Self::new(144, 18, false, 26),
            "calendar" => {
                let mut profile = Self::new(44, 1, false, 16);
                profile.columns = 3;
                profile.placeholder = "DD/MM/YYYY";
                profile.message_first = true;
                profile.compact_list = true;
                profile.centered_message = true;
                profile
            }
            "clipboard" => Self::new(110, 16, false, 18),
            "keybinds" => {
                let mut profile = Self::new(110, 10, false, 18);
                profile.columns = 3;
                profile
            }
            "network" => {
                let mut profile = Self::new(62, 13, false, 16);
                profile.placeholder = "\u{2026}";
                profile.message_first = true;
                profile
            }
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
        let columns = profile.columns;
        let placeholder = escape_rasi(profile.placeholder);
        let mainbox_children = if profile.message_first {
            "message, inputbar, listview"
        } else {
            "inputbar, message, listview"
        };
        let message_align = if profile.centered_message {
            "    text-align: center;\n"
        } else {
            ""
        };
        let listview_layout = if profile.compact_list {
            "    background-color: transparent;\n    border: 0;\n    padding: 0;\n    spacing: 4px;\n"
                .to_owned()
        } else {
            format!(
                "    background-color: {mantle};\n    border: 1px;\n    border-color: {border};\n    border-radius: 8px;\n    padding: 4px;\n    spacing: 0;\n"
            )
        };
        let element_layout = if profile.compact_list {
            "    padding: 6px 8px;\n    border: 0;\n    border-radius: 6px;\n".to_owned()
        } else {
            format!(
                "    padding: 6px 10px;\n    border: 0px 0px 1px 0px;\n    border-color: {border};\n    border-radius: 0;\n"
            )
        };
        let selected_layout =
            format!("    border: 1px;\n    border-color: {accent};\n    border-radius: 6px;\n");

        let mut out = String::new();
        let _ = write!(
            out,
            r#"/* Generated by rofi-dmenu: app layout + palette fallback.
   Overrides welcome; the app regenerates its runtime copy when it changes. */
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
    selected-normal-background: {surface_alt};
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
    padding: 16px;
    background-color: {base};
}}

mainbox {{
    background-color: {base};
    children: [ {mainbox_children} ];
    spacing: 12px;
    padding: 0;
}}

inputbar {{
    background-color: {mantle};
    text-color: {text};
    padding: 8px 12px;
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
{message_align}}}

textbox {{
    background-color: transparent;
    text-color: {text};
    padding: 0;
    border: 0;
{message_align}}}

prompt {{
    background-color: transparent;
    text-color: {accent};
}}

entry {{
    background-color: transparent;
    text-color: {text};
    placeholder: "{placeholder}";
    placeholder-color: {muted};
}}

listview {{
    columns: {columns};
    lines: {lines};
    scrollbar: false;
    cycle: true;
    dynamic: true;
    fixed-height: false;
{listview_layout}}}

/* Section rows are marked active by the apps; the band separates zones. */
element normal.active, element alternate.active, element selected.active {{
    background-color: {surface_alt};
    text-color: {text};
    border: 0px 0px 1px 0px;
    border-color: {border};
    border-radius: 4px;
}}

element {{
    children: [ element-icon, element-text ];
{element_layout}    background-color: {mantle};
    text-color: {text};
    spacing: 8px;
}}

element selected {{
    background-color: {surface_alt};
    text-color: {text};
{selected_layout}}}

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
    /// The theme colors currently in effect for this user/session.
    ///
    /// The palette comes from the user's theme module (a matugen palette or
    /// `static.env`) and falls back to the builtin palette. No hub state is
    /// consulted.
    pub fn active() -> Self {
        Self::from_system().unwrap_or_else(Self::builtin)
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
        // A wallpaper palette does not always ship the optional secondary
        // palette. Keep the generated surface/accent instead of rejecting the
        // whole palette and falling back to static.env or Gruvbox.
        let power = json_color(&value, &["palettes", "secondary", "70", "color"])
            .or_else(|| json_color(&value, &["colors", "secondary", "default", "color"]))
            .or_else(|| json_color(&value, &["colors", "tertiary", "default", "color"]))
            .or_else(|| json_color(&value, &["colors", "primary", "default", "color"]))?;
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
        if let Some(surface_alt) = json_color(
            &value,
            &["colors", "surface_container_high", "default", "color"],
        ) {
            theme.surface_alt = surface_alt;
        }
        if let Some(border) = json_color(&value, &["colors", "outline_variant", "default", "color"])
        {
            theme.border = border;
        }
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
        theme.mantle = mantle.clone();
        theme.text = text;
        theme.muted = muted.clone();
        theme.accent = accent.clone();
        theme.work = accent;
        theme.power = power;
        theme.surface_alt = optional_color(
            &vars,
            &["THEME_STATIC_SURFACE_ALT", "THEME_STATIC_OVERLAY"],
            &mantle,
        );
        theme.border = optional_color(&vars, &["THEME_STATIC_OUTLINE"], &theme.border);
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

/// The theme colors currently in effect.
///
/// Apps use these role colors for row markup. The Rofi window itself is
/// themed by [`resolve_theme_file`], which layers the user's base theme under
/// the app layout.
pub fn active() -> Theme {
    Theme::active()
}

/// Resolve the Rofi theme file for one app. Precedence:
///
/// 1. `~/.config/rofi/<app>.rasi` (full app override)
/// 2. `ROFI_THEME` (full global override)
/// 3. `~/.config/rofi/theme.rasi` (user base + app layout)
/// 4. `$XDG_DATA_DIRS/rofi/<app>.rasi` (packaged fallback layout + palette)
/// 5. built-in fallback palette + layout
///
/// Generated files are app-scoped because each app has its own geometry, and
/// are only rewritten when their content changes.
pub fn resolve_theme_file(app: &str) -> Option<std::path::PathBuf> {
    resolve_theme_file_with(app, &Theme::active())
}

/// Resolve the app theme with an explicit palette for the built-in fallback.
/// User overrides, `ROFI_THEME`, and the user base theme keep precedence.
pub fn resolve_theme_file_with(app: &str, theme: &Theme) -> Option<std::path::PathBuf> {
    resolve_theme_file_in(
        app,
        &paths::config_home(),
        &paths::data_dirs_with_system_fallback(),
        paths::global_theme_override().as_deref(),
        &paths::rofi_runtime_dir(),
        theme,
    )
}

fn resolve_theme_file_in(
    app: &str,
    config_home: &std::path::Path,
    data_dirs: &[std::path::PathBuf],
    global_override: Option<&std::path::Path>,
    runtime_dir: &std::path::Path,
    theme: &Theme,
) -> Option<std::path::PathBuf> {
    let app_override = config_home.join("rofi").join(format!("{app}.rasi"));
    if app_override.is_file() {
        return Some(app_override);
    }

    if let Some(path) = global_override.filter(|path| path.is_file()) {
        return Some(path.to_path_buf());
    }

    let base = config_home.join("rofi/theme.rasi");
    if base.is_file() {
        return write_generated_theme_in(runtime_dir, app, &layered_theme(app, &base));
    }

    for dir in data_dirs {
        let candidate = dir.join("rofi").join(format!("{app}.rasi"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    write_generated_theme_in(
        runtime_dir,
        app,
        &theme.rasi_with(ChromeProfile::for_app(app)),
    )
}

/// Write a full generated theme (palette + app layout) for `app`.
pub fn generate_theme(app: &str, theme: &Theme) -> Option<std::path::PathBuf> {
    write_generated_theme(app, &theme.rasi_with(ChromeProfile::for_app(app)))
}

/// Layer a palette over the configured app theme so a theme shortcut
/// retains the user's fonts, widget sizes, spacing, and borders.
pub fn write_palette_theme(app: &str, base_theme: &Path, theme: &Theme) -> Option<PathBuf> {
    let import = escape_rasi(&base_theme.display().to_string());
    let Theme {
        base,
        mantle,
        surface_alt,
        border,
        text,
        muted,
        accent,
        ..
    } = theme;
    let selected_text = if contrast_ratio(accent, base) >= contrast_ratio(accent, text) {
        base
    } else {
        text
    };
    let content = format!(
        r#"@import "{import}"

* {{
    text-color: {text};
    border-color: {border};
    background: {base};
    foreground: {text};
    normal-background: {mantle};
    normal-foreground: {text};
    selected-normal-background: {surface_alt};
    selected-normal-foreground: {text};
}}
window {{ background-color: {base}; border-color: {border}; }}
mainbox, listview {{ background-color: transparent; }}
inputbar, message {{ background-color: {mantle}; border-color: {border}; text-color: {text}; }}
prompt {{ background-color: {accent}; text-color: {selected_text}; }}
entry {{ text-color: {text}; placeholder-color: {muted}; }}
textbox {{ text-color: {text}; }}
element, element normal.normal, element alternate.normal {{
    background-color: transparent;
    border-color: {border};
    text-color: {text};
}}
element normal.active, element alternate.active, element selected.active {{
    background-color: {surface_alt};
    text-color: {text};
    border: 0px 0px 1px 0px;
    border-color: {border};
    border-radius: 4px;
}}
element selected, element selected.normal {{
    background-color: {surface_alt};
    border: 1px;
    border-color: {accent};
    border-radius: 6px;
    text-color: {text};
}}
element-text {{ text-color: inherit; }}
"#
    );
    write_generated_theme(app, &content)
}

/// A base theme plus layout-only rules. Colors and fonts stay owned by the
/// base theme; the app only contributes geometry.
fn layered_theme(app: &str, base: &std::path::Path) -> String {
    let import = escape_rasi(&base.display().to_string());
    format!(
        "/* Generated by {app}: user base theme + app layout. */\n@import \"{import}\"\n\n{}",
        layout_rasi(ChromeProfile::for_app(app))
    )
}

fn layout_rasi(profile: ChromeProfile) -> String {
    let width = profile.width_ch;
    let lines = profile.lines;
    let show_icons = profile.show_icons;
    let columns = profile.columns;
    let placeholder = escape_rasi(profile.placeholder);
    let mainbox_children = if profile.message_first {
        "message, inputbar, listview"
    } else {
        "inputbar, message, listview"
    };
    let message_align = if profile.centered_message {
        "    text-align: center;\n"
    } else {
        ""
    };
    let listview_layout = if profile.compact_list {
        "    padding: 0;\n    border: 0;\n    spacing: 4px;\n"
    } else {
        "    padding: 4px;\n    border: 1px;\n    border-radius: 8px;\n    spacing: 0;\n"
    };
    let element_layout = if profile.compact_list {
        "    padding: 6px 8px;\n    border: 0;\n    border-radius: 6px;\n"
    } else {
        "    padding: 6px 10px;\n    border: 0px 0px 1px 0px;\n    border-radius: 0;\n"
    };
    format!(
        r#"configuration {{
    show-icons: {show_icons};
    disable-history: true;
    hover-select: false;
    cycle: true;
    matching: "fuzzy";
}}

window {{
    width: calc( ( {width}ch + 64px ) min 94% );
    location: center;
    anchor: center;
    border: 1px;
    border-radius: 12px;
    padding: 16px;
}}

mainbox {{
    children: [ {mainbox_children} ];
    spacing: 12px;
    padding: 0;
}}

inputbar {{
    padding: 8px 12px;
    border: 1px;
    border-radius: 8px;
    children: [ prompt, entry ];
    spacing: 10px;
}}

message {{
    padding: 0px 12px;
    border: 0px 0px 1px 0px;
{message_align}}}

textbox {{
    padding: 0;
    border: 0;
{message_align}}}

entry {{
    placeholder: "{placeholder}";
}}

listview {{
    columns: {columns};
    lines: {lines};
    scrollbar: false;
    cycle: true;
    dynamic: true;
    fixed-height: false;
{listview_layout}}}

element {{
{element_layout}    spacing: 8px;
}}

element-icon {{
    size: 1.2em;
}}

element-text {{
    vertical-align: 0.5;
}}
"#
    )
}

/// Write `content` to the app-scoped runtime theme file. The write is skipped
/// when the file already has the same bytes, so a cache hit touches nothing.
pub fn write_generated_theme(app: &str, content: &str) -> Option<std::path::PathBuf> {
    write_generated_theme_in(&paths::rofi_runtime_dir(), app, content)
}

fn write_generated_theme_in(
    dir: &std::path::Path,
    app: &str,
    content: &str,
) -> Option<std::path::PathBuf> {
    std::fs::create_dir_all(dir).ok()?;
    set_private_dir(dir);

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
    if std::fs::read_to_string(&path).ok().as_deref() == Some(content) {
        return Some(path);
    }

    let temp = dir.join(format!(".theme-{safe_app}.{}", std::process::id()));
    if std::fs::write(&temp, content).is_err() {
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
        assert!(rasi.contains("padding: 16px;"));
        assert!(rasi.contains("width: calc( ( 96ch + 64px ) min 94% );"));
        assert!(rasi.contains("border-radius: 12px;"));
        assert!(rasi.contains("border: 1px;"));
        assert!(rasi.contains("border-color: #88c0d0;"));
        assert!(rasi.contains("hover-select: false;"));
        assert!(rasi.contains("lines: 20;"));
        assert!(rasi.contains("padding: 6px 10px;"));
        assert!(rasi.contains("border-radius: 8px;"));
    }

    #[test]
    fn generated_theme_keeps_active_headers_out_of_the_selection_style() {
        let rasi = Theme::nord().rasi();
        let active = rasi
            .split("element normal.active")
            .nth(1)
            .and_then(|rest| rest.split('}').next())
            .expect("active header rule");
        assert!(active.contains("text-color: #eceff4;"), "{active}");
        assert!(active.contains("border-radius: 4px;"), "{active}");
        assert!(
            !rasi.contains(
                "element selected.active {{\n    background-color: #434c5e;\n    border: 1px;"
            ),
            "selected active header must not inherit the row outline"
        );
    }

    #[test]
    fn matugen_palette_maps_surface_and_border_roles() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("palette.json");
        std::fs::write(
            &path,
            r##"{
              "colors": {
                "surface": {"default": {"color": "#101010"}},
                "surface_container": {"default": {"color": "#202020"}},
                "surface_container_high": {"default": {"color": "#303030"}},
                "on_surface": {"default": {"color": "#f0f0f0"}},
                "on_surface_variant": {"default": {"color": "#909090"}},
                "primary": {"default": {"color": "#00cccc"}},
                "outline_variant": {"default": {"color": "#404040"}}
              },
              "palettes": {"secondary": {"70": {"color": "#00cccc"}}}
            }"##,
        )
        .unwrap();
        let theme = Theme::from_palette_file(&path).unwrap();
        assert_eq!(theme.surface_alt, "#303030");
        assert_eq!(theme.border, "#404040");
    }

    #[test]
    fn matugen_palette_without_secondary_still_uses_its_surfaces() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("palette.json");
        std::fs::write(
            &path,
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
        )
        .unwrap();
        let theme = Theme::from_palette_file(&path).expect("palette without secondary");
        assert_eq!(theme.base, "#101410");
        assert_eq!(theme.mantle, "#1c211b");
        assert_eq!(theme.surface_alt, "#272b26");
        assert_eq!(theme.border, "#424940");
        assert_eq!(theme.accent, "#9fd49c");
        assert_eq!(theme.power, "#9fd49c");
    }

    #[test]
    fn static_env_maps_surface_and_border_roles() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("static.env");
        std::fs::write(
            &path,
            "THEME_STATIC_BASE='#101410'\n\
             THEME_STATIC_MANTLE='#1c211b'\n\
             THEME_STATIC_SURFACE_ALT='#272b26'\n\
             THEME_STATIC_TEXT='#e0e4db'\n\
             THEME_STATIC_SUBTEXT='#c2c9bd'\n\
             THEME_STATIC_ACCENT='#9fd49c'\n\
             THEME_STATIC_CYAN='#9eb09a'\n\
             THEME_STATIC_OUTLINE='#424940'\n",
        )
        .unwrap();
        let theme = Theme::from_static_env(&path).unwrap();
        assert_eq!(theme.surface_alt, "#272b26");
        assert_eq!(theme.border, "#424940");
        assert_eq!(theme.accent, "#9fd49c");
    }

    #[test]
    fn static_theme_files_match_generator() {
        let hub = include_str!("../../rofi-hub-menu/share/rofi/hub-menu.rasi");
        assert_eq!(
            Theme::gruvbox().rasi_with(ChromeProfile::for_app("hub")),
            hub
        );
        let screen_time = include_str!("../../screen-time/share/rofi/screen-time.rasi");
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

    #[test]
    fn theme_precedence_matches_the_contract() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join("config");
        let data = tmp.path().join("data");
        let runtime = tmp.path().join("run");
        std::fs::create_dir_all(config.join("rofi")).unwrap();
        std::fs::create_dir_all(data.join("rofi")).unwrap();
        let app_override = config.join("rofi/calendar.rasi");
        let global = tmp.path().join("global.rasi");
        let base = config.join("rofi/theme.rasi");
        let packaged = data.join("rofi/calendar.rasi");
        let theme = Theme::gruvbox();

        // 1. User app override wins over everything.
        std::fs::write(&app_override, "/* override */\n").unwrap();
        std::fs::write(&global, "/* global */\n").unwrap();
        std::fs::write(&base, "/* base */\n").unwrap();
        std::fs::write(&packaged, "/* packaged */\n").unwrap();
        let picked = resolve_theme_file_in(
            "calendar",
            &config,
            std::slice::from_ref(&data),
            Some(&global),
            &runtime,
            &theme,
        )
        .unwrap();
        assert_eq!(picked, app_override);

        // 2. ROFI_THEME is a full global override when no app override exists.
        std::fs::remove_file(&app_override).unwrap();
        let picked = resolve_theme_file_in(
            "calendar",
            &config,
            std::slice::from_ref(&data),
            Some(&global),
            &runtime,
            &theme,
        )
        .unwrap();
        assert_eq!(picked, global);

        // 3. The user base theme is layered under the app layout.
        let picked = resolve_theme_file_in(
            "calendar",
            &config,
            std::slice::from_ref(&data),
            None,
            &runtime,
            &theme,
        )
        .unwrap();
        assert_eq!(picked, runtime.join("theme-calendar.rasi"));
        let generated = std::fs::read_to_string(&picked).unwrap();
        assert!(generated.contains("@import"));
        assert!(generated.contains(base.to_str().unwrap()));
        assert!(!generated.contains("background-color"));

        // 4. Packaged fallback when the user has no base theme.
        std::fs::remove_file(&base).unwrap();
        let picked = resolve_theme_file_in(
            "calendar",
            &config,
            std::slice::from_ref(&data),
            None,
            &runtime,
            &theme,
        )
        .unwrap();
        assert_eq!(picked, packaged);

        // 5. Built-in fallback when nothing user-owned exists.
        std::fs::remove_file(&packaged).unwrap();
        let picked = resolve_theme_file_in(
            "calendar",
            &config,
            std::slice::from_ref(&data),
            None,
            &runtime,
            &theme,
        )
        .unwrap();
        assert_eq!(picked, runtime.join("theme-calendar.rasi"));
        let generated = std::fs::read_to_string(&picked).unwrap();
        assert!(generated.contains("element selected"));
        assert!(generated.contains("background-color: #1d2021;"));
    }

    #[test]
    fn theme_cache_does_not_rewrite_unchanged_files() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let tmp = tempfile::tempdir().unwrap();
        let first = write_generated_theme_in(tmp.path(), "calendar", "same\n").unwrap();
        let inode = std::fs::metadata(&first).unwrap().ino();
        let second = write_generated_theme_in(tmp.path(), "calendar", "same\n").unwrap();
        assert_eq!(first, second);
        assert_eq!(std::fs::metadata(&second).unwrap().ino(), inode);
        assert_eq!(
            std::fs::metadata(&second).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
