//! Shared task categories and their semantic colors.

use crate::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Work,
    Browser,
    Communication,
    Entertainment,
    System,
    Other,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Work => "Work",
            Category::Browser => "Browser",
            Category::Communication => "Communication",
            Category::Entertainment => "Entertainment",
            Category::System => "System",
            Category::Other => "Other",
        }
    }

    pub fn color(self, theme: &Theme) -> &str {
        match self {
            Category::Work => &theme.work,
            Category::Browser => &theme.browser,
            Category::Communication => &theme.communication,
            Category::Entertainment => &theme.entertainment,
            Category::System => &theme.power,
            Category::Other => &theme.other,
        }
    }

    /// Map provider metadata (`Productivity`, `Utilities`, `System`, ...).
    pub fn from_label(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "productivity" | "work" => Category::Work,
            "browser" | "web" => Category::Browser,
            "communication" | "social" => Category::Communication,
            "entertainment" | "media" | "leisure" => Category::Entertainment,
            "system" | "utilities" | "utility" => Category::System,
            _ => Category::Other,
        }
    }

    /// Classify a screen-time app name using the same buckets as the tracker.
    pub fn from_app_name(name: &str) -> Self {
        let name = name.to_ascii_lowercase();
        match name.as_str() {
            "code" | "neovim" | "vim" | "emacs" | "terminal" | "kitty" | "alacritty"
            | "wezterm" | "foot" | "tmux" | "zellij" | "obsidian" | "libreoffice" => Category::Work,
            "firefox" | "chromium" | "chrome" | "brave" | "zen" | "qutebrowser" | "epiphany" => {
                Category::Browser
            }
            "discord" | "slack" | "telegram" | "signal" | "element" | "thunderbird" | "mail"
            | "zoom" => Category::Communication,
            "mpv" | "vlc" | "spotify" | "steam" | "gamescope" | "youtube" => {
                Category::Entertainment
            }
            "hyprland" | "waybar" | "rofi" | "systemd" | "gnome-shell" | "plasmashell" | "niri"
            | "sway" => Category::System,
            _ => Category::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_colors_are_stable() {
        let theme = Theme::gruvbox();
        assert_eq!(Category::Work.label(), "Work");
        assert_eq!(Category::Work.color(&theme), "#b8bb26");
        assert_eq!(Category::Browser.color(&theme), "#83a598");
        assert_eq!(Category::Communication.color(&theme), "#fe8019");
        assert_eq!(Category::Entertainment.color(&theme), "#d3869b");
        assert_eq!(Category::Other.color(&theme), "#928374");
        assert_eq!(Category::System.color(&theme), "#d5c4a1");
    }

    #[test]
    fn classifies_common_apps() {
        assert_eq!(Category::from_app_name("Firefox"), Category::Browser);
        assert_eq!(Category::from_app_name("discord"), Category::Communication);
        assert_eq!(Category::from_app_name("mpv"), Category::Entertainment);
        assert_eq!(Category::from_app_name("code"), Category::Work);
        assert_eq!(Category::from_app_name("unknown-app"), Category::Other);
        assert_eq!(Category::from_label("Productivity"), Category::Work);
        assert_eq!(Category::from_label("Utilities"), Category::System);
    }
}
