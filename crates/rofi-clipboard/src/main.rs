//! Clipboard history picker backed by `cliphist` and `wl-copy`.
//!
//! The list is kept out of the Rofi row ids: each row carries its index, so
//! clipboard contents cannot break the dmenu metadata protocol.

use std::io::Write;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result, bail};
use rofi_dmenu::{
    ChromeProfile, DmenuOptions, Item, MenuInput, Outcome, Rofi, Theme, app, ui, which,
};

const TITLE_WIDTH: usize = 72;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rofi-clipboard: {error:#}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    Link,
    Image,
    Binary,
}

impl Kind {
    fn icon(self) -> &'static str {
        match self {
            Self::Text => "\u{2261}",
            Self::Link => "\u{2197}",
            Self::Image => "\u{25a7}",
            Self::Binary => "\u{25eb}",
        }
    }

    fn color(self, theme: &Theme) -> &str {
        match self {
            Self::Text => &theme.accent,
            Self::Link => &theme.browser,
            Self::Image => &theme.entertainment,
            Self::Binary => &theme.communication,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    /// The exact line printed by `cliphist list`, consumed by `decode`.
    raw: String,
    /// One-line, human-readable preview.
    title: String,
    /// Type-specific metadata shown after the preview.
    detail: String,
    kind: Kind,
}

fn parse_entries(stdout: &str) -> Vec<Entry> {
    stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let preview = line
                .split_once('\t')
                .map(|(_, preview)| preview)
                .unwrap_or(line);
            let preview = normalize_preview(preview);
            if preview.is_empty() {
                return Entry {
                    raw: line.to_owned(),
                    title: "(empty)".to_owned(),
                    detail: "empty".to_owned(),
                    kind: Kind::Text,
                };
            }
            let (kind, title, detail) = classify(&preview);
            Entry {
                raw: line.to_owned(),
                title,
                detail,
                kind,
            }
        })
        .collect()
}

/// Collapse control characters and whitespace so a clipboard line stays one
/// visible row. The raw line is kept separately for `cliphist decode`.
fn normalize_preview(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last_space = false;
    for ch in value.chars() {
        if is_invisible_format(ch) {
            continue;
        }
        let ch = if ch.is_control() { ' ' } else { ch };
        if ch == ' ' {
            if last_space {
                continue;
            }
            last_space = true;
        } else {
            last_space = false;
        }
        out.push(ch);
    }
    out.trim().to_owned()
}

