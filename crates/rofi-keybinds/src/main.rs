//! Hyprland keyboard shortcut reference.
//!
//! Every `bind` is shown with its section from `# --- Name ---` comments.
//! Shortcuts whose dispatcher is `exec` can be launched from the menu; all
//! other rows are reference only.

use std::collections::HashMap;
use std::fs;
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result, bail};
use rofi_core::{Item, Outcome, Rofi, Theme, app, paths, ui};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rofi-keybinds: {error:#}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Binding {
    section: String,
    combo: String,
    description: String,
    /// Only `exec` bindings are runnable from the menu.
    command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedBinding {
    section: String,
    combo: String,
    description: String,
    command: Option<String>,
    key: String,
    mods: String,
    dispatcher: String,
}

impl Binding {
    fn from_parsed(parsed: &ParsedBinding) -> Self {
        Self {
            section: parsed.section.clone(),
            combo: parsed.combo.clone(),
            description: parsed.description.clone(),
            command: parsed.command.clone(),
        }
    }
}

fn run() -> Result<()> {
    let config = paths::config_home().join("hypr/hyprland.conf");
    let text =
        fs::read_to_string(&config).with_context(|| format!("cannot read {}", config.display()))?;

    let bindings = parse_config(&text);
    if bindings.is_empty() {
        bail!("no bindings found in {}", config.display());
    }

    let theme = Theme::active();
    let items = build_items(&bindings, &theme);

    let rofi = Rofi::find()?;
    let mut options = app::dmenu_options("keybinds", "SHORTCUTS");
    options.extra_args = vec![
        "-selected-row".to_owned(),
        "1".to_owned(),
        "-mesg".to_owned(),
        ui::header(
            "SHORTCUTS",
            &format!("{} bindings", bindings.len()),
            "hyprland.conf",
            &theme,
        ),
    ];

    match rofi.dmenu(&items, &options)? {
        Outcome::Selected(id) if id == "hint" => Ok(()),
        Outcome::Selected(id) => {
            let index = id
                .parse::<usize>()
                .with_context(|| format!("rofi selected an unknown keybinding row: {id}"))?;
            let Some(binding) = bindings.get(index) else {
                bail!("rofi selected keybinding row {index}, which is out of range");
            };
            if let Some(command) = &binding.command {
                run_command(command)?;
            }
            Ok(())
        }
        Outcome::Cancelled | Outcome::Custom(_) => Ok(()),
    }
}

/// Parse `hyprland.conf` text into menu rows.
fn parse_config(text: &str) -> Vec<Binding> {
    let variables = collect_variables(text);
    let mut section = "Basic".to_owned();
    let mut bindings = Vec::new();
    let mut pending: Option<Pending> = None;

    for line in text.lines() {
        if let Some(title) = section_title(line) {
            flush(&mut pending, &mut bindings);
            section = title;
            continue;
        }

        let Some(payload) = bind_payload(line) else {
            continue;
        };
        let Some(parsed) = parse_binding(payload, &section, &variables) else {
            continue;
        };

        let extends_workspace = pending.as_ref().is_some_and(|pending| {
            pending.parsed.section == parsed.section
                && pending.parsed.mods == parsed.mods
                && pending.parsed.dispatcher == parsed.dispatcher
                && parsed.key.len() == 1
                && parsed.key.chars().all(|ch| ch.is_ascii_digit())
                && pending.last_key.len() == 1
                && parsed
                    .key
                    .parse::<u32>()
                    .ok()
                    .zip(pending.last_key.parse::<u32>().ok())
                    .is_some_and(|(next, last)| next == last + 1)
        });

        if extends_workspace {
            let pending = pending.as_mut().expect("checked above");
            pending.last_key = parsed.key.clone();
            let range = format!("{}..{}", pending.first_key, pending.last_key);
            pending.parsed.combo = combo(&parsed.mods, &range, &variables);
            pending.parsed.description = if parsed.dispatcher == "movetoworkspace" {
                format!("Move to WS {range}")
            } else {
                format!("Workspace {range}")
            };
            continue;
        }

        flush(&mut pending, &mut bindings);
        pending = Some(Pending {
            first_key: parsed.key.clone(),
            last_key: parsed.key.clone(),
            parsed,
        });
    }

    flush(&mut pending, &mut bindings);
    bindings
}

#[derive(Debug)]
struct Pending {
    parsed: ParsedBinding,
    first_key: String,
    last_key: String,
}

fn flush(pending: &mut Option<Pending>, out: &mut Vec<Binding>) {
    if let Some(pending) = pending.take() {
        out.push(Binding::from_parsed(&pending.parsed));
    }
}

fn parse_binding(
    payload: &str,
    section: &str,
    variables: &HashMap<String, String>,
) -> Option<ParsedBinding> {
    let fields: Vec<&str> = payload.split(',').map(str::trim).collect();
    if fields.len() < 3 {
        return None;
    }
    let mods = fields[0].to_owned();
    let key = fields[1].to_owned();
    let dispatcher = fields[2].to_owned();
    if key.is_empty() || dispatcher.is_empty() {
        return None;
    }
    let argument = fields[3..].join(",");
    let argument = argument.trim();
    let description = describe(&dispatcher, argument);
    let command = (dispatcher == "exec" && !argument.is_empty()).then(|| argument.to_owned());

    Some(ParsedBinding {
        section: section.to_owned(),
        combo: combo(&mods, &key, variables),
        description,
        command,
        key,
        mods,
        dispatcher,
    })
}

fn collect_variables(text: &str) -> HashMap<String, String> {
    let mut variables = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if !name.starts_with('$') || name.len() < 2 {
            continue;
        }
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        variables.insert(name.to_owned(), value.to_owned());
    }
    variables
}

