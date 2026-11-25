//! NetworkManager menu for `rofi-hub network`.
//!
//! The menu speaks to NetworkManager through `nmcli` and renders with the same
//! Rofi config/theme as the rest of the hub. It intentionally keeps the state
//! machine small: scan, connect (including a password prompt), saved profiles,
//! wired devices, radio toggle, rescan, and disconnect.

use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use rofi_core::{DmenuOptions, Item, Level, Outcome, Rofi, Theme, paths, ui, which};

use crate::config::Config;
use crate::hub::rofi_config;
use crate::theme::{self, Choice};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    ConnectAp { ssid: String, security: String },
    ConnectHidden,
    Saved(String),
    ConnectEthernet(String),
    DisconnectRole { iface: String, wifi: bool },
    Rescan,
    WifiOn,
    WifiOff,
    Quit,
}

#[derive(Debug, Default)]
struct WifiState {
    enabled: bool,
    iface: Option<String>,
    active_ssid: Option<String>,
}

pub fn run(theme_override: Option<&str>, no_theme: bool, config: &Config) -> Result<()> {
    which::require("nmcli")?;
    let rofi = Rofi::find()?;
    let choice = theme::resolve(theme_override, config)?;
    let mut status = String::new();

    loop {
        let state = wifi_state()?;
        let (items, actions) = build_menu(&state, &choice.theme);
        let mut options = DmenuOptions::new("NETWORK");
        if !no_theme {
            options.theme = choice.path.clone();
        }
        options.config = rofi_config();
        let runtime = paths::hub_runtime_dir();
        let _ = std::fs::create_dir_all(&runtime);
        options.pid = Some(runtime.join("rofi-network.pid"));
        let hero = state.active_ssid.clone().unwrap_or_else(|| {
            if state.enabled {
                "Wi-Fi on".to_owned()
            } else {
                "Wi-Fi off".to_owned()
            }
        });
        let meta = state
            .iface
            .clone()
            .unwrap_or_else(|| "no wireless adapter".to_owned());
        let status_line = if status.is_empty() {
            String::new()
        } else {
            format!("\n{}", ui::status(&status, Level::Info, &choice.theme))
        };
        options.extra_args = vec![
            "-selected-row".to_owned(),
            "1".to_owned(),
            "-mesg".to_owned(),
            format!(
                "{}{}",
                ui::header("NETWORK", &hero, &meta, &choice.theme),
                status_line
            ),
        ];

        let selected = match rofi.dmenu(&items, &options)? {
            Outcome::Selected(id) => id,
            Outcome::Cancelled | Outcome::Custom(_) => return Ok(()),
        };
        let index: usize = selected
            .parse()
            .with_context(|| format!("network menu returned invalid row `{selected}`"))?;
        let action = actions
            .get(index)
            .cloned()
            .with_context(|| format!("network menu row {index} is out of range"))?;
        status.clear();

        match action {
            Action::Quit => return Ok(()),
            Action::WifiOn => {
                nmcli(&["radio", "wifi", "on"])?;
                status = "Wi-Fi turned on.".to_owned();
            }
            Action::WifiOff => {
                nmcli(&["radio", "wifi", "off"])?;
                status = "Wi-Fi turned off.".to_owned();
            }
            Action::Rescan => {
                if let Some(iface) = &state.iface {
                    nmcli(&["device", "wifi", "rescan", "ifname", iface])?;
                    status = format!("Requested a Wi-Fi scan on {iface}.");
                }
            }
            Action::DisconnectRole { iface, wifi } => {
                nmcli(&["device", "disconnect", &iface])?;
                status = format!(
                    "Disconnected {} {iface}.",
                    if wifi { "Wi-Fi" } else { "Ethernet" }
                );
            }
            Action::ConnectEthernet(iface) => {
                nmcli(&["device", "connect", &iface])?;
                status = format!("Connected {iface}.");
            }
            Action::Saved(name) => {
                nmcli(&["connection", "up", &name])?;
                status = format!("Activated saved connection {name}.");
            }
            Action::ConnectHidden => {
                let Some(iface) = &state.iface else {
                    status = "No Wi-Fi adapter detected.".to_owned();
                    continue;
                };
                let Some(ssid) = prompt(&rofi, &choice, no_theme, "Hidden network SSID", false)?
                else {
                    continue;
                };
                if ssid.trim().is_empty() {
                    continue;
                }
                let password = prompt(&rofi, &choice, no_theme, "Password (blank if open)", true)?;
                connect_ap(
                    iface,
                    ssid.trim(),
                    password.as_deref().unwrap_or(""),
                    true,
                    &mut status,
                )?;
            }
            Action::ConnectAp { ssid, security } => {
                let Some(iface) = &state.iface else {
                    status = "No Wi-Fi adapter detected.".to_owned();
                    continue;
                };
                let open = security.is_empty()
                    || security == "--"
                    || security.eq_ignore_ascii_case("open");
                let password = if open {
                    None
                } else {
                    prompt(
                        &rofi,
                        &choice,
                        no_theme,
                        &format!("Password for {ssid}"),
                        true,
                    )?
                };
                if !open && password.as_deref().map(str::is_empty).unwrap_or(true) {
                    status = "Password entry cancelled.".to_owned();
                    continue;
                }
                connect_ap(
                    iface,
                    &ssid,
                    password.as_deref().unwrap_or(""),
                    false,
                    &mut status,
                )?;
            }
        }
    }
}

