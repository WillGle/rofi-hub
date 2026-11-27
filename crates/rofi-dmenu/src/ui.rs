//! Shared Pango markup components for every native app.
//!
//! The goal is one grammar: app label, hero, muted meta, section headers,
//! rows with a leading/trailing slot, and a muted footer hint. Apps may add
//! their own body rows, but the chrome comes from here.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
         <span foreground='{text}' weight='bold' size='18000'>{hero}</span>\n\
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
///
/// Title and metadata deliberately inherit the row foreground. Rofi applies
/// the same Pango markup to normal and selected rows, so hardcoding `text` or
/// `muted` here would stay light on a light selected background.
pub fn row_with_leading(
    leading: &str,
    leading_color: &str,
    title: &str,
    detail: &str,
    trailing: &str,
    _theme: &Theme,
) -> String {
    let mut out = String::new();
    if !leading.is_empty() {
        out.push_str(&format!(
            "<span foreground='{leading_color}'>{}</span> ",
            escape_markup(leading)
        ));
    }
    out.push_str(&format!(
        "<span weight='600'>{}</span>",
        escape_markup(title)
    ));
    if !detail.is_empty() {
        out.push_str(&format!(
            "  <span alpha='72%'>{}</span>",
            escape_markup(detail)
        ));
    }
    if !trailing.is_empty() {
        out.push_str(&format!(
            "  <span alpha='72%'>{}</span>",
            escape_markup(trailing)
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

/// Compact textual action hints: `[key label]` with the key in the accent role.
///
/// Use this inside prompts and confirmations. Primary app screens should use
/// real RASI buttons for mouse actions; Rofi cannot attach click actions to
/// arbitrary markup.
pub fn toolbar(actions: &[(&str, &str)], theme: &Theme) -> String {
    let parts: Vec<String> = actions
        .iter()
        .map(|(key, label)| {
            let key = escape_markup(key);
            let label = escape_markup(label);
            format!(
                "<span foreground='{accent}' weight='bold'>[</span>\
                 <span foreground='{accent}' weight='bold'>{key}</span>\
                 <span foreground='{text}'>{separator}{label}</span>\
                 <span foreground='{accent}' weight='bold'>]</span>",
                accent = theme.accent,
                text = theme.text,
                separator = if label.is_empty() { "" } else { " " },
            )
        })
        .collect();
    parts.join("  ")
}

/// A detail row with a fixed-width label and a value: `IP  192.0.2.42/24`.
pub fn detail_row(label: &str, value: &str, theme: &Theme) -> String {
    format!(
        "<span foreground='{muted}'>{:>10}</span>  <span foreground='{text}'>{}</span>",
        escape_markup(label),
        escape_markup(value),
        muted = theme.muted,
        text = theme.text,
    )
}

/// A compact line of label/value pairs: `IP 192.0.2.42/24   Gateway 192.0.2.1`.
pub fn inline_detail(pairs: &[(&str, &str)], theme: &Theme) -> String {
    let parts: Vec<String> = pairs
        .iter()
        .map(|(label, value)| {
            format!(
                "<span foreground='{muted}'>{}</span> <span foreground='{text}'>{}</span>",
                escape_markup(label),
                escape_markup(value),
                muted = theme.muted,
                text = theme.text,
            )
        })
        .collect();
    parts.join("   ")
}

/// A clipboard/history row: index, colored kind glyph, title, muted metadata.
///
/// The index and detail inherit the row foreground with reduced alpha, so a
/// selected row stays readable on themes with a light selection band.
pub fn entry_row(
    index: usize,
    icon: &str,
    icon_color: &str,
    title: &str,
    detail: &str,
    _theme: &Theme,
) -> String {
    let mut out = format!("<span alpha='55%'>{index:>3}</span>");
    if !icon.is_empty() {
        out.push_str(&format!(
            "  <span foreground='{icon_color}'>{}</span>",
            escape_markup(icon)
        ));
    }
    out.push_str(&format!(
        "  <span weight='600'>{}</span>",
        escape_markup(title)
    ));
    if !detail.is_empty() {
        out.push_str(&format!(
            "  <span alpha='72%'>{}</span>",
            escape_markup(detail)
        ));
    }
    out
}

/// Widest display width across rendered rows.
pub fn widest(items: &[crate::Item]) -> usize {
    items
        .iter()
        .map(|item| display_width(&item.display))
        .max()
        .unwrap_or(0)
}

/// Widest display width across the lines of a markup block.
pub fn widest_text(text: &str) -> usize {
    text.lines().map(display_width).max().unwrap_or(0)
}

/// Rofi arguments that override the generated window width for this launch.
///
/// The base theme keeps a conservative `ch` width for stability; apps with
/// short content (calendar) or unusually long rows (clipboard) can fit the
/// window to the rows they actually render.
pub fn width_override(width_ch: usize) -> [String; 2] {
    ["-theme-str".to_owned(), width_rule(width_ch)]
}

/// The same width rule without Rofi arguments, for apps that combine several
/// theme overrides into one `-theme-str`.
pub fn width_rule(width_ch: usize) -> String {
    format!("window {{ width: calc( ( {width_ch}ch + 48px ) min 94% ); }}")
}

/// Comma-separated indexes of rows marked active. Rofi's `-a` option applies
/// the fixed active-row style used for section headers.
pub fn active_rows(items: &[crate::Item]) -> String {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.active)
        .map(|(index, _)| index.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Fixed appearance for active section headers across normal, alternate, and
/// selected states. Clicking or navigating onto a header must not paint the
/// regular selected-row outline.
pub fn static_header_theme(theme: &Theme) -> String {
    format!(
        "element normal.active, element alternate.active, element selected.active {{\n\
         \x20   background-color: {surface};\n\
         \x20   text-color: {text};\n\
         \x20   border: 0px 0px 1px 0px;\n\
         \x20   border-color: {border};\n\
         \x20   border-radius: 4px;\n\
         }}",
        surface = theme.surface_alt,
        text = theme.text,
        border = theme.border,
    )
}

/// Truncate to a display width, reserving one cell for an ellipsis.
pub fn truncate(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let budget = max_width.saturating_sub(1);
    let mut out = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + char_width > budget {
            out.push('\u{2026}');
            break;
        }
        out.push(ch);
        width += char_width;
    }
    out
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
    fn row_text_inherits_the_selected_foreground() {
        let theme = theme();
        let text = row("\u{25b6}", "Title", "detail", "trailing", &theme);
        assert!(
            text.contains(&theme.accent),
            "leading marker keeps its role"
        );
        assert!(
            !text.contains(&theme.text),
            "title must inherit the row color"
        );
        assert!(
            !text.contains(&theme.muted),
            "metadata must inherit the row color"
        );
        assert!(text.contains("alpha='72%'"), "{text}");
    }

    #[test]
    fn active_header_style_is_identical_for_selected_state() {
        let theme = theme();
        let rule = static_header_theme(&theme);
        for state in ["normal.active", "alternate.active", "selected.active"] {
            assert!(rule.contains(state), "{state}: {rule}");
        }
        assert!(rule.contains(&theme.surface_alt));
        assert!(rule.contains("border-radius: 4px"));
        assert!(!rule.contains("selected-normal"));
    }

    #[test]
    fn active_rows_lists_header_indexes() {
        let items = vec![
            crate::Item::new("section", "Section").active(),
            crate::Item::new("row", "Row"),
            crate::Item::new("section2", "Section 2").active(),
        ];
        assert_eq!(active_rows(&items), "0,2");
        assert_eq!(active_rows(&[]), "");
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
    fn toolbar_renders_key_and_label() {
        let text = toolbar(&[("Alt+d", "Delete"), ("Alt+x", "Clear")], &theme());
        assert!(text.contains("Alt+d"));
        assert!(text.contains("Delete"));
        assert!(text.contains("Alt+x"));
        assert!(!text.contains('\u{00b7}'));
        assert!(text.contains('['));
    }

    #[test]
    fn inline_detail_keeps_pairs_on_one_line() {
        let text = inline_detail(
            &[("IP", "192.0.2.42/24"), ("Gateway", "192.0.2.1")],
            &theme(),
        );
        assert!(text.contains("IP"));
        assert!(text.contains("192.0.2.42/24"));
        assert!(text.contains("Gateway"));
        assert!(text.contains("192.0.2.1"));
    }

    #[test]
    fn entry_row_escapes_text_and_keeps_index() {
        let theme = theme();
        let text = entry_row(7, "\u{25a7}", "#83a598", "a & b", "<40 KiB>", &theme);
        assert!(text.contains("  7"));
        assert!(text.contains("a &amp; b"));
        assert!(text.contains("&lt;40 KiB&gt;"));
        assert!(text.contains("#83a598"));
        assert!(
            !text.contains(&theme.text),
            "title must inherit the row color"
        );
        assert!(
            !text.contains(&theme.muted),
            "metadata must inherit the row color"
        );
    }

    #[test]
    fn widest_measures_markup_rows_and_multiline_text() {
        let items = vec![
            crate::Item::new("a", "<b>short</b>"),
            crate::Item::new("b", "a longer row"),
        ];
        assert_eq!(widest(&items), 12);
        assert_eq!(widest_text("short\nlonger line"), 11);
        let args = width_override(42);
        assert_eq!(args[0], "-theme-str");
        assert!(args[1].contains("42ch"), "{}", args[1]);
    }

    #[test]
    fn truncate_reserves_space_for_the_ellipsis() {
        assert_eq!(truncate("hello", 10), "hello");
        let short = truncate("hello world", 6);
        assert_eq!(short, "hello\u{2026}");
        assert_eq!(display_width(&short), 6);
        assert_eq!(truncate("hello", 0), "");
    }

    #[test]
    fn detail_row_aligns_short_labels() {
        let text = detail_row("IP", "192.0.2.42/24", &theme());
        assert!(text.contains("        IP"));
        assert!(text.contains("192.0.2.42/24"));
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
