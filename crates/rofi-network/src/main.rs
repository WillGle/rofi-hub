//! Standalone NetworkManager menu for Rofi.
//!
//! `rofi-network` is an independent app: it resolves its own theme and config
//! and does not depend on the optional hub menu or installer.
//!
//! Available networks and connection actions share one filterable list.
//! Details and saved profiles expand inline. Network operations use `nmcli`.

use std::env;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use clap::Parser;
use rofi_dmenu::{
    DmenuOptions, Item, Level, MenuInput, Outcome, Rofi, Theme, app, escape_markup, paths, ui,
    which,
};

/// Visible width of the content area used by wrapped detail values.
const DETAIL_WIDTH: usize = 56;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    ConnectAp { ssid: String, security: String },
    ConnectHidden,
    ConnectEthernet(String),
    QuickConnectEthernet,
    DisconnectEthernet(String),
    DisconnectWifi,
    Rescan,
    EditSaved,
    EditSavedProfile(String),
    WifiOn,
    WifiOff,
    Noop,
}

#[derive(Debug, Default)]
struct WifiState {
    enabled: bool,
    iface: Option<String>,
    active_ssid: Option<String>,
    active: Option<AccessPoint>,
    access_points: Vec<AccessPoint>,
    ethernet: Vec<Ethernet>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ConnectionDetails {
    iface: String,
    address: Option<String>,
    gateway: Option<String>,
    ipv6: Option<String>,
    ipv6_gateway: Option<String>,
    vpn: String,
    subnet: Option<String>,
    broadcast: Option<String>,
    dns: Vec<String>,
    mac: Option<String>,
}

#[derive(Debug, Parser)]
#[command(name = "rofi-network", version, about = "NetworkManager menu for Rofi")]
struct Cli {
    /// Do not pass a generated theme to Rofi.
    #[arg(long)]
    no_theme: bool,
}

#[derive(Debug, Clone)]
struct Choice {
    theme: Theme,
    path: Option<std::path::PathBuf>,
}

fn resolve_choice() -> Choice {
    let (theme, path) = app::initial_theme("network");
    Choice { theme, path }
}

fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(cli) {
        eprintln!("rofi-network: {error:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    which::require("nmcli")?;
    let rofi = Rofi::find()?;
    let mut choice = resolve_choice();
    let no_theme = cli.no_theme;

    let mut status = String::new();
    let mut details = None;
    let mut saved = None;
    let mut focus = None;

    loop {
        let state = wifi_state()?;
        let mut input = MenuInput::default();
        let (outcome, actions) = loop {
            let (mut items, mut actions) = build_main(&state, &choice.theme);
            let first_action = items.len() + 1;
            items.push(section_row("actions", "Network actions", &choice.theme));
            add_actions(
                &state,
                saved.as_deref(),
                &choice.theme,
                &mut items,
                &mut actions,
            );

            let mut options = DmenuOptions::new("Find network or action");
            if !no_theme {
                options.theme = choice.path.clone();
            }
            options.config = app::config_path();
            options.pid = app::pid_path("network");
            // Rofi suppresses custom keys when -no-custom has no matching rows.
            options.allow_custom = true;
            let selected_row = focus
                .as_deref()
                .and_then(|id| items.iter().position(|item| item.id == id))
                .unwrap_or_else(|| {
                    items
                        .iter()
                        .position(|item| !item.nonselectable)
                        .unwrap_or(0)
                });
            let sections = items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.active)
                .map(|(index, _)| index.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let mut message = main_message(&state, &status, &choice.theme, no_theme);
            if let Some(details) = &details {
                message.push('\n');
                message.push_str(&connection_summary(&state, details, &choice.theme));
            }
            options.extra_args = vec![
                "-mesg".to_owned(),
                message,
                "-a".to_owned(),
                sections,
                "-no-hover-select".to_owned(),
                "-kb-custom-1".to_owned(),
                "Alt+r".to_owned(),
                "-kb-custom-2".to_owned(),
                "Alt+m".to_owned(),
                "-kb-custom-7".to_owned(),
                "Alt+i".to_owned(),
            ];

            input.restore(&mut options, selected_row);
            if !no_theme {
                app::bind_theme_shortcuts(&mut options, 2);
                options.extra_args.extend([
                    "-theme-str".to_owned(),
                    network_toolbar(
                        &choice.theme,
                        state.enabled && state.iface.is_some(),
                        details.is_some(),
                    ),
                ]);
            }

            let (outcome, current_input) = rofi.dmenu_with_input(&items, &options)?;
            if outcome == Outcome::Custom(1) {
                // Jump to the actions below the networks in the same list.
                input = MenuInput {
                    text: String::new(),
                    row: Some(first_action),
                };
                continue;
            }
            if outcome == Outcome::Custom(6) {
                details = if details.is_some() {
                    None
                } else {
                    Some(load_connection_details(&state))
                };
                input = current_input;
                continue;
            }
            if let Outcome::Custom(binding) = outcome
                && !no_theme
                && let Some((theme, path)) = app::session_theme("network", binding, 2)
            {
                choice = Choice { theme, path };
                input = current_input;
                continue;
            }
            // Header rows and unmatched input cannot trigger a network action.
            if matches!(outcome, Outcome::Selected(_))
                && current_input
                    .row
                    .and_then(|row| items.get(row))
                    .is_none_or(|item| item.nonselectable)
            {
                input = MenuInput {
                    row: None,
                    ..current_input
                };
                continue;
            }
            break (outcome, actions);
        };

        let selected = match outcome {
            Outcome::Cancelled => return Ok(()),
            Outcome::Custom(0) => {
                // Alt+r requests a scan from any screen.
                if let Some(iface) = &state.iface {
                    match nmcli(&["device", "wifi", "rescan", "ifname", iface]) {
                        Ok(_) => status = format!("Requested a Wi-Fi scan on {iface}."),
                        Err(error) => status = format!("Scan failed: {error}"),
                    }
                } else {
                    status = "No Wi-Fi adapter detected.".to_owned();
                }
                continue;
            }
            Outcome::Custom(_) => return Ok(()),
            Outcome::Selected(id) => id,
        };

        let action = actions
            .iter()
            .find(|(id, _)| id == &selected)
            .map(|(_, action)| action.clone())
            .with_context(|| format!("network menu returned unknown row `{selected}`"))?;

        focus = Some(selected.clone());
        if matches!(
            action,
            Action::WifiOn
                | Action::WifiOff
                | Action::DisconnectWifi
                | Action::DisconnectEthernet(_)
                | Action::ConnectEthernet(_)
                | Action::QuickConnectEthernet
                | Action::ConnectAp { .. }
                | Action::ConnectHidden
        ) {
            details = None;
        }
        match action {
            Action::Noop => {}
            Action::Rescan => {
                if let Some(iface) = &state.iface {
                    match nmcli(&["device", "wifi", "rescan", "ifname", iface]) {
                        Ok(_) => status = format!("Requested a Wi-Fi scan on {iface}."),
                        Err(error) => status = format!("Scan failed: {error}"),
                    }
                }
            }
            Action::WifiOn => match nmcli(&["radio", "wifi", "on"]) {
                Ok(_) => status = "Wi-Fi turned on.".to_owned(),
                Err(error) => status = format!("Could not turn Wi-Fi on: {error}"),
            },
            Action::WifiOff => match nmcli(&["radio", "wifi", "off"]) {
                Ok(_) => status = "Wi-Fi turned off.".to_owned(),
                Err(error) => status = format!("Could not turn Wi-Fi off: {error}"),
            },
            Action::DisconnectWifi => {
                let Some(iface) = &state.iface else {
                    status = "No Wi-Fi adapter detected.".to_owned();

                    continue;
                };
                match nmcli(&["device", "disconnect", iface]) {
                    Ok(_) => status = format!("Disconnected Wi-Fi {iface}."),
                    Err(error) => status = format!("Could not disconnect Wi-Fi: {error}"),
                }
            }
            Action::ConnectEthernet(iface) => match nmcli(&["device", "connect", &iface]) {
                Ok(_) => status = format!("Connected {iface}."),
                Err(error) => status = format!("Could not connect {iface}: {error}"),
            },
            Action::DisconnectEthernet(iface) => match nmcli(&["device", "disconnect", &iface]) {
                Ok(_) => status = format!("Disconnected {iface}."),
                Err(error) => status = format!("Could not disconnect {iface}: {error}"),
            },
            Action::QuickConnectEthernet => {
                let Some(iface) = state.ethernet.iter().find(|iface| !iface.connected) else {
                    status = "No wired interface available.".to_owned();

                    continue;
                };
                match nmcli(&["device", "connect", &iface.name]) {
                    Ok(_) => status = format!("Connected {}.", iface.name),
                    Err(error) => status = format!("Could not connect {}: {error}", iface.name),
                }
            }
            Action::ConnectHidden => {
                let Some(iface) = &state.iface else {
                    status = "No Wi-Fi adapter detected.".to_owned();

                    continue;
                };
                let Some(ssid) = prompt(&rofi, &choice, no_theme, "Hidden network SSID", false)?
                else {
                    status.clear();

                    continue;
                };
                let ssid = ssid.trim();
                if ssid.is_empty() {
                    status = "Hidden network name cannot be empty.".to_owned();

                    continue;
                }
                let Some(password) =
                    prompt(&rofi, &choice, no_theme, "Password (blank if open)", true)?
                else {
                    status.clear();

                    continue;
                };
                if password.is_empty()
                    && !confirm_open(&rofi, &choice, no_theme, "Connect without a password?")?
                {
                    status.clear();

                    continue;
                }
                connect_ap(iface, ssid, &password, true, &mut status)?;
            }
            Action::ConnectAp { ssid, security } => {
                let Some(iface) = &state.iface else {
                    status = "No Wi-Fi adapter detected.".to_owned();

                    continue;
                };
                let open = is_open(&security);
                let password = if open {
                    String::new()
                } else {
                    let Some(password) = prompt(
                        &rofi,
                        &choice,
                        no_theme,
                        &format!("Password for {ssid}"),
                        true,
                    )?
                    else {
                        status.clear();

                        continue;
                    };
                    if password.is_empty() {
                        status = "Password entry cancelled.".to_owned();

                        continue;
                    }
                    password
                };
                connect_ap(iface, &ssid, &password, false, &mut status)?;
            }
            Action::EditSaved => {
                if saved.take().is_none() {
                    let profiles = saved_connections();
                    if let Some(first) = profiles.first() {
                        focus = Some(format!("saved:{}", first.name));
                    }
                    saved = Some(profiles);
                }
            }
            Action::EditSavedProfile(name) => {
                edit_connection(&name, &mut status)?;
            }
        }
    }
}

/// `nmcli device wifi connect` for a visible or hidden SSID.
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
    if !password.is_empty() {
        args.extend(["password", password]);
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

fn is_open(security: &str) -> bool {
    security.is_empty()
        || security == "--"
        || security.eq_ignore_ascii_case("open")
        || security.eq_ignore_ascii_case("none")
}

fn connection_line(state: &WifiState, theme: &Theme) -> String {
    let ethernet = state.ethernet.iter().find(|iface| iface.connected);
    let suffix = ethernet
        .map(|iface| {
            format!(
                "   <span foreground=\"{}\">\u{00b7}</span> <span foreground=\"{}\">Ethernet {}</span>",
                theme.muted,
                theme.muted,
                escape_markup(&iface.name)
            )
        })
        .unwrap_or_default();

    if !state.enabled {
        return format!(
            "<span foreground=\"{}\" weight=\"bold\">Wi-Fi off</span>{suffix}",
            theme.accent
        );
    }
    match &state.active_ssid {
        Some(ssid) => format!(
            "<span foreground=\"{}\" weight=\"bold\">Connected</span> \
             <span foreground=\"{}\">\u{00b7}</span> \
             <span foreground=\"{}\">{}</span>{suffix}",
            theme.work,
            theme.muted,
            theme.text,
            escape_markup(ssid)
        ),
        None if state.iface.is_some() => format!(
            "<span foreground=\"{}\" weight=\"bold\">Wi-Fi disconnected</span>{suffix}",
            theme.muted
        ),
        None => format!(
            "<span foreground=\"{}\" weight=\"bold\">No Wi-Fi adapter</span>{suffix}",
            theme.danger
        ),
    }
}

fn status_line(state: &WifiState, message: &str, theme: &Theme) -> String {
    let mut lines = vec![connection_line(state, theme)];
    if !message.is_empty() {
        let level = if message.starts_with("Could not")
            || message.starts_with("Scan failed")
            || message.starts_with("No ")
        {
            Level::Danger
        } else {
            Level::Info
        };
        lines.push(ui::status(
            &ui::truncate(message, DETAIL_WIDTH),
            level,
            theme,
        ));
    }
    lines.join("\n")
}

fn network_toolbar(theme: &Theme, can_scan: bool, show_details: bool) -> String {
    let buttons = if can_scan {
        "button-scan, button-details, button-actions"
    } else {
        "button-details, button-actions"
    };
    let details_label = if show_details {
        "Hide details"
    } else {
        "Details"
    };
    let lines = if show_details { 5 } else { 13 };
    // Leave room below the button borders at fractional display scaling.
    format!(
        r#"
window {{ width: calc( ( 74ch + 64px ) min 94% ); padding: 18px; }}
mainbox {{
    children: [ message, box-network-actions, inputbar, listview ];
    spacing: 16px;
}}
message {{ padding: 10px 16px; }}
inputbar {{ padding: 8px 16px; }}
listview {{ lines: {lines}; scrollbar: true; spacing: 2px; }}
element {{ padding: 4px 14px; }}
element normal.active, element alternate.active, element selected.active {{
    background-color: {mantle};
    text-color: {text};
    border: 1px;
    border-color: transparent;
    border-radius: 6px;
}}
box-network-actions {{
    background-color: transparent;
    orientation: horizontal;
    expand: false;
    spacing: 14px;
    padding: 0 0 2px;
    children: [ {buttons} ];
}}
button-scan, button-details, button-actions {{
    expand: false;
    padding: 7px 12px;
    margin: 0 0 2px;
    border: 1px;
    border-radius: 6px;
    border-color: {muted};
    background-color: {mantle};
    text-color: {text};
}}
button-scan {{
    content: "Scan Alt+r";
    action: "kb-custom-1";
}}
button-details {{
    content: "{details_label} Alt+i";
    action: "kb-custom-7";
}}
button-actions {{
    content: "Actions Alt+m";
    action: "kb-custom-2";
}}
"#,
        muted = theme.muted,
        mantle = theme.mantle,
        text = theme.text,
    )
}

fn main_message(state: &WifiState, status: &str, theme: &Theme, no_theme: bool) -> String {
    let mut keys = Vec::new();
    if !state.enabled {
        keys.push(("Enter", "Turn Wi-Fi on"));
    } else if state.iface.is_some() {
        keys.push(("Enter", "Select"));
        if no_theme {
            keys.push(("Alt+r", "Scan"));
        }
    }
    keys.push(if no_theme {
        ("Alt+m", "Actions")
    } else {
        ("Ctrl+1–4", "Theme")
    });
    if no_theme {
        keys.push(("Alt+i", "Details"));
    }
    let hints = ui::hint(&keys, theme);
    format!("{}\n{hints}", status_line(state, status, theme))
}

fn connection_summary(state: &WifiState, details: &ConnectionDetails, theme: &Theme) -> String {
    let addresses = joined_values(
        [details.address.as_deref(), details.ipv6.as_deref()]
            .into_iter()
            .flatten(),
    );
    let gateways = joined_values(
        [details.gateway.as_deref(), details.ipv6_gateway.as_deref()]
            .into_iter()
            .flatten(),
    );
    let dns = joined_values(details.dns.iter().map(String::as_str));
    let mut lines = detail_lines("IP/prefix", &addresses);
    lines.extend(detail_lines("Gateway", &gateways));
    lines.extend(detail_lines("VPN", &details.vpn));
    lines.extend(detail_lines("DNS", &dns));
    if !details.iface.is_empty() {
        let mut link = vec![("Interface", details.iface.as_str())];
        if state.iface.as_deref() == Some(details.iface.as_str())
            && let Some(active) = &state.active
        {
            let band = active.band();
            if !band.is_empty() {
                link.push(("Band", band));
            }
            if !active.security.is_empty() && active.security != "--" {
                link.push(("Security", &active.security));
            }
        }
        lines.push(ui::inline_detail(&link, theme));
        lines.push(ui::inline_detail(
            &[
                ("Subnet", details.subnet.as_deref().unwrap_or("--")),
                ("Broadcast", details.broadcast.as_deref().unwrap_or("--")),
            ],
            theme,
        ));
        lines.push(ui::inline_detail(
            &[("MAC", details.mac.as_deref().unwrap_or("--"))],
            theme,
        ));
    }
    lines.join("\n")
}

fn joined_values<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let text = values.collect::<Vec<_>>().join(", ");
    if text.is_empty() {
        "--".to_owned()
    } else {
        text
    }
}

/// A label/value pair, with the value wrapped under the value column when a
/// long address or DNS list would otherwise be clipped.
fn detail_lines(label: &str, value: &str) -> Vec<String> {
    let value_width = DETAIL_WIDTH.saturating_sub(14).max(16);
    let wrapped = wrap_value(value, value_width);
    let mut lines = Vec::with_capacity(wrapped.len());
    for (index, part) in wrapped.iter().enumerate() {
        let left = if index == 0 {
            format!("<b>{label:>10}</b>  ")
        } else {
            "            ".to_owned()
        };
        lines.push(format!("{left}{}", escape_markup(part)));
    }
    lines
}

fn wrap_value(value: &str, width: usize) -> Vec<String> {
    if ui::display_width(value) <= width {
        return vec![value.to_owned()];
    }
    let mut parts = Vec::new();
    let mut rest = value;
    while !rest.is_empty() {
        let mut cut = 0;
        let mut width_seen = 0;
        for (index, ch) in rest.char_indices() {
            let char_width = ui::display_width(&ch.to_string());
            if width_seen + char_width > width {
                break;
            }
            width_seen += char_width;
            cut = index + ch.len_utf8();
        }
        if cut == 0 {
            cut = rest.len();
        }
        let (head, tail) = rest.split_at(cut);
        parts.push(head.to_owned());
        rest = tail;
    }
    parts
}

fn build_main(state: &WifiState, theme: &Theme) -> (Vec<Item>, Vec<(String, Action)>) {
    let mut items = Vec::new();
    let mut actions = Vec::new();

    if !state.enabled {
        return (items, actions);
    }

    if state.iface.is_none() {
        if state.ethernet.is_empty() {
            push_row(
                &mut items,
                &mut actions,
                "static:no-adapter".to_owned(),
                Item::new("", ui::empty("No Wi-Fi adapter", theme)).nonselectable(),
                Action::Noop,
            );
        }
        return (items, actions);
    }

    let mut networks = state.access_points.clone();
    networks.retain(|ap| !ap.active);
    networks.sort_by(|left, right| {
        right
            .signal
            .cmp(&left.signal)
            .then_with(|| left.ssid.to_lowercase().cmp(&right.ssid.to_lowercase()))
    });

    if networks.is_empty() {
        let label = if state.active_ssid.is_some() {
            "No other networks found"
        } else {
            "No networks found"
        };
        push_row(
            &mut items,
            &mut actions,
            "static:empty".to_owned(),
            Item::new("", ui::empty(label, theme)).nonselectable(),
            Action::Noop,
        );
        push_row(
            &mut items,
            &mut actions,
            "action:scan".to_owned(),
            Item::new("", label_row("Scan for networks", "", false, theme)),
            Action::Rescan,
        );
        return (items, actions);
    }

    items.push(section_row("wifi", "Nearby Wi-Fi", theme));
    for ap in networks {
        let open = is_open(&ap.security);
        let security = if open { "Open" } else { "Secured" };
        let display = network_row(&ap.ssid, ap.signal, security, theme);
        push_row(
            &mut items,
            &mut actions,
            format!("ap:{}", ap.ssid),
            Item::new("", display),
            Action::ConnectAp {
                ssid: ap.ssid,
                security: ap.security,
            },
        );
    }
    (items, actions)
}

fn add_actions(
    state: &WifiState,
    saved: Option<&[Saved]>,
    theme: &Theme,
    items: &mut Vec<Item>,
    actions: &mut Vec<(String, Action)>,
) {
    if state.enabled && state.iface.is_some() {
        push_row(
            items,
            actions,
            "manage:hidden".to_owned(),
            Item::new("", label_row("Connect to hidden network", "", false, theme)),
            Action::ConnectHidden,
        );
    }
    if state.enabled && state.iface.is_some() {
        push_row(
            items,
            actions,
            "manage:wifi-off".to_owned(),
            Item::new("", label_row("Turn Wi-Fi off", "", true, theme)),
            Action::WifiOff,
        );
    } else if state.iface.is_some() || !state.enabled {
        push_row(
            items,
            actions,
            "action:wifi-on".to_owned(),
            Item::new("", label_row("Turn Wi-Fi on", "", false, theme)),
            Action::WifiOn,
        );
    }
    if state.active_ssid.is_some() {
        push_row(
            items,
            actions,
            "manage:disconnect-wifi".to_owned(),
            Item::new("", label_row("Disconnect current Wi-Fi", "", true, theme)),
            Action::DisconnectWifi,
        );
    }

    if state.ethernet.iter().any(|iface| !iface.connected) {
        push_row(
            items,
            actions,
            "manage:quick-wired".to_owned(),
            Item::new("", label_row("Connect wired network", "", false, theme)),
            Action::QuickConnectEthernet,
        );
    }
    push_ethernet_rows(state, items, actions, theme);

    let label = if saved.is_some() {
        "Saved connections  ·  Hide"
    } else {
        "Saved connections  ·  Show"
    };
    push_row(
        items,
        actions,
        "manage:saved".to_owned(),
        Item::new("", label_row(label, "", false, theme)).meta("saved profiles edit connection"),
        Action::EditSaved,
    );
    if let Some(saved) = saved {
        add_saved_rows(saved, theme, items, actions);
    }
}

fn push_ethernet_rows(
    state: &WifiState,
    items: &mut Vec<Item>,
    actions: &mut Vec<(String, Action)>,
    theme: &Theme,
) {
    for iface in &state.ethernet {
        let (label_text, action, id) = if iface.connected {
            (
                format!("Disconnect Ethernet {}", iface.name),
                Action::DisconnectEthernet(iface.name.clone()),
                format!("ethernet:disconnect:{}", iface.name),
            )
        } else {
            (
                format!("Connect Ethernet {}", iface.name),
                Action::ConnectEthernet(iface.name.clone()),
                format!("ethernet:connect:{}", iface.name),
            )
        };
        push_row(
            items,
            actions,
            id,
            Item::new(
                "",
                label_row(&label_text, &iface.state, iface.connected, theme),
            ),
            action,
        );
    }
}

fn add_saved_rows(
    saved: &[Saved],
    theme: &Theme,
    items: &mut Vec<Item>,
    actions: &mut Vec<(String, Action)>,
) {
    if saved.is_empty() {
        push_row(
            items,
            actions,
            "static:no-saved".to_owned(),
            Item::new("", ui::empty("No saved connections", theme)).nonselectable(),
            Action::Noop,
        );
    } else {
        for entry in saved {
            push_row(
                items,
                actions,
                format!("saved:{}", entry.name),
                Item::new("", label_row(&entry.name, &entry.kind, false, theme)),
                Action::EditSavedProfile(entry.name.clone()),
            );
        }
    }
}

/// One compact row: a bold SSID, right-aligned signal, and fixed security
/// position. `ui::truncate` only shortens what the user sees; the row id
/// carries the full SSID so Rofi's filter matches the whole name.
fn network_row(ssid: &str, signal: u8, security: &str, theme: &Theme) -> String {
    let ssid_width = 39;
    let name = ui::truncate(ssid, ssid_width);
    let name_pad = " ".repeat(ssid_width.saturating_sub(ui::display_width(&name)));
    format!(
        "<span foreground=\"{}\">●</span>  <b>{}</b>{name_pad}  {signal:>3}%   {security:>7}",
        theme.browser,
        escape_markup(&name),
    )
}

fn section_row(id: &str, title: &str, theme: &Theme) -> Item {
    Item::new(format!("section:{id}"), ui::section(title, "", 34, theme))
        .active()
        .nonselectable()
}

fn label_row(text: &str, detail: &str, destructive: bool, theme: &Theme) -> String {
    let (marker, color) = if destructive {
        ("!", &theme.danger)
    } else {
        ("›", &theme.accent)
    };
    // Keep the label in Rofi's row foreground for contrast on selection.
    let title = format!(
        "<span foreground=\"{color}\">{marker}</span>  {}",
        escape_markup(text)
    );
    if detail.is_empty() {
        title
    } else {
        format!("{title}  {}", escape_markup(detail))
    }
}

fn push_row(
    items: &mut Vec<Item>,
    actions: &mut Vec<(String, Action)>,
    id: String,
    mut item: Item,
    action: Action,
) {
    // Rofi filters and returns the row id. Access-point ids carry the full
    // SSID, so Rofi can filter on the whole name even when the display is
    // shortened; the rendered text stays compact.
    item.id = id.clone();
    items.push(item);
    actions.push((id, action));
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
        ethernet: ethernet_ifaces(),
        ..WifiState::default()
    };
    for line in devices.lines() {
        let fields = split_fields(line, 3);
        if fields.len() == 3 && fields[1] == "wifi" {
            state.iface = Some(fields[0].clone());
            break;
        }
    }
    if let Some(iface) = &state.iface
        && state.enabled
    {
        state.access_points = access_points(iface);
        state.active = state.access_points.iter().find(|ap| ap.active).cloned();
        state.active_ssid = state.active.as_ref().map(|ap| ap.ssid.clone());
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

fn load_connection_details(state: &WifiState) -> ConnectionDetails {
    let iface = if state.active_ssid.is_some() || state.active.is_some() {
        state.iface.as_deref()
    } else {
        state
            .ethernet
            .iter()
            .find(|iface| iface.connected)
            .map(|iface| iface.name.as_str())
            .or(state.iface.as_deref())
    };
    let mut details = iface.and_then(connection_details).unwrap_or_default();
    details.vpn = vpn_status();
    details
}

fn connection_details(iface: &str) -> Option<ConnectionDetails> {
    let output = nmcli(&[
        "-t",
        "-f",
        "GENERAL.HWADDR,IP4.ADDRESS,IP4.GATEWAY,IP4.DNS,IP6.ADDRESS,IP6.GATEWAY,IP6.DNS",
        "device",
        "show",
        iface,
    ])
    .ok()?;
    let mut details = ConnectionDetails {
        iface: iface.to_owned(),
        ..ConnectionDetails::default()
    };
    for line in output.lines() {
        let fields = split_fields(line, 2);
        let [key, value] = fields.as_slice() else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() || value == "--" {
            continue;
        }
        match key.as_str() {
            "GENERAL.HWADDR" => details.mac = Some(value.to_owned()),
            "IP4.ADDRESS[1]" | "IP4.ADDRESS" => details.address = Some(value.to_owned()),
            "IP4.GATEWAY" => details.gateway = Some(value.to_owned()),
            "IP6.ADDRESS[1]" | "IP6.ADDRESS" => details.ipv6 = Some(value.to_owned()),
            "IP6.GATEWAY" => details.ipv6_gateway = Some(value.to_owned()),
            key if key.starts_with("IP4.DNS") || key.starts_with("IP6.DNS") => {
                details.dns.push(value.to_owned());
            }
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

fn vpn_status() -> String {
    let Ok(output) = nmcli(&["-t", "-f", "NAME,TYPE", "connection", "show", "--active"]) else {
        return "Unavailable".to_owned();
    };
    let names: Vec<_> = output
        .lines()
        .filter_map(|line| {
            let fields = split_fields(line, 2);
            match fields.as_slice() {
                [name, kind] if matches!(kind.as_str(), "vpn" | "wireguard") => Some(name.clone()),
                _ => None,
            }
        })
        .collect();
    if names.is_empty() {
        "None in NetworkManager".to_owned()
    } else {
        names.join(", ")
    }
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
    if let Some(config) = app::config_path() {
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

/// A one-row confirmation used when an empty password means an open network.
fn confirm_open(rofi: &Rofi, choice: &Choice, no_theme: bool, prompt: &str) -> Result<bool> {
    let items = vec![
        Item::new("confirm", "Connect without a password"),
        Item::new("cancel", "Back"),
    ];
    let mut options = DmenuOptions::new(prompt);
    if !no_theme {
        options.theme = choice.path.clone();
    }
    options.config = app::config_path();
    let runtime = paths::rofi_runtime_dir();
    let _ = std::fs::create_dir_all(&runtime);
    options.pid = Some(runtime.join("rofi-network-confirm.pid"));
    match rofi.dmenu(&items, &options)? {
        Outcome::Selected(id) => Ok(id == "confirm"),
        _ => Ok(false),
    }
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
    fn open_security_is_recognized() {
        assert!(is_open("--"));
        assert!(is_open(""));
        assert!(is_open("open"));
        assert!(!is_open("WPA2"));
    }

    #[test]
    fn wraps_long_detail_values() {
        let parts = wrap_value("192.0.2.42/24, 2001:db8::1, 2001:db8::2", 16);
        assert!(parts.len() > 1);
        for part in parts {
            assert!(ui::display_width(&part) <= 16, "{part}");
        }
    }

    #[test]
    fn network_rows_keep_full_ssid_in_the_filtered_text() {
        let row = network_row(
            "A very long network name that should be shortened",
            92,
            "Secured",
            &Theme::gruvbox(),
        );
        assert!(row.contains("A very long network name"));
        assert!(row.contains("92%"));
        assert!(row.contains("Secured"));
    }

    #[test]
    fn main_view_reserves_twelve_candidate_rows_plus_the_header() {
        let toolbar = network_toolbar(&Theme::gruvbox(), true, false);
        assert!(toolbar.contains("listview { lines: 13;"), "{toolbar}");
        assert!(!toolbar.contains("lines: 12;"), "{toolbar}");
    }
}
