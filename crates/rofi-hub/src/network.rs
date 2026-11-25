//! NetworkManager menu for `rofi-hub network`.
//!
//! The menu speaks to NetworkManager through `nmcli` and renders with the same
//! Rofi config/theme as the rest of the hub. It intentionally keeps the state
//! machine small: scan, connect (including a password prompt), saved profiles,
//! wired devices, radio toggle, rescan, and disconnect.

use std::env;
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
    ConnectEthernet(String),
    DisconnectRole { iface: String, wifi: bool },
    Rescan,
    ConnectWired,
    EditSaved,
    WifiOn,
    WifiOff,
    Quit,
}

#[derive(Debug, Default)]
struct WifiState {
    enabled: bool,
    iface: Option<String>,
    active_ssid: Option<String>,
    active: Option<AccessPoint>,
    details: Option<ConnectionDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectionDetails {
    connection: String,
    address: Option<String>,
    gateway: Option<String>,
    subnet: Option<String>,
    broadcast: Option<String>,
    dns: Vec<String>,
    mac: Option<String>,
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
            // The hub's own theme file uses the hub profile (96ch/20 lines).
            // Network needs its own geometry so the larger cards still fit.
            options.theme = if choice.custom_path {
                choice.path.clone()
            } else {
                rofi_core::theme::prepare(choice.mode, &choice.theme, "network")
            };
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
        let meta = header_meta(&state);
        let status_line = if status.is_empty() {
            String::new()
        } else {
            format!("\n{}", ui::status(&status, Level::Info, &choice.theme))
        };
        let summary = state
            .details
            .as_ref()
            .map(|details| {
                ui::inline_detail(
                    &[
                        ("IP", details.address.as_deref().unwrap_or("--")),
                        ("Gateway", details.gateway.as_deref().unwrap_or("--")),
                    ],
                    &choice.theme,
                )
            })
            .unwrap_or_default();
        let mesg = format!(
            "{}\n{}\n{}",
            ui::header("NETWORK", &hero, &meta, &choice.theme),
            summary,
            status_line
        );
        let fitted_width = ui::widest(&items).max(ui::widest_text(&mesg)).clamp(48, 96) + 2;
        options.extra_args = vec![
            "-selected-row".to_owned(),
            "1".to_owned(),
            "-mesg".to_owned(),
            mesg,
        ];
        options.extra_args.extend(ui::width_override(fitted_width));

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
            Action::ConnectWired => {
                let Some(iface) = ethernet_ifaces().into_iter().find(|iface| !iface.connected)
                else {
                    status = "No wired interface available.".to_owned();
                    continue;
                };
                nmcli(&["device", "connect", &iface.name])?;
                status = format!("Connected {}.", iface.name);
            }
            Action::EditSaved => edit_saved(&rofi, &choice, no_theme, &mut status)?,
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

fn header_meta(state: &WifiState) -> String {
    let iface = state.iface.as_deref().unwrap_or("no wireless adapter");
    if !state.enabled {
        return format!("Wi-Fi off \u{00b7} {iface}");
    }
    let ssid = state.active_ssid.as_deref().or_else(|| {
        state
            .details
            .as_ref()
            .map(|details| details.connection.as_str())
    });
    let Some(ssid) = ssid else {
        return format!("Wi-Fi on \u{00b7} {iface}");
    };
    let signal = state
        .active
        .as_ref()
        .map_or_else(|| "--".to_owned(), |ap| ap.signal.to_string());
    let band = state.active.as_ref().map(AccessPoint::band).unwrap_or("");
    let band = if band.is_empty() {
        String::new()
    } else {
        format!(", {band}")
    };
    let security = state
        .active
        .as_ref()
        .map(|ap| ap.security.as_str())
        .filter(|value| !value.is_empty() && *value != "--")
        .unwrap_or("--");
    format!("Wi-Fi: {ssid} ({signal}%{band}) \u{00b7} {security} \u{00b7} {iface}")
}

fn edit_saved(rofi: &Rofi, choice: &Choice, no_theme: bool, status: &mut String) -> Result<()> {
    let saved = saved_connections();
    if saved.is_empty() {
        *status = "No saved connections.".to_owned();
        return Ok(());
    }
    let items: Vec<Item> = saved
        .iter()
        .map(|saved| {
            Item::new(
                saved.name.clone(),
                ui::row("\u{f02e}", &saved.name, &saved.kind, "", &choice.theme),
            )
        })
        .collect();
    let mut options = DmenuOptions::new("EDIT CONNECTION");
    if !no_theme {
        options.theme = choice.path.clone();
    }
    options.config = rofi_config();
    let runtime = paths::hub_runtime_dir();
    let _ = std::fs::create_dir_all(&runtime);
    options.pid = Some(runtime.join("rofi-network-edit.pid"));
    options.extra_args = vec![
        "-mesg".to_owned(),
        ui::header(
            "EDIT CONNECTION",
            "Saved profiles",
            "Enter opens `nmcli connection edit` in a terminal",
            &choice.theme,
        ),
    ];
    if let Outcome::Selected(name) = rofi.dmenu(&items, &options)? {
        edit_connection(&name, status)?;
    }
    Ok(())
}

fn edit_connection(name: &str, status: &mut String) -> Result<()> {
    let mut candidates = Vec::new();
    if let Ok(terminal) = env::var("TERMINAL")
        && !terminal.trim().is_empty()
    {
        candidates.push(terminal);
    }
    candidates.extend(["foot", "kitty", "alacritty", "wezterm", "xterm"].map(str::to_owned));
    let Some(terminal) = candidates
        .into_iter()
        .find(|candidate| which::find(candidate).is_some())
    else {
        *status = format!("No terminal found; run `nmcli connection edit id {name}`.");
        return Ok(());
    };
    let mut command = Command::new(&terminal);
    if terminal.contains("wezterm") {
        command.args(["start", "--"]);
    } else {
        command.arg("-e");
    }
    command
        .args(["nmcli", "connection", "edit", "id", name])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match command.spawn() {
        Ok(_) => *status = format!("Opened editor for {name}."),
        Err(error) => *status = format!("Could not open a terminal: {error}"),
    }
    Ok(())
}

fn build_menu(state: &WifiState, theme: &Theme) -> (Vec<Item>, Vec<Action>) {
    let mut items = Vec::new();
    let mut actions = Vec::new();
    let ethernet = ethernet_ifaces();
    push_section(&mut items, &mut actions, "QUICK ACTIONS", "", theme);
    push_row(
        &mut items,
        &mut actions,
        Item::new(
            "wired",
            ui::row("\u{f00c}", "Connect wired network", "", "", theme),
        ),
        Action::ConnectWired,
    );
    push_row(
        &mut items,
        &mut actions,
        Item::new(
            "new-network",
            ui::row("\u{f067}", "Connect new network", "", "", theme),
        ),
        Action::ConnectHidden,
    );
    push_row(
        &mut items,
        &mut actions,
        Item::new(
            "rescan",
            ui::row("\u{f021}", "Scan for networks", "", "", theme),
        ),
        Action::Rescan,
    );
    if state.enabled {
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

    if let Some(details) = &state.details
        && (details.address.is_some() || details.mac.is_some())
    {
        push_section(&mut items, &mut actions, "CONNECTION", "", theme);
        let dns = if details.dns.is_empty() {
            "--".to_owned()
        } else {
            details.dns.join(", ")
        };
        push_detail(
            &mut items,
            &mut actions,
            "Subnet",
            details.subnet.as_deref().unwrap_or("--"),
            theme,
        );
        push_detail(
            &mut items,
            &mut actions,
            "Broadcast",
            details.broadcast.as_deref().unwrap_or("--"),
            theme,
        );
        push_detail(&mut items, &mut actions, "DNS", &dns, theme);
        push_detail(
            &mut items,
            &mut actions,
            "MAC",
            details.mac.as_deref().unwrap_or("--"),
            theme,
        );
    }

    if state.enabled {
        if let Some(iface) = &state.iface {
            let access_points = access_points(iface);
            push_section(
                &mut items,
                &mut actions,
                "NEARBY WI-FI",
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
                let open = ap.security.is_empty()
                    || ap.security == "--"
                    || ap.security.eq_ignore_ascii_case("open");
                let trailing = if ap.active {
                    "\u{2713}".to_owned()
                } else if open {
                    String::new()
                } else {
                    "\u{f023}".to_owned()
                };
                let band = ap.band();
                let detail = if band.is_empty() {
                    format!(
                        "{}% \u{00b7} {}",
                        ap.signal,
                        if open { "open" } else { &ap.security }
                    )
                } else {
                    format!(
                        "{}% \u{00b7} {band} \u{00b7} {}",
                        ap.signal,
                        if open { "open" } else { &ap.security }
                    )
                };
                push_row(
                    &mut items,
                    &mut actions,
                    Item::new(
                        ap.ssid.clone(),
                        ui::row_with_leading(
                            signal_glyph(ap.signal),
                            &theme.browser,
                            &ap.ssid,
                            &detail,
                            &trailing,
                            theme,
                        ),
                    ),
                    Action::ConnectAp {
                        ssid: ap.ssid,
                        security: ap.security,
                    },
                );
            }
        } else {
            push_section(
                &mut items,
                &mut actions,
                "NEARBY WI-FI",
                "no adapter",
                theme,
            );
            push_row(
                &mut items,
                &mut actions,
                Item::new("no-wifi", ui::empty("No Wi-Fi adapter detected", theme)),
                Action::WifiOff,
            );
        }
    }

    push_section(&mut items, &mut actions, "MANAGE", "", theme);
    if let Some(ssid) = &state.active_ssid {
        push_row(
            &mut items,
            &mut actions,
            Item::new(
                "disconnect",
                ui::row(
                    "\u{f00d}",
                    &format!("Disconnect current network ({ssid})"),
                    "",
                    "",
                    theme,
                ),
            ),
            Action::DisconnectRole {
                iface: state.iface.clone().unwrap_or_default(),
                wifi: true,
            },
        );
    }
    push_row(
        &mut items,
        &mut actions,
        Item::new(
            "edit-saved",
            ui::row("\u{f02e}", "Edit saved network", "", "", theme),
        ),
        Action::EditSaved,
    );

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

fn push_row(items: &mut Vec<Item>, actions: &mut Vec<Action>, mut item: Item, action: Action) {
    // Rofi returns the row id; keep it numeric so the action index is stable
    // even when labels or section ids change.
    item.id = items.len().to_string();
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
    let mut item = Item::new(
        format!("section-{}", label.to_ascii_lowercase()),
        ui::section(label, meta, 78, theme),
    )
    .active()
    .nonselectable();
    item.id = items.len().to_string();
    items.push(item);
    actions.push(Action::Quit);
}

fn push_detail(
    items: &mut Vec<Item>,
    actions: &mut Vec<Action>,
    label: &str,
    value: &str,
    theme: &Theme,
) {
    let mut item = Item::new(
        format!("detail-{}", label.to_ascii_lowercase()),
        ui::detail_row(label, value, theme),
    )
    .nonselectable();
    item.id = items.len().to_string();
    items.push(item);
    actions.push(Action::Quit);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AccessPoint {
    active: bool,
    ssid: String,
    signal: u8,
    security: String,
    freq: u32,
}

impl AccessPoint {
    fn band(&self) -> &'static str {
        match self.freq {
            5925.. => "6 GHz",
            4900.. => "5 GHz",
            2400.. => "2.4 GHz",
            _ => "",
        }
    }
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
        state.active = aps.iter().find(|ap| ap.active).cloned();
        state.active_ssid = state.active.as_ref().map(|ap| ap.ssid.clone());
        state.details = connection_details(iface);
    }
    Ok(state)
}

fn access_points(iface: &str) -> Vec<AccessPoint> {
    let output = nmcli(&[
        "-t",
        "-f",
        "IN-USE,SSID,SIGNAL,SECURITY,FREQ",
        "device",
        "wifi",
        "list",
        "ifname",
        iface,
        "--rescan",
        "no",
    ])
    .unwrap_or_default();
    let mut access_points: Vec<AccessPoint> = output
        .lines()
        .filter_map(|line| {
            let fields = split_fields(line, 5);
            if fields.len() < 5 || fields[1].is_empty() {
                return None;
            }
            Some(AccessPoint {
                active: fields[0] == "*",
                ssid: fields[1].clone(),
                signal: fields[2].parse().unwrap_or(0),
                security: fields[3].clone(),
                freq: fields[4].parse().unwrap_or(0),
            })
        })
        .collect();
    dedupe_access_points(&mut access_points);
    access_points
}

/// One row per SSID: a scan often returns several BSSIDs with the same name,
/// which makes a text menu unreadable. Prefer the active BSSID, then the
/// strongest signal.
fn dedupe_access_points(access_points: &mut Vec<AccessPoint>) {
    let mut deduped: Vec<AccessPoint> = Vec::with_capacity(access_points.len());
    for ap in access_points.drain(..) {
        if let Some(existing) = deduped.iter_mut().find(|existing| existing.ssid == ap.ssid) {
            let replace = (ap.active && !existing.active)
                || (ap.active == existing.active && ap.signal > existing.signal);
            if replace {
                *existing = ap;
            }
        } else {
            deduped.push(ap);
        }
    }
    *access_points = deduped;
}

fn connection_details(iface: &str) -> Option<ConnectionDetails> {
    let output = nmcli(&[
        "-t",
        "-f",
        "GENERAL.CONNECTION,GENERAL.HWADDR,IP4.ADDRESS,IP4.GATEWAY,IP4.DNS",
        "device",
        "show",
        iface,
    ])
    .ok()?;
    let mut details = ConnectionDetails {
        connection: iface.to_owned(),
        address: None,
        gateway: None,
        subnet: None,
        broadcast: None,
        dns: Vec::new(),
        mac: None,
    };
    for line in output.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match key {
            "GENERAL.CONNECTION" => details.connection = value.to_owned(),
            "GENERAL.HWADDR" => details.mac = Some(value.to_owned()),
            "IP4.ADDRESS[1]" | "IP4.ADDRESS" => details.address = Some(value.to_owned()),
            "IP4.GATEWAY" => details.gateway = Some(value.to_owned()),
            key if key.starts_with("IP4.DNS") => details.dns.push(value.to_owned()),
            _ => {}
        }
    }
    if let Some(address) = &details.address
        && let Some((subnet, broadcast)) = ipv4_network(address)
    {
        details.subnet = Some(subnet);
        details.broadcast = Some(broadcast);
    }
    Some(details)
}

fn ipv4_network(address: &str) -> Option<(String, String)> {
    let (ip, prefix) = address.split_once('/')?;
    let ip: std::net::Ipv4Addr = ip.parse().ok()?;
    let prefix: u32 = prefix.parse().ok()?;
    if prefix > 32 {
        return None;
    }
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    let ip = u32::from(ip);
    Some((
        std::net::Ipv4Addr::from(ip & mask).to_string(),
        std::net::Ipv4Addr::from((ip & mask) | !mask).to_string(),
    ))
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
        let rows = "*:Home\\:5G:82:WPA2:5180\n :Cafe:41:--:2412\n";
        let aps: Vec<AccessPoint> = rows
            .lines()
            .filter_map(|line| {
                let fields = split_fields(line, 5);
                (fields.len() == 5 && !fields[1].is_empty()).then(|| AccessPoint {
                    active: fields[0] == "*",
                    ssid: fields[1].clone(),
                    signal: fields[2].parse().unwrap_or(0),
                    security: fields[3].clone(),
                    freq: fields[4].parse().unwrap_or(0),
                })
            })
            .collect();
        assert_eq!(aps[0].ssid, "Home:5G");
        assert_eq!(aps[0].signal, 82);
        assert_eq!(aps[0].band(), "5 GHz");
        assert_eq!(aps[1].security, "--");
        assert_eq!(aps[1].band(), "2.4 GHz");
    }

    #[test]
    fn dedupes_access_points_by_ssid_and_prefers_active() {
        let mut aps = vec![
            AccessPoint {
                active: false,
                ssid: "Home".to_owned(),
                signal: 90,
                security: "WPA2".to_owned(),
                freq: 5180,
            },
            AccessPoint {
                active: true,
                ssid: "Home".to_owned(),
                signal: 70,
                security: "WPA2".to_owned(),
                freq: 2412,
            },
            AccessPoint {
                active: false,
                ssid: "Cafe".to_owned(),
                signal: 40,
                security: "--".to_owned(),
                freq: 2412,
            },
        ];
        dedupe_access_points(&mut aps);
        assert_eq!(aps.len(), 2);
        assert_eq!(aps[0].ssid, "Home");
        assert!(aps[0].active);
        assert_eq!(aps[0].signal, 70);
        assert_eq!(aps[1].ssid, "Cafe");
    }

    #[test]
    fn computes_ipv4_subnet_and_broadcast() {
        assert_eq!(
            ipv4_network("192.0.2.42/24"),
            Some(("192.0.2.0".to_owned(), "192.0.2.255".to_owned()))
        );
        assert_eq!(
            ipv4_network("10.0.0.5/8"),
            Some(("10.0.0.0".to_owned(), "10.255.255.255".to_owned()))
        );
        assert_eq!(ipv4_network("bad"), None);
    }

    #[test]
    fn signal_glyphs_cover_the_range() {
        assert_eq!(signal_glyph(100), "\u{2588}");
        assert_eq!(signal_glyph(0), "\u{2581}");
    }
}
