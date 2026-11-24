//! Rofi dmenu rows and Pango markup helpers.

/// Escape text before embedding it in Pango markup.
pub fn escape_markup(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A single Rofi dmenu row.
#[derive(Debug, Clone, Default)]
pub struct Item {
    /// Stable token returned by `rofi -format s`.
    pub id: String,
    /// Pango markup shown to the user. Newlines are encoded as `&#10;`.
    pub display: String,
    /// Optional icon-theme name or path, emitted as `\0icon`.
    pub icon: Option<String>,
    /// Optional meta text.
    pub meta: Option<String>,
    /// Section/header styling.
    pub active: bool,
    /// Warning styling (for example, a missing provider).
    pub urgent: bool,
    /// Ask Rofi not to select this row when supported.
    pub nonselectable: bool,
}

impl Item {
    pub fn new(id: impl Into<String>, display: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display: display.into(),
            ..Self::default()
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    pub fn active(mut self) -> Self {
        self.active = true;
        self
    }

    pub fn urgent(mut self) -> Self {
        self.urgent = true;
        self
    }

    pub fn nonselectable(mut self) -> Self {
        self.nonselectable = true;
        self
    }

    /// Encode one row using Rofi's dmenu metadata protocol.
    pub fn to_line(&self) -> String {
        let mut line = self.id.clone();
        line.push_str("\0display\x1f");
        line.push_str(&self.display.replace('\n', "&#10;"));

        if let Some(icon) = &self.icon {
            line.push_str("\0icon\x1f");
            line.push_str(icon);
        }
        if let Some(meta) = &self.meta {
            line.push_str("\0meta\x1f");
            line.push_str(meta);
        }
        if self.active {
            line.push_str("\0active\x1ftrue");
        }
        if self.urgent {
            line.push_str("\0urgent\x1ftrue");
        }
        if self.nonselectable {
            line.push_str("\0nonselectable\x1ftrue");
        }

        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_pango_markup() {
        assert_eq!(escape_markup("a & b < c > d"), "a &amp; b &lt; c &gt; d");
    }

    #[test]
    fn encodes_display_and_metadata() {
        let line = Item::new("apps", "<b>Apps</b>")
            .icon("application-x-executable")
            .active()
            .to_line();

        assert_eq!(
            line,
            "apps\0display\x1f<b>Apps</b>\0icon\x1fapplication-x-executable\0active\x1ftrue"
        );
    }

    #[test]
    fn encodes_newlines_for_pango() {
        let line = Item::new("apps", "Apps\nLauncher").to_line();
        assert!(line.contains("Apps&#10;Launcher"));
    }
}