fn section_title(line: &str) -> Option<String> {
    let line = line.trim_start();
    for (prefix, suffix) in [("# --- ", " ---"), ("# ── ", " ──")] {
        if let Some(rest) = line.strip_prefix(prefix) {
            let title = rest.strip_suffix(suffix).unwrap_or(rest).trim();
            if !title.is_empty() {
                return Some(title.to_owned());
            }
        }
    }
    None
}

fn bind_payload(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let rest = line.strip_prefix("bind")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=')?;
    Some(strip_inline_comment(rest).trim())
}

fn strip_inline_comment(value: &str) -> &str {
    let bytes = value.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'#' && index > 0 && bytes[index - 1].is_ascii_whitespace() {
            return &value[..index];
        }
    }
    value
}

fn combo(mods: &str, key: &str, variables: &HashMap<String, String>) -> String {
    let mut out = String::new();
    for raw in mods.split_whitespace() {
        let mut token = raw;
        if token.starts_with('$')
            && let Some(value) = variables.get(token)
        {
            token = value;
        }
        let name = match token.to_ascii_uppercase().as_str() {
            "$MOD" | "SUPER" | "MOD4" => "Super",
            "SHIFT" => "Shift",
            "ALT" => "Alt",
            "CTRL" | "CONTROL" => "Ctrl",
            _ => token,
        };
        out.push_str(name);
        out.push('+');
    }
    out.push_str(&key_name(key));
    out
}