fn connect_ap(
    iface: &str,
    ssid: &str,
    password: &str,
    hidden: bool,
    status: &mut String,
) -> Result<()> {
    let mut args = vec!["device", "wifi", "connect", ssid, "ifname", iface];
    if hidden {
        args.extend(["hidden", "yes"]);
    }
    let password_owned;
    if !password.is_empty() {
        password_owned = password.to_owned();
        args.extend(["password", &password_owned]);
    }
    match nmcli(&args) {
        Ok(_) => {
            *status = format!("Connected to {ssid}.");
            Ok(())
        }
        Err(error) => {
            *status = format!("Could not connect to {ssid}: {error}");
            Ok(())
        }
    }
}

fn build_menu(state: &WifiState, theme: &Theme) -> (Vec<Item>, Vec<Action>) {
    let mut items = Vec::new();
    let mut actions = Vec::new();

    if state.enabled {
        if let Some(iface) = &state.iface {
            let access_points = access_points(iface);
            push_section(
                &mut items,
                &mut actions,
                "WI-FI",
                &format!("{} networks", access_points.len()),
                theme,
            );
            if access_points.is_empty() {
                push_row(
                    &mut items,
                    &mut actions,
                    Item::new("empty", ui::empty("No Wi-Fi networks visible", theme))
                        .nonselectable(),
                    Action::Rescan,
                );
            }
            for ap in access_points {
                let open = ap.security.is_empty() || ap.security == "--";
                let lock = if open { "" } else { "\u{f023}" };
                let active = if ap.active { "\u{25cf}" } else { "" };
                push_row(
                    &mut items,
                    &mut actions,
                    Item::new(
                        ap.ssid.clone(),
                        ui::row_with_leading(
                            signal_glyph(ap.signal),
                            &theme.browser,
                            &ap.ssid,
                            &format!(
                                "{}% \u{00b7} {}",
                                ap.signal,
                                if open { "open" } else { "secured" }
                            ),
                            if active.is_empty() { lock } else { active },
                            theme,
                        ),
                    ),
                    Action::ConnectAp {
                        ssid: ap.ssid,
                        security: ap.security,
                    },
                );
            }
            push_row(
                &mut items,
                &mut actions,
                Item::new(
                    "rescan",
                    ui::row("\u{f021}", "Scan for networks", "", "", theme),
                ),
                Action::Rescan,
            );
            push_row(
                &mut items,
                &mut actions,
                Item::new(
                    "hidden",
                    ui::row("\u{f023}", "Connect to hidden network", "", "", theme),
                ),
                Action::ConnectHidden,
            );
            if let Some(ssid) = &state.active_ssid {
                push_row(
                    &mut items,
                    &mut actions,
                    Item::new(
                        "disconnect",
                        ui::row(
                            "\u{f00d}",
                            &format!("Disconnect from {ssid}"),
                            "",
                            "",
                            theme,
                        ),
                    ),
                    Action::DisconnectRole {
                        iface: iface.clone(),
                        wifi: true,
                    },
                );
            }
            push_row(
                &mut items,
                &mut actions,
                Item::new(
                    "wifi-off",
                    ui::row("\u{f00d}", "Turn Wi-Fi off", "", "", theme),
                ),
                Action::WifiOff,
            );
        } else {
            push_section(&mut items, &mut actions, "WI-FI", "no adapter", theme);
            push_row(
                &mut items,
                &mut actions,
                Item::new("no-wifi", ui::empty("No Wi-Fi adapter detected", theme)),
                Action::WifiOff,
            );
        }
    } else {
        push_section(&mut items, &mut actions, "WI-FI", "disabled", theme);
        push_row(
            &mut items,
            &mut actions,
            Item::new(
                "wifi-on",
                ui::row("\u{f00c}", "Turn Wi-Fi on", "", "", theme),
            ),
            Action::WifiOn,
        );
    }

    let ethernet = ethernet_ifaces();
    if !ethernet.is_empty() {
        push_section(
            &mut items,
            &mut actions,
            "ETHERNET",
            &format!("{} interfaces", ethernet.len()),
            theme,
        );
    }
    for iface in ethernet {
        if iface.connected {
            push_row(
                &mut items,
                &mut actions,
                Item::new(
                    format!("eth-down:{}", iface.name),
                    ui::row(
                        "\u{f00d}",
                        &format!("Disconnect Ethernet {}", iface.name),
                        &iface.state,
                        "",
                        theme,
                    ),
                ),
                Action::DisconnectRole {
                    iface: iface.name,
                    wifi: false,
                },
            );
        } else {
            push_row(
                &mut items,
                &mut actions,
                Item::new(
                    format!("eth-up:{}", iface.name),
                    ui::row(
                        "\u{f00c}",
                        &format!("Connect Ethernet {}", iface.name),
                        &iface.state,
                        "",
                        theme,
                    ),
                ),
                Action::ConnectEthernet(iface.name),
            );
        }
    }

    let saved = saved_connections();
    if !saved.is_empty() {
        push_section(
            &mut items,
            &mut actions,
            "SAVED",
            &format!("{} profiles", saved.len()),
            theme,
        );
    }
    for saved in saved {
        push_row(
            &mut items,
            &mut actions,
            Item::new(
                format!("saved:{}", saved.name),
                ui::row("\u{f02e}", &saved.name, &saved.kind, "", theme),
            ),
            Action::Saved(saved.name),
        );
    }

    if items.is_empty() {
        push_row(
            &mut items,
            &mut actions,
            Item::new("empty", ui::empty("No NetworkManager devices found", theme)),
            Action::Quit,
        );
    }
    push_row(
        &mut items,
        &mut actions,
        Item::new(
            "hint",
            ui::hint(&[("Enter", "select"), ("esc", "close")], theme),
        )
        .nonselectable(),
        Action::Quit,
    );
    (items, actions)
}

