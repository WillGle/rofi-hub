//! Shared Pango markup components for every native app.
//!
//! The goal is one grammar: app label, hero, muted meta, section headers,
//! rows with a leading/trailing slot, and a muted footer hint. Apps may add
//! their own body rows, but the chrome comes from here.

use unicode_width::UnicodeWidthStr;

use crate::Theme;
use crate::item::escape_markup;

/// Severity used by [`status`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Success,
    Warning,
    Danger,
}

/// `LABEL` accent, hero value, muted meta.
pub fn header(label: &str, hero: &str, meta: &str, theme: &Theme) -> String {
    let label = escape_markup(&label.to_uppercase());
    let hero = escape_markup(hero);
    let meta = escape_markup(meta);
    format!(
        "<span foreground='{accent}' weight='bold'>{label}</span>\n\
         <span foreground='{text}' weight='bold' size='26000'>{hero}</span>\n\
         <span foreground='{muted}'>{meta}</span>",
        accent = theme.accent,
        text = theme.text,
        muted = theme.muted,
    )
}

/// One-line header for apps whose Rofi height is fixed: `LABEL  HERO  META`
/// with explicit sizes so a large hero never leaks into the meta text.
pub fn header_inline(label: &str, hero: &str, meta: &str, theme: &Theme) -> String {
    let label = escape_markup(&label.to_uppercase());
    let hero = escape_markup(hero);
    let meta = escape_markup(meta);
    format!(
        "<span foreground='{accent}' weight='bold' size='12000'>{label}</span>   \
         <span foreground='{text}' weight='bold' size='26000'>{hero}</span>   \
         <span foreground='{muted}' size='12000'>{meta}</span>",
        accent = theme.accent,
        text = theme.text,
        muted = theme.muted,
    )
}

/// `▌ TITLE` on the left, muted meta right-aligned to `width` cells.
pub fn section(label: &str, meta: &str, width: usize, theme: &Theme) -> String {
    let label = format!("\u{258c} {}", label.to_uppercase());
    let plain = format!("{label}{meta}");
    let pad = width
        .saturating_sub(display_width(&plain))
        .saturating_sub(8)
        .max(1);
    format!(
        "<span foreground='{accent}' weight='bold'>{}</span>{}<span foreground='{muted}'>{}</span>",
        escape_markup(&label),
        " ".repeat(pad),
        escape_markup(meta),
        accent = theme.accent,
        muted = theme.muted,
    )
}

/// One row: leading marker, bold title, muted detail, right-aligned trailing.
pub fn row(leading: &str, title: &str, detail: &str, trailing: &str, theme: &Theme) -> String {
    row_with_leading(leading, &theme.accent, title, detail, trailing, theme)
}

/// A row whose leading marker uses an explicit semantic color.
pub fn row_with_leading(
    leading: &str,
    leading_color: &str,
    title: &str,
    detail: &str,
    trailing: &str,
    theme: &Theme,
) -> String {
    let mut out = String::new();
    if !leading.is_empty() {
        out.push_str(&format!(
            "<span foreground='{leading_color}'>{}</span> ",
            escape_markup(leading)
        ));
    }
    out.push_str(&format!(
        "<span foreground='{text}' weight='600'>{}</span>",
        escape_markup(title),
        text = theme.text
    ));
    if !detail.is_empty() {
        out.push_str(&format!(
            "  <span foreground='{muted}'>{}</span>",
            escape_markup(detail),
            muted = theme.muted
        ));
    }
    if !trailing.is_empty() {
        out.push_str(&format!(
            "  <span foreground='{muted}'>{}</span>",
            escape_markup(trailing),
            muted = theme.muted
        ));
    }
    out
}

/// A single muted footer line. `keys` is `(binding, description)`.
pub fn hint(keys: &[(&str, &str)], theme: &Theme) -> String {
    let parts: Vec<String> = keys
        .iter()
        .map(|(key, description)| {
            if description.is_empty() {
                escape_markup(key)
            } else {
                format!("{} {}", escape_markup(key), escape_markup(description))
            }
        })
        .collect();
    format!(
        "<span foreground='{muted}'>{}</span>",
        parts.join(" \u{00b7} "),
        muted = theme.muted
    )
}