fn is_invisible_format(ch: char) -> bool {
    matches!(
        ch,
        '\u{200b}'
            | '\u{200c}'
            | '\u{200d}'
            | '\u{2060}'
            | '\u{feff}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

fn classify(preview: &str) -> (Kind, String, String) {
    if let Some(inner) = preview
        .strip_prefix("[[ binary data ")
        .and_then(|value| value.strip_suffix(" ]]"))
    {
        let parts: Vec<&str> = inner.split_whitespace().collect();
        let image = parts.iter().any(|part| {
            matches!(
                part.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp"
            )
        });
        let kind = if image { Kind::Image } else { Kind::Binary };
        return (
            kind,
            if image {
                "Image".to_owned()
            } else {
                "Binary data".to_owned()
            },
            binary_detail(&parts),
        );
    }

    if preview.starts_with("http://") || preview.starts_with("https://") {
        return (
            Kind::Link,
            ui::truncate(preview, TITLE_WIDTH),
            String::new(),
        );
    }

    (
        Kind::Text,
        ui::truncate(preview, TITLE_WIDTH),
        String::new(),
    )
}

fn binary_detail(parts: &[&str]) -> String {
    let Some(first) = parts.first() else {
        return "binary".to_owned();
    };
    let mut fields = Vec::new();
    if let Some(unit) = parts.get(1) {
        fields.push(format!("{first} {unit}"));
    } else {
        fields.push((*first).to_owned());
    }
    if let Some(kind) = parts.get(2) {
        fields.push((*kind).to_owned());
    }
    if let Some(dimensions) = parts.get(3) {
        fields.push(dimensions.replace('x', "\u{00d7}"));
    }
    fields.join(" \u{00b7} ")
}

fn count_label(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

fn header(entries: &[Entry], theme: &Theme) -> String {
    let mut text = 0;
    let mut links = 0;
    let mut images = 0;
    let mut binary = 0;
    for entry in entries {
        match entry.kind {
            Kind::Text => text += 1,
            Kind::Link => links += 1,
            Kind::Image => images += 1,
            Kind::Binary => binary += 1,
        }
    }
    let mut stats = Vec::new();
    if text > 0 {
        stats.push(format!("{text} text"));
    }
    if links > 0 {
        stats.push(count_label(links, "link", "links"));
    }
    if images > 0 {
        stats.push(count_label(images, "image", "images"));
    }
    if binary > 0 {
        stats.push(format!("{binary} binary"));
    }
    let meta = if stats.is_empty() {
        "cliphist history".to_owned()
    } else {
        stats.join(" \u{00b7} ")
    };
    format!(
        "{}\n{}\n",
        ui::header(
            "CLIPBOARD",
            &format!("{} items", entries.len()),
            &meta,
            theme
        ),
        ui::hint(
            &[("Enter", "copy"), ("Ctrl+1–4", "theme"), ("esc", "close")],
            theme,
        ),
    )
}

fn empty_header(theme: &Theme) -> String {
    format!(
        "{}\n{}\n",
        ui::header(
            "CLIPBOARD",
            "No history",
            "Copy something, then reopen this picker",
            theme
        ),
        ui::hint(
            &[
                ("Alt+r", "refresh"),
                ("Ctrl+1–4", "theme"),
                ("esc", "close")
            ],
            theme,
        ),
    )
}

/// Real RASI buttons replace the old textual action chips. Each button points
/// at the same Rofi key used by the keyboard path.
fn clipboard_toolbar(theme: &Theme, refresh_only: bool) -> String {
    let buttons = if refresh_only {
        "button-refresh"
    } else {
        "button-copy, button-delete, button-clear, button-refresh"
    };
    format!(
        r#"
mainbox {{
    children: [ inputbar, message, box-clipboard-actions, listview ];
    spacing: 12px;
}}
box-clipboard-actions {{
    background-color: transparent;
    orientation: horizontal;
    expand: false;
    spacing: 10px;
    padding: 0 0 2px;
    children: [ {buttons} ];
}}
button-copy, button-delete, button-clear, button-refresh {{
    expand: false;
    padding: 6px 11px;
    margin: 0 0 2px;
    border: 1px;
    border-radius: 6px;
    border-color: {muted};
    background-color: {mantle};
    text-color: {text};
}}
button-copy {{
    content: "Copy Enter";
    action: "kb-accept-entry";
    border-color: {accent};
}}
button-delete {{
    content: "Delete Alt+d";
    action: "kb-custom-3";
}}
button-clear {{
    content: "Clear Alt+x";
    action: "kb-custom-1";
    border-color: {danger};
    text-color: {danger};
}}
button-refresh {{
    content: "Refresh Alt+r";
    action: "kb-custom-2";
}}
button selected,
button selected.normal {{
    background-color: {accent};
    text-color: {base};
}}
button-clear selected,
button-clear selected.normal {{
    background-color: {danger};
    text-color: {base};
}}
"#,
        muted = theme.muted,
        mantle = theme.mantle,
        text = theme.text,
        accent = theme.accent,
        danger = theme.danger,
        base = theme.base,
    )
}

fn render_items(entries: &[Entry], theme: &Theme) -> Vec<Item> {
    let width = ChromeProfile::for_app("clipboard")
        .width_ch
        .saturating_sub(26);
    let mut items = Vec::with_capacity(entries.len() + 1);
    items.push(
        Item::new(
            "section-history",
            ui::section(
                "HISTORY",
                &format!("{} entries", entries.len()),
                width,
                theme,
            ),
        )
        .active()
        .nonselectable(),
    );
    for (index, entry) in entries.iter().enumerate() {
        // Dmenu matches the row text. Keep the original preview searchable
        // even when its formatted display has been shortened.
        let preview = entry
            .raw
            .split_once('\t')
            .map_or(entry.raw.as_str(), |(_, preview)| preview);
        let preview = preview.replace(['\0', '\n', '\x1f'], " ");
        items.push(Item::new(
            format!("{index}:{preview}"),
            ui::entry_row(
                index + 1,
                entry.kind.icon(),
                entry.kind.color(theme),
                &entry.title,
                &entry.detail,
                theme,
            ),
        ));
    }
    items
}

fn empty_items(theme: &Theme) -> Vec<Item> {
    vec![
        Item::new("empty", ui::empty("Clipboard history is empty", theme)).nonselectable(),
        Item::new(
            "hint",
            ui::hint(
                &[
                    ("Alt+r", "refresh"),
                    ("Ctrl+1–4", "theme"),
                    ("esc", "close"),
                ],
                theme,
            ),
        )
        .nonselectable(),
    ]
}

fn main_options(
    items: &[Item],
    entries: &[Entry],
    theme: &Theme,
    selected_row: usize,
) -> DmenuOptions {
    let mut options = app::dmenu_options("clipboard", "FILTER");
    options.extra_args = vec![
        "-selected-row".to_owned(),
        selected_row.to_string(),
        "-mesg".to_owned(),
        header(entries, theme),
        "-a".to_owned(),
        ui::active_rows(items),
        "-kb-custom-1".to_owned(),
        "Alt+x".to_owned(),
        "-kb-custom-2".to_owned(),
        "Alt+r".to_owned(),
        "-kb-custom-3".to_owned(),
        "Alt+d".to_owned(),
    ];
    options.theme_str.push(ui::static_header_theme(theme));
    options.theme_str.push(clipboard_toolbar(theme, false));
    options
}

fn empty_options(theme: &Theme) -> DmenuOptions {
    let mut options = app::dmenu_options("clipboard", "CLIPBOARD");
    options.extra_args = vec![
        "-mesg".to_owned(),
        empty_header(theme),
        "-kb-custom-1".to_owned(),
        "Alt+r".to_owned(),
    ];
    options.theme_str.push(clipboard_toolbar(theme, true));
    options
}

fn confirm_header(theme: &Theme) -> String {
    format!(
        "{}\n{}\n",
        ui::header(
            "CLEAR HISTORY",
            "Remove all items?",
            "This cannot be undone",
            theme
        ),
        ui::toolbar(&[("\u{23ce}", "Select"), ("esc", "Cancel")], theme),
    )
}

fn fit_options(options: &mut DmenuOptions, items: &[Item], text: &str, min: usize, max: usize) {
    let width = ui::widest(items).max(ui::widest_text(text)).clamp(min, max) + 2;
    options.theme_str.push(ui::width_rule(width));
}

fn confirm_options(theme: &Theme) -> DmenuOptions {
    let mut options = app::dmenu_options("clipboard-confirm", "CLEAR HISTORY");
    options.extra_args = vec!["-mesg".to_owned(), confirm_header(theme)];
    options
        .theme_str
        .push("inputbar { enabled: false; }".to_owned());
    options
}

fn run() -> Result<()> {
    let cliphist = which::require("cliphist")?;
    let wl_copy = which::require("wl-copy")?;
    let (mut theme, mut theme_path) = app::initial_theme("clipboard");
    let rofi = Rofi::find()?;

    let mut filter = String::new();
    let mut focus = 1usize;
    loop {
        let entries = load_entries(&cliphist)?;
        if entries.is_empty() {
            let items = empty_items(&theme);
            let mut options = empty_options(&theme);
            options.theme = theme_path.clone();
            app::bind_theme_shortcuts(&mut options, 1);
            fit_options(&mut options, &items, &empty_header(&theme), 32, 80);
            match rofi.dmenu(&items, &options)? {
                Outcome::Custom(0) => continue,
                Outcome::Custom(binding) if binding >= 1 => {
                    if let Some((next, path)) = app::session_theme("clipboard", binding, 1) {
                        theme = next;
                        theme_path = path;
                    }
                    continue;
                }
                _ => return Ok(()),
            }
        }

        let items = render_items(&entries, &theme);
        let mut options = main_options(&items, &entries, &theme, focus);
        options.theme = theme_path.clone();
        app::bind_theme_shortcuts(&mut options, 3);
        fit_options(&mut options, &items, &header(&entries, &theme), 48, 112);
        options.allow_custom = true;
        options
            .extra_args
            .extend(["-filter".to_owned(), filter.clone()]);
        let (outcome, input) = rofi.dmenu_with_input(&items, &options)?;
        filter.clone_from(&input.text);
        if let Some(row) = input.row {
            focus = row;
        }
        match outcome {
            Outcome::Selected(id) => {
                let Some(index) = selected_entry_index(&items, &input, &id) else {
                    // Header rows and unknown custom text never copy or delete.
                    continue;
                };
                let Some(entry) = entries.get(index) else {
                    continue;
                };
                let contents = decode(&cliphist, &entry.raw)?;
                return copy(&wl_copy, &contents);
            }
            Outcome::Custom(0) => {
                if confirm_clear(&rofi, &theme, theme_path.as_deref())? {
                    run_cliphist(&cliphist, &["wipe"], None)?;
                }
            }
            Outcome::Custom(1) => {}
            Outcome::Custom(2) => {
                if let Some(index) = selected_entry_index(&items, &input, "")
                    && let Some(entry) = entries.get(index)
                {
                    run_cliphist(&cliphist, &["delete"], Some(&entry.raw))?;
                }
            }
            Outcome::Custom(binding) if binding >= 3 => {
                if let Some((next, path)) = app::session_theme("clipboard", binding, 3) {
                    theme = next;
                    theme_path = path;
                }
            }
            Outcome::Cancelled | Outcome::Custom(_) => return Ok(()),
        }
    }
}

/// Resolve the selected data row. Rofi's `i` output is preferred; the id also
/// starts with the entry index for wrappers that return only `-format s`.
fn selected_entry_index(items: &[Item], input: &MenuInput, id: &str) -> Option<usize> {
    if let Some(row) = input.row {
        let item = items.get(row)?;
        if item.nonselectable || item.id.starts_with("section-") {
            return None;
        }
        return item.id.split(':').next()?.parse().ok();
    }
    let index = id.split(':').next()?.parse::<usize>().ok()?;
    let exists = items.iter().any(|item| {
        !item.nonselectable && item.id.split(':').next().and_then(|v| v.parse().ok()) == Some(index)
    });
    exists.then_some(index)
}

fn load_entries(cliphist: &Path) -> Result<Vec<Entry>> {
    let output = Command::new(cliphist)
        .arg("list")
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("cannot run {}", cliphist.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("cliphist list failed: {}", stderr.trim());
    }
    Ok(parse_entries(&String::from_utf8_lossy(&output.stdout)))
}

fn confirm_clear(rofi: &Rofi, theme: &Theme, theme_path: Option<&std::path::Path>) -> Result<bool> {
    let items = vec![
        Item::new("no", ui::row("\u{2022}", "Cancel", "", "", theme)),
        Item::new(
            "yes",
            ui::row_with_leading("\u{2716}", &theme.danger, "Clear all", "", "", theme),
        ),
    ];
    let mut options = confirm_options(theme);
    if let Some(path) = theme_path {
        options.theme = Some(path.to_path_buf());
    }
    fit_options(&mut options, &items, &confirm_header(theme), 32, 80);
    match rofi.dmenu(&items, &options)? {
        Outcome::Selected(id) => Ok(id == "yes"),
        _ => Ok(false),
    }
}

fn run_cliphist(cliphist: &Path, args: &[&str], stdin: Option<&str>) -> Result<()> {
    let mut command = Command::new(cliphist);
    command
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .with_context(|| format!("cannot run {}", cliphist.display()))?;
    if let Some(input) = stdin
        && let Some(pipe) = child.stdin.as_mut()
    {
        pipe.write_all(input.as_bytes())?;
        pipe.write_all(b"\n")?;
    }
    drop(child.stdin.take());
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "cliphist {} failed: {}",
            args.first().unwrap_or(&""),
            stderr.trim()
        );
    }
    Ok(())
}

fn decode(cliphist: &Path, raw: &str) -> Result<Vec<u8>> {
    let mut child = Command::new(cliphist)
        .arg("decode")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("cannot run {}", cliphist.display()))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(raw.as_bytes())?;
        stdin.write_all(b"\n")?;
    }
    drop(child.stdin.take());

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("cliphist decode failed: {}", stderr.trim());
    }
    Ok(output.stdout)
}