fn push_row(items: &mut Vec<Item>, actions: &mut Vec<Action>, item: Item, action: Action) {
    items.push(item);
    actions.push(action);
}

fn push_section(
    items: &mut Vec<Item>,
    actions: &mut Vec<Action>,
    label: &str,
    meta: &str,
    theme: &Theme,
) {
    items.push(
        Item::new(
            format!("section-{}", label.to_ascii_lowercase()),
            ui::section(label, meta, 90, theme),
        )
        .nonselectable(),
    );
    actions.push(Action::Quit);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AccessPoint {
    active: bool,
    ssid: String,
    signal: u8,
    security: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Ethernet {
    name: String,
    state: String,
    connected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Saved {
    name: String,
    kind: String,
}

fn wifi_state() -> Result<WifiState> {
    let enabled = nmcli(&["radio", "wifi"])?
        .trim()
        .eq_ignore_ascii_case("enabled");
    let devices = nmcli(&["-t", "-f", "DEVICE,TYPE,STATE", "device"])?;
    let mut state = WifiState {
        enabled,
        ..WifiState::default()
    };
    for line in devices.lines() {
        let fields = split_fields(line, 3);
        if fields.len() == 3 && fields[1] == "wifi" {
            state.iface = Some(fields[0].clone());
            break;
        }
    }
    if let Some(iface) = &state.iface {
        let aps = access_points(iface);
        state.active_ssid = aps.iter().find(|ap| ap.active).map(|ap| ap.ssid.clone());
        // `access_points` is intentionally only the visible list; an active
        // hidden network is still reported by `device wifi list` when present.
    }
    Ok(state)
}

fn access_points(iface: &str) -> Vec<AccessPoint> {
    let output = nmcli(&[
        "-t",
        "-f",
        "IN-USE,SSID,SIGNAL,SECURITY",
        "device",
        "wifi",
        "list",
        "ifname",
        iface,
        "--rescan",
        "no",
    ])
    .unwrap_or_default();
    output
        .lines()
        .filter_map(|line| {
            let fields = split_fields(line, 4);
            if fields.len() < 4 || fields[1].is_empty() {
                return None;
            }
            Some(AccessPoint {
                active: fields[0] == "*",
                ssid: fields[1].clone(),
                signal: fields[2].parse().unwrap_or(0),
                security: fields[3].clone(),
            })
        })
        .collect()
}

fn ethernet_ifaces() -> Vec<Ethernet> {
    let output = nmcli(&["-t", "-f", "DEVICE,TYPE,STATE", "device"]).unwrap_or_default();
    output
        .lines()
        .filter_map(|line| {
            let fields = split_fields(line, 3);
            if fields.len() == 3 && fields[1] == "ethernet" && fields[2] != "unmanaged" {
                Some(Ethernet {
                    name: fields[0].clone(),
                    state: fields[2].clone(),
                    connected: fields[2] == "connected",
                })
            } else {
                None
            }
        })
        .collect()
}

fn saved_connections() -> Vec<Saved> {
    let output = nmcli(&["-t", "-f", "NAME,TYPE", "connection", "show"]).unwrap_or_default();
    output
        .lines()
        .filter_map(|line| {
            let fields = split_fields(line, 2);
            if fields.len() == 2
                && matches!(fields[1].as_str(), "802-11-wireless" | "802-3-ethernet")
            {
                Some(Saved {
                    name: fields[0].clone(),
                    kind: if fields[1] == "802-11-wireless" {
                        "Wi-Fi".to_owned()
                    } else {
                        "Ethernet".to_owned()
                    },
                })
            } else {
                None
            }
        })
        .collect()
}

/// Split nmcli's colon-separated output, honoring backslash escapes.
fn split_fields(line: &str, limit: usize) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == ':' && fields.len() + 1 < limit {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

fn signal_glyph(signal: u8) -> &'static str {
    match signal {
        80..=100 => "\u{2588}",
        60..=79 => "\u{2586}",
        40..=59 => "\u{2584}",
        20..=39 => "\u{2582}",
        _ => "\u{2581}",
    }
}

fn prompt(
    rofi: &Rofi,
    choice: &Choice,
    no_theme: bool,
    prompt: &str,
    password: bool,
) -> Result<Option<String>> {
    let mut command = Command::new(rofi.binary());
    command.args(["-dmenu", "-p", prompt]);
    if password {
        command.arg("-password");
    }
    if !no_theme && let Some(path) = &choice.path {
        command.arg("-theme").arg(path);
    }
    if let Some(config) = rofi_config() {
        command.arg("-config").arg(config);
    }
    let output = command
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("cannot run Rofi prompt for {prompt}"))?;
    match output.status.code() {
        Some(0) => Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        )),
        Some(1) | None => Ok(None),
        Some(code) => bail!("Rofi prompt exited with status {code}"),
    }
}