/// Status line with a glyph and semantic color.
pub fn status(text: &str, level: Level, theme: &Theme) -> String {
    let (glyph, color) = match level {
        Level::Info => ("\u{2022}", theme.muted.as_str()),
        Level::Success => ("\u{25cf}", theme.work.as_str()),
        Level::Warning => ("\u{25b2}", theme.accent.as_str()),
        Level::Danger => ("\u{2716}", theme.danger.as_str()),
    };
    format!(
        "<span foreground='{color}'>{glyph}</span> <span foreground='{color}'>{}</span>",
        escape_markup(text)
    )
}

/// Empty-state line, muted and deliberately not a row action.
pub fn empty(text: &str, theme: &Theme) -> String {
    format!(
        "<span foreground='{muted}'>\u{2205} {}</span>",
        escape_markup(text),
        muted = theme.muted
    )
}

/// Visible width of Pango markup in terminal cells.
///
/// Tags and entities are removed before measuring; `unicode-width` handles
/// combining marks, wide CJK/emoji, and Nerd Font private-use glyphs.
pub fn display_width(text: &str) -> usize {
    let plain = strip_markup(text);
    UnicodeWidthStr::width(plain.as_str())
}

fn strip_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if in_tag => {}
            '&' => {
                let mut entity = String::new();
                while let Some(&next) = chars.peek() {
                    if next == ';' {
                        chars.next();
                        break;
                    }
                    entity.push(next);
                    chars.next();
                }
                let decoded = match entity.as_str() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "apos" | "#39" => "'",
                    "#10" => "\n",
                    _ => "",
                };
                out.push_str(decoded);
            }
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> Theme {
        Theme::gruvbox()
    }

    #[test]
    fn header_uses_roles_and_uppercases_the_label() {
        let text = header("rofi hub", "4 apps", "auto", &theme());
        assert!(text.contains("ROFI HUB"));
        assert!(text.contains("#fabd2f"));
        assert!(text.contains("#fbf1c7"));
        assert!(text.contains("#a89984"));
    }

    #[test]
    fn section_pads_to_width() {
        let text = section("Wi-Fi", "3", 20, &theme());
        assert!(text.contains("\u{258c} WI-FI"));
        assert!((12..=20).contains(&display_width(&text)));
    }

    #[test]
    fn row_escapes_user_text() {
        let text = row("\u{25b6}", "a & b", "<detail>", "ok", &theme());
        assert!(text.contains("a &amp; b"));
        assert!(text.contains("&lt;detail&gt;"));
    }

    #[test]
    fn hint_joins_bindings() {
        let text = hint(
            &[("\u{2191}/\u{2193}", "select"), ("esc", "close")],
            &theme(),
        );
        assert!(text.contains("\u{2191}/\u{2193} select"));
        assert!(text.contains("esc close"));
        assert!(text.contains("\u{00b7}"));
    }

    #[test]
    fn status_uses_danger_for_errors() {
        let text = status("rofi missing", Level::Danger, &theme());
        assert!(text.contains("#fb4934"));
        assert!(text.contains("\u{2716}"));
    }

    #[test]
    fn display_width_ignores_markup_and_entities() {
        assert_eq!(display_width("<b>Hello</b>"), 5);
        assert_eq!(display_width("a &amp; b"), 5);
        assert_eq!(display_width("a &lt; b"), 5);
    }

    #[test]
    fn display_width_handles_vietnamese_and_emoji() {
        assert!(display_width("Tiếng Việt") >= 9);
        assert!(display_width("⌨") >= 1);
        assert_eq!(display_width("\u{0301}"), 0);
    }

    #[test]
    fn empty_state_is_muted() {
        let text = empty("Clipboard history is empty", &theme());
        assert!(text.contains("Clipboard history is empty"));
        assert!(text.contains("#a89984"));
    }
}