fn key_name(key: &str) -> String {
    if let Some(rest) = key.strip_prefix("XF86") {
        return match rest {
            "AudioMute" => "Mute".to_owned(),
            "AudioLowerVolume" => "Vol-".to_owned(),
            "AudioRaiseVolume" => "Vol+".to_owned(),
            "AudioMicMute" => "MicMute".to_owned(),
            "MonBrightnessDown" => "Bright-".to_owned(),
            "MonBrightnessUp" => "Bright+".to_owned(),
            "TouchpadToggle" => "Touchpad".to_owned(),
            _ => rest.to_owned(),
        };
    }

    match key.to_ascii_lowercase().as_str() {
        "return" => "Enter".to_owned(),
        "escape" => "Esc".to_owned(),
        "period" => ".".to_owned(),
        "equal" => "=".to_owned(),
        "mouse:272" => "LMB".to_owned(),
        "mouse:273" => "RMB".to_owned(),
        "left" | "right" | "up" | "down" => {
            let mut chars = key.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
        _ => key.to_owned(),
    }
}

fn describe(dispatcher: &str, argument: &str) -> String {
    match dispatcher {
        "movefocus" => {
            let direction = match argument {
                "l" => "Left",
                "r" => "Right",
                "u" => "Up",
                "d" => "Down",
                other => other,
            };
            format!("Focus ({direction})")
        }
        "movetoworkspace" => format!("Move to WS {argument}"),
        "workspace" => format!("Workspace {argument}"),
        "killactive" => "Close Window".to_owned(),
        "togglefloating" => "Toggle Float".to_owned(),
        "exit" => "Exit Session".to_owned(),
        "exec" => exec_description(argument),
        _ if argument.is_empty() => dispatcher.to_owned(),
        _ => format!("{dispatcher} {argument}"),
    }
}

fn exec_description(command: &str) -> String {
    let contains = |needle: &str| command.contains(needle);
    let mapped = if contains("rofi-hub launcher") || contains("rofi-hub drun") {
        Some("App Launcher")
    } else if contains("rofi-hub calc") {
        Some("Calculator")
    } else if contains("rofi-hub emoji") {
        Some("Emoji Picker")
    } else if contains("rofi-hub network") {
        Some("Network Manager")
    } else if contains("rofi-hub clipboard") {
        Some("Clipboard History")
    } else if contains("rofi-hub window") {
        Some("Window Switcher")
    } else if contains("rofi-hub calendar") {
        Some("Calendar")
    } else if contains("rofi-hub power") {
        Some("Power Menu")
    } else if contains("toggle_waybar") {
        Some("Toggle Waybar")
    } else if contains("monitor-setup") {
        Some("Monitor Setup")
    } else if contains("theme-lock") {
        Some("Lock Screen")
    } else if contains("foot yazi") {
        Some("File Manager")
    } else if contains("rotate_select") {
        Some("Rotate Screen")
    } else if contains("toggle_touchpad") {
        Some("Toggle Touchpad")
    } else if contains("wpctl set-mute") {
        Some("Audio Mute")
    } else if contains("wpctl set-volume") && command.contains('-') {
        Some("Volume -")
    } else if contains("wpctl set-volume") && command.contains('+') {
        Some("Volume +")
    } else if contains("brightnessctl") && command.contains('-') {
        Some("Brightness -")
    } else if contains("brightnessctl") && command.contains('+') {
        Some("Brightness +")
    } else if contains("screenshot region") {
        Some("Screenshot (Area)")
    } else if contains("screenshot clip") {
        Some("Screenshot (Clipboard)")
    } else if contains("screenshot full") {
        Some("Screenshot (Full)")
    } else if contains("screen-rec region") {
        Some("Record (Area)")
    } else if contains("screen-rec toggle") {
        Some("Record (Full)")
    } else {
        None
    };

    match mapped {
        Some(label) => label.to_owned(),
        None if command.trim() == "foot" => "Terminal".to_owned(),
        None => command.to_owned(),
    }
}

fn build_items(bindings: &[Binding], theme: &Theme) -> Vec<Item> {
    let mut items = Vec::with_capacity(bindings.len() + 4);
    let mut section = "";
    for (index, binding) in bindings.iter().enumerate() {
        if binding.section != section {
            section = &binding.section;
            items.push(
                Item::new(
                    format!("section-{index}"),
                    ui::section(section, "", 98, theme),
                )
                .active()
                .nonselectable(),
            );
        }
        let marker = if binding.command.is_some() {
            "\u{25b6}"
        } else {
            "\u{2022}"
        };
        items.push(Item::new(
            index.to_string(),
            ui::row(marker, &binding.combo, &binding.description, "", theme),
        ));
    }
    items.push(
        Item::new(
            "hint",
            ui::hint(&[("Enter", "run"), ("esc", "close")], theme),
        )
        .nonselectable(),
    );
    items
}

fn run_command(command: &str) -> Result<()> {
    // The command comes from the user's own Hyprland config and may use shell
    // syntax. It is user-owned configuration, not provider input.
    Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("cannot run shortcut: {command}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
$mod = MOD4

# --- Basic ---
bind = $mod, Return, exec, foot
bind = $mod, Q, killactive
bind = $mod SHIFT, left, movefocus, l

# --- Workspaces ---
bind = $mod, 1, workspace, 1
bind = $mod, 2, workspace, 2
bind = $mod, 3, workspace, 3
bind = $mod SHIFT, 1, movetoworkspace, 1
bind = $mod SHIFT, 2, movetoworkspace, 2

# --- Media ---
bind = , XF86AudioMute, exec, wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle
bind = , XF86AudioLowerVolume, exec, wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-
"#;

    #[test]
    fn parses_sections_and_shortcuts() {
        let bindings = parse_config(SAMPLE);
        assert!(bindings.iter().any(|binding| {
            binding.section == "Basic"
                && binding.combo == "Super+Enter"
                && binding.description == "Terminal"
                && binding.command.as_deref() == Some("foot")
        }));
        assert!(bindings.iter().any(|binding| {
            binding.combo == "Super+Q" && binding.description == "Close Window"
        }));
        assert!(bindings.iter().any(|binding| {
            binding.combo == "Super+Shift+Left" && binding.description == "Focus (Left)"
        }));
        assert!(
            bindings
                .iter()
                .any(|binding| { binding.section == "Media" && binding.combo == "Mute" })
        );
    }

    #[test]
    fn folds_consecutive_workspace_digits() {
        let bindings = parse_config(SAMPLE);
        assert!(bindings.iter().any(|binding| {
            binding.combo == "Super+1..3" && binding.description == "Workspace 1..3"
        }));
        assert!(bindings.iter().any(|binding| {
            binding.combo == "Super+Shift+1..2" && binding.description == "Move to WS 1..2"
        }));
        assert!(!bindings.iter().any(|binding| binding.combo == "Super+2"));
    }

    #[test]
    fn strips_inline_comments_and_handles_empty_mods() {
        let bindings = parse_config(
            "bind = , escape, exec, ~/.local/bin/foo # comment\n\
             bind = SUPER, period, exec, ~/.local/bin/bar\n",
        );
        assert_eq!(bindings[0].combo, "Esc");
        assert_eq!(bindings[0].command.as_deref(), Some("~/.local/bin/foo"));
        assert_eq!(bindings[1].combo, "Super+.");
    }

    #[test]
    fn shows_hyprland_dispatch_descriptions() {
        assert_eq!(describe("workspace", "3"), "Workspace 3");
        assert_eq!(describe("movetoworkspace", "3"), "Move to WS 3");
        assert_eq!(describe("movefocus", "r"), "Focus (Right)");
        assert_eq!(describe("exec", "rofi-hub calendar"), "Calendar");
        assert_eq!(
            describe("exec", "wpctl set-volume @DEFAULT_SINK@ 5%-"),
            "Volume -"
        );
        assert_eq!(describe("exec", "screenshot region"), "Screenshot (Area)");
        assert_eq!(describe("resizeactive", "10 10"), "resizeactive 10 10");
    }

    #[test]
    fn menu_has_section_headers_and_clean_rows() {
        let bindings = parse_config(SAMPLE);
        let items = build_items(&bindings, &Theme::gruvbox());
        let section_rows: Vec<_> = items
            .iter()
            .filter(|item| item.display.contains("\u{258c}"))
            .collect();
        assert!(!section_rows.is_empty());
        assert!(section_rows.iter().all(|item| item.nonselectable));
        assert!(
            items
                .iter()
                .any(|item| item.display.contains("Super+Enter"))
        );
        assert!(!items.iter().any(|item| item.display.contains("[Basic]")));
    }
}