fn nmcli(args: &[&str]) -> Result<String> {
    let output = Command::new("nmcli")
        .args(args)
        .output()
        .with_context(|| format!("cannot run nmcli {}", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("nmcli {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_escaped_nmcli_fields() {
        assert_eq!(
            split_fields("wlan0:wifi:connected", 3),
            vec!["wlan0", "wifi", "connected"]
        );
        assert_eq!(
            split_fields(r"*:My\:Network:82:WPA2", 4),
            vec!["*", "My:Network", "82", "WPA2"]
        );
    }

    #[test]
    fn parses_wifi_list_rows() {
        let rows = "*:Home\\:5G:82:WPA2\n :Cafe:41:--\n";
        let aps: Vec<AccessPoint> = rows
            .lines()
            .filter_map(|line| {
                let fields = split_fields(line, 4);
                (fields.len() == 4 && !fields[1].is_empty()).then(|| AccessPoint {
                    active: fields[0] == "*",
                    ssid: fields[1].clone(),
                    signal: fields[2].parse().unwrap_or(0),
                    security: fields[3].clone(),
                })
            })
            .collect();
        assert_eq!(aps[0].ssid, "Home:5G");
        assert_eq!(aps[0].signal, 82);
        assert_eq!(aps[1].security, "--");
    }

    #[test]
    fn signal_glyphs_cover_the_range() {
        assert_eq!(signal_glyph(100), "\u{2588}");
        assert_eq!(signal_glyph(0), "\u{2581}");
    }
}
