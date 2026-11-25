//! Clipboard history picker backed by `cliphist` and `wl-copy`.
//!
//! The list is kept out of the Rofi row ids: each row carries its index, so
//! clipboard contents cannot break the dmenu metadata protocol.

use std::io::Write;
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result, bail};
use rofi_core::{Item, Outcome, Rofi, Theme, app, ui, which};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rofi-clipboard: {error:#}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    /// The exact line printed by `cliphist list`, consumed by `decode`.
    raw: String,
    preview: String,
}

fn parse_entries(stdout: &str) -> Vec<Entry> {
    stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let preview = line
                .split_once('\t')
                .map(|(_, preview)| preview)
                .unwrap_or(line)
                .replace('\t', " ");
            let preview = if preview.trim().is_empty() {
                "(empty)".to_owned()
            } else {
                preview
            };
            Entry {
                raw: line.to_owned(),
                preview,
            }
        })
        .collect()
}

fn run() -> Result<()> {
    let cliphist = which::require("cliphist")?;
    let wl_copy = which::require("wl-copy")?;

    let output = Command::new(&cliphist)
        .arg("list")
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("cannot run {}", cliphist.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("cliphist list failed: {}", stderr.trim());
    }

    let entries = parse_entries(&String::from_utf8_lossy(&output.stdout));
    let theme = Theme::active();
    let rofi = Rofi::find()?;
    let mut options = app::dmenu_options("clipboard", "CLIPBOARD");
    options.extra_args = vec![
        "-mesg".to_owned(),
        ui::header(
            "CLIPBOARD",
            &format!("{} items", entries.len()),
            "cliphist history",
            &theme,
        ),
    ];

    if entries.is_empty() {
        let items = vec![
            Item::new("empty", ui::empty("Clipboard history is empty", &theme)).nonselectable(),
            Item::new(
                "hint",
                ui::hint(&[("Enter", "copy"), ("esc", "close")], &theme),
            )
            .nonselectable(),
        ];
        let _ = rofi.dmenu(&items, &options)?;
        return Ok(());
    }

    let mut items: Vec<Item> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            Item::new(
                index.to_string(),
                ui::row(&format!("{:>3}", index + 1), &entry.preview, "", "", &theme),
            )
        })
        .collect();
    items.push(
        Item::new(
            "hint",
            ui::hint(&[("Enter", "copy"), ("esc", "close")], &theme),
        )
        .nonselectable(),
    );
    let index = match rofi.dmenu(&items, &options)? {
        Outcome::Selected(id) if id == "hint" => return Ok(()),
        Outcome::Selected(id) => id
            .parse::<usize>()
            .with_context(|| format!("rofi selected an unknown clipboard row: {id}"))?,
        Outcome::Cancelled | Outcome::Custom(_) => return Ok(()),
    };
    let Some(entry) = entries.get(index) else {
        bail!("rofi selected clipboard row {index}, which is out of range");
    };

    let contents = decode(&cliphist, &entry.raw)?;
    copy(&wl_copy, &contents)
}

fn decode(cliphist: &std::path::Path, raw: &str) -> Result<Vec<u8>> {
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

fn copy(wl_copy: &std::path::Path, contents: &[u8]) -> Result<()> {
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
        assert_eq!(
            entries,
            vec![
                Entry {
                    raw: "123\tHello\tworld".to_owned(),
                    preview: "Hello world".to_owned(),
                },
                Entry {
                    raw: "456\t".to_owned(),
                    preview: "(empty)".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn ignores_blank_lines() {
        assert!(parse_entries("\n\n").is_empty());
    }
}