fn copy(wl_copy: &Path, contents: &[u8]) -> Result<()> {
    let mut child = Command::new(wl_copy)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("cannot run {}", wl_copy.display()))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(contents)?;
    }
    drop(child.stdin.take());

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("wl-copy failed: {}", stderr.trim());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cliphist_lines_and_keeps_the_raw_entry() {
        let entries = parse_entries("123\tHello\tworld\n456\t\n");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].raw, "123\tHello\tworld");
        assert_eq!(entries[0].title, "Hello world");
        assert_eq!(entries[0].detail, "");
        assert_eq!(entries[0].kind, Kind::Text);
        assert_eq!(entries[1].raw, "456\t");
        assert_eq!(entries[1].title, "(empty)");
        assert_eq!(entries[1].detail, "empty");
    }

    #[test]
    fn strips_zero_width_formatting_from_urls() {
        let entries = parse_entries("1\th\u{200b}t\u{200b}t\u{200b}p\u{200b}s://example.com\n");
        assert_eq!(entries[0].title, "https://example.com");
        assert_eq!(entries[0].kind, Kind::Link);
    }

    #[test]
    fn ignores_blank_lines() {
        assert!(parse_entries("\n\n").is_empty());
    }

    #[test]
    fn classifies_urls_images_and_binary_previews() {
        let entries = parse_entries(
            "1\thttps://example.com/path\n2\t[[ binary data 40 KiB png 1524x513 ]]\n3\t[[ binary data 12 B pdf ]]\n",
        );
        assert_eq!(entries[0].kind, Kind::Link);
        assert_eq!(entries[0].title, "https://example.com/path");
        assert_eq!(entries[0].detail, "");
        assert_eq!(entries[1].kind, Kind::Image);
        assert_eq!(entries[1].title, "Image");
        assert_eq!(
            entries[1].detail,
            "40 KiB \u{00b7} png \u{00b7} 1524\u{00d7}513"
        );
        assert_eq!(entries[2].kind, Kind::Binary);
        assert_eq!(entries[2].title, "Binary data");
        assert_eq!(entries[2].detail, "12 B \u{00b7} pdf");
    }

    #[test]
    fn long_previews_are_truncated_for_the_row() {
        let long = "x".repeat(200);
        let entries = parse_entries(&format!("1\t{long}\n"));
        assert!(ui::display_width(&entries[0].title) <= TITLE_WIDTH);
        assert!(entries[0].title.ends_with('\u{2026}'));
    }
}
