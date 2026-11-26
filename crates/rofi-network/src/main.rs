//! Standalone NetworkManager menu for Rofi.
//!
//! `rofi-network` is an independent app: it resolves its own theme and config
//! and does not depend on the optional hub menu or installer.
//!
//! The default screen is for choosing a network: one connection line, a filter
//! box, the available SSIDs, and a fixed shortcut line. Manage and Connection
//! details are separate screens so the first view stays small. The app talks
//! to NetworkManager through `nmcli`.

use std::env;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use clap::Parser;
use rofi_dmenu::{
    DmenuOptions, Item, Level, Outcome, Rofi, Theme, app, escape_markup, paths, ui, which,
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
    Details,
    EditSaved,
    EditSavedProfile(String),
    WifiOn,
    WifiOff,
    Back,
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Main,
    Manage,
    Details,
    Saved,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectionDetails {
    connection: String,
    iface: String,
    address: Option<String>,
    gateway: Option<String>,
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
    Choice {
        theme: Theme::active(),
        path: app::theme_path("network"),
    }
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
    let choice = resolve_choice();
    let no_theme = cli.no_theme;

    let mut screen = Screen::Main;
    let mut status = String::new();
    let mut details: Option<ConnectionDetails> = None;

    loop {
        let state = wifi_state()?;
        let (items, actions, message, prompt_label) = match screen {
            Screen::Main => {
                let (items, actions) = build_main(&state, &choice.theme);
                (
                    items,
                    actions,
                    main_message(&state, &status, &choice.theme),
                    "Find network",
                )
            }
            Screen::Manage => {
                let (items, actions) = build_manage(&state, &choice.theme);
                (
                    items,
                    actions,
                    manage_message(&state, &status, &choice.theme),
                    "Manage network",
                )
            }
            Screen::Details => {
                let details = details.as_ref();
                (
                    details_items(&choice.theme),
                    vec![("back".to_owned(), Action::Back)],
                    details_message(&state, details, &choice.theme),
                    "Connection details",
                )
            }
            Screen::Saved => {
                let saved = saved_connections();
                let (items, actions) = saved_items(&saved, &choice.theme);
                (
                    items,
                    actions,
                    saved_message(&choice.theme),
                    "Saved connections",
                )
            }
        };

        let mut options = DmenuOptions::new(prompt_label);
        if !no_theme {
            options.theme = choice.path.clone();
        }
        options.config = app::config_path();
        let runtime = paths::rofi_runtime_dir();
        let _ = std::fs::create_dir_all(&runtime);
        options.pid = Some(runtime.join("rofi-network.pid"));
        let selected_row = items
            .iter()
            .position(|item| !item.nonselectable)
            .unwrap_or(0)
            .to_string();
        options.extra_args = vec![
            "-selected-row".to_owned(),
            selected_row,
            "-mesg".to_owned(),
            message,
            "-kb-custom-1".to_owned(),
            "Alt+r".to_owned(),
            "-kb-custom-2".to_owned(),
            "Alt+m".to_owned(),
        ];

        let selected = match rofi.dmenu(&items, &options)? {
            Outcome::Cancelled => {
                screen = match screen {
                    Screen::Main => return Ok(()),
                    Screen::Manage => Screen::Main,
                    Screen::Details => Screen::Manage,
                    Screen::Saved => Screen::Manage,
                };
                continue;
            }
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
            Outcome::Custom(1) => {
                // Alt+m opens Manage even with an empty or filtered list.
                screen = Screen::Manage;
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

        match action {
            Action::Noop => {}
            Action::Back => {
                screen = match screen {
                    Screen::Details => Screen::Manage,
                    Screen::Saved => Screen::Manage,
                    _ => Screen::Main,
                };
            }
            Action::Details => {
                let wifi_active = state.active_ssid.is_some() || state.active.is_some();
                let iface = if wifi_active {
                    state.iface.clone()
                } else {
                    state
                        .ethernet
                        .iter()
                        .find(|iface| iface.connected)
                        .map(|iface| iface.name.clone())
                        .or_else(|| state.iface.clone())
                };
                match iface {
                    Some(iface) => {
                        details = connection_details(&iface);
                        if details.is_some() {
                            screen = Screen::Details;
                            status.clear();
                        } else {
                            status = format!("No connection details for {iface}.");
                        }
                    }
                    None => status = "No active interface to inspect.".to_owned(),
                }
            }
            Action::Rescan => {
                if let Some(iface) = &state.iface {
                    match nmcli(&["device", "wifi", "rescan", "ifname", iface]) {
                        Ok(_) => status = format!("Requested a Wi-Fi scan on {iface}."),
                        Err(error) => status = format!("Scan failed: {error}"),
                    }
                }
                screen = Screen::Main;
            }
            Action::WifiOn => {
                match nmcli(&["radio", "wifi", "on"]) {
                    Ok(_) => status = "Wi-Fi turned on.".to_owned(),
                    Err(error) => status = format!("Could not turn Wi-Fi on: {error}"),
                }
                screen = Screen::Main;
            }
            Action::WifiOff => {
                match nmcli(&["radio", "wifi", "off"]) {
                    Ok(_) => status = "Wi-Fi turned off.".to_owned(),
                    Err(error) => status = format!("Could not turn Wi-Fi off: {error}"),
                }
                screen = Screen::Main;
            }
            Action::DisconnectWifi => {
                let Some(iface) = &state.iface else {
                    status = "No Wi-Fi adapter detected.".to_owned();
                    screen = Screen::Main;
                    continue;
                };
                match nmcli(&["device", "disconnect", iface]) {
                    Ok(_) => status = format!("Disconnected Wi-Fi {iface}."),
                    Err(error) => status = format!("Could not disconnect Wi-Fi: {error}"),
                }
                screen = Screen::Main;
            }
            Action::ConnectEthernet(iface) => {
                match nmcli(&["device", "connect", &iface]) {
                    Ok(_) => status = format!("Connected {iface}."),
                    Err(error) => status = format!("Could not connect {iface}: {error}"),
                }
                screen = Screen::Main;
            }
            Action::DisconnectEthernet(iface) => {
                match nmcli(&["device", "disconnect", &iface]) {
                    Ok(_) => status = format!("Disconnected {iface}."),
                    Err(error) => status = format!("Could not disconnect {iface}: {error}"),
                }
                screen = Screen::Main;
            }
            Action::QuickConnectEthernet => {
                let Some(iface) = state.ethernet.iter().find(|iface| !iface.connected) else {
                    status = "No wired interface available.".to_owned();
                    screen = Screen::Manage;
                    continue;
                };
                match nmcli(&["device", "connect", &iface.name]) {
                    Ok(_) => status = format!("Connected {}.", iface.name),
                    Err(error) => status = format!("Could not connect {}: {error}", iface.name),
                }
                screen = Screen::Main;
            }
            Action::ConnectHidden => {
                let Some(iface) = &state.iface else {
                    status = "No Wi-Fi adapter detected.".to_owned();
                    screen = Screen::Manage;
                    continue;
                };
                let Some(ssid) = prompt(&rofi, &choice, no_theme, "Hidden network SSID", false)?
                else {
                    status.clear();
                    screen = Screen::Manage;
                    continue;
                };
                let ssid = ssid.trim();
                if ssid.is_empty() {
                    status = "Hidden network name cannot be empty.".to_owned();
                    screen = Screen::Manage;
                    continue;
                }
                let Some(password) =
                    prompt(&rofi, &choice, no_theme, "Password (blank if open)", true)?
                else {
                    status.clear();
                    screen = Screen::Manage;
                    continue;
                };
                if password.is_empty()
                    && !confirm_open(&rofi, &choice, no_theme, "Connect without a password?")?
                {
                    status.clear();
                    screen = Screen::Manage;
                    continue;
                }
                connect_ap(iface, ssid, &password, true, &mut status)?;
                screen = Screen::Main;
            }
            Action::ConnectAp { ssid, security } => {
                let Some(iface) = &state.iface else {
                    status = "No Wi-Fi adapter detected.".to_owned();
                    screen = Screen::Main;
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
                        screen = Screen::Main;
                        continue;
                    };
                    if password.is_empty() {
                        status = "Password entry cancelled.".to_owned();
                        screen = Screen::Main;
                        continue;
                    }
                    password
                };
                connect_ap(iface, &ssid, &password, false, &mut status)?;
                screen = Screen::Main;
            }
            Action::EditSaved => {
                screen = Screen::Saved;
                status.clear();
            }
            Action::EditSavedProfile(name) => {
                edit_connection(&name, &mut status)?;
                screen = Screen::Manage;
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

fn main_message(state: &WifiState, status: &str, theme: &Theme) -> String {
    let hints = if !state.enabled {
        ui::hint(&[("Enter", "Turn Wi-Fi on"), ("Alt+m", "Manage")], theme)
    } else if state.iface.is_none() {
        ui::hint(&[("Alt+m", "Manage")], theme)
    } else {
        ui::hint(
            &[("Enter", "Connect"), ("Alt+r", "Scan"), ("Alt+m", "Manage")],
            theme,
        )
    };
    format!("{}\n{hints}", status_line(state, status, theme))
}

fn manage_message(state: &WifiState, status: &str, theme: &Theme) -> String {
    let hints = ui::hint(&[("Enter", "Select"), ("Esc", "Back")], theme);
    format!("{}\n{hints}", status_line(state, status, theme))
}

fn saved_message(theme: &Theme) -> String {
    ui::hint(&[("Enter", "Edit"), ("Esc", "Back")], theme)
}

fn details_message(
    state: &WifiState,
    details: Option<&ConnectionDetails>,
    theme: &Theme,
) -> String {
    let Some(details) = details else {
        return format!(
            "No connection details\n{}",
            ui::hint(&[("Esc", "Back")], theme)
        );
    };
    let title = escape_markup(&details.connection);
    let mut lines = vec![format!(
        "<span foreground=\"{}\" weight=\"bold\" size=\"14000\">{title}</span>",
        theme.text
    )];

    let mut qualified = vec![("Interface", details.iface.clone())];
    if state.iface.as_deref() == Some(details.iface.as_str())
        && let Some(active) = &state.active
    {
        let band = active.band();
        if !band.is_empty() {
            qualified.push(("Band", band.to_owned()));
        }
        if !active.security.is_empty() && active.security != "--" {
            qualified.push(("Security", active.security.clone()));
        }
    }
    for (label, value) in qualified {
        lines.extend(detail_lines(label, &value, theme));
    }
    lines.extend(detail_lines(
        "IP/prefix",
        details.address.as_deref().unwrap_or("--"),
        theme,
    ));
    lines.extend(detail_lines(
        "Gateway",
        details.gateway.as_deref().unwrap_or("--"),
        theme,
    ));
    lines.extend(detail_lines(
        "Subnet",
        details.subnet.as_deref().unwrap_or("--"),
        theme,
    ));
    lines.extend(detail_lines(
        "Broadcast",
        details.broadcast.as_deref().unwrap_or("--"),
        theme,
    ));
    let dns = if details.dns.is_empty() {
        "--".to_owned()
    } else {
        details.dns.join(", ")
    };
    lines.extend(detail_lines("DNS", &dns, theme));
    lines.extend(detail_lines(
        "MAC",
        details.mac.as_deref().unwrap_or("--"),
        theme,
    ));
    lines.push(ui::hint(&[("Esc", "Back")], theme));
    lines.join("\n")
}

/// A label/value pair, with the value wrapped under the value column when a
/// long address or DNS list would otherwise be clipped.
fn detail_lines(label: &str, value: &str, theme: &Theme) -> Vec<String> {
    let value_width = DETAIL_WIDTH.saturating_sub(14).max(16);
    let wrapped = wrap_value(value, value_width);
    let mut lines = Vec::with_capacity(wrapped.len());
    for (index, part) in wrapped.iter().enumerate() {
        let left = if index == 0 {
            format!("<span foreground=\"{}\">{label:>10}</span>  ", theme.muted)
        } else {
            "            ".to_owned()
        };
        lines.push(format!(
            "{left}<span foreground=\"{}\">{}</span>",
            theme.text,
            escape_markup(part)
        ));
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
        push_row(
            &mut items,
            &mut actions,
            "action:wifi-on".to_owned(),
            Item::new("", label("Turn Wi-Fi on", theme)),
            Action::WifiOn,
        );
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
        } else {
            push_ethernet_rows(state, &mut items, &mut actions, theme);
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
            Item::new("", label_row("Scan for networks", theme)),
            Action::Rescan,
        );
        return (items, actions);
    }

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

fn build_manage(state: &WifiState, theme: &Theme) -> (Vec<Item>, Vec<(String, Action)>) {
    let mut items = Vec::new();
    let mut actions = Vec::new();

    if state.active_ssid.is_some()
        || state.active.is_some()
        || state.ethernet.iter().any(|iface| iface.connected)
    {
        push_row(
            &mut items,
            &mut actions,
            "manage:details".to_owned(),
            Item::new("", label_row("Connection details", theme)),
            Action::Details,
        );
    }
    if state.enabled {
        push_row(
            &mut items,
            &mut actions,
            "manage:wifi-off".to_owned(),
            Item::new("", label_row("Turn Wi-Fi off", theme)),
            Action::WifiOff,
        );
    } else {
        push_row(
            &mut items,
            &mut actions,
            "manage:wifi-on".to_owned(),
            Item::new("", label_row("Turn Wi-Fi on", theme)),
            Action::WifiOn,
        );
    }
    if state.iface.is_some() {
        push_row(
            &mut items,
            &mut actions,
            "manage:hidden".to_owned(),
            Item::new("", label_row("Connect to hidden network", theme)),
            Action::ConnectHidden,
        );
    }
    if !saved_connections().is_empty() {
        push_row(
            &mut items,
            &mut actions,
            "manage:saved".to_owned(),
            Item::new("", label_row("Edit saved connection", theme)),
            Action::EditSaved,
        );
    }
    if state.active_ssid.is_some() {
        push_row(
            &mut items,
            &mut actions,
            "manage:disconnect-wifi".to_owned(),
            Item::new("", label_row("Disconnect current Wi-Fi", theme)),
            Action::DisconnectWifi,
        );
    }

    if state.ethernet.iter().any(|iface| !iface.connected) {
        push_row(
            &mut items,
            &mut actions,
            "manage:quick-wired".to_owned(),
            Item::new("", label_row("Connect wired network", theme)),
            Action::QuickConnectEthernet,
        );
    }
    push_ethernet_rows(state, &mut items, &mut actions, theme);

    push_row(
        &mut items,
        &mut actions,
        "back".to_owned(),
        Item::new("", label_row("Back", theme)),
        Action::Back,
    );
    (items, actions)
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
        let detail = if iface.state.is_empty() {
            ""
        } else {
            &iface.state
        };
        push_row(
            items,
            actions,
            id,
            Item::new("", ui::row("", &label_text, detail, "", theme)),
            action,
        );
    }
}

fn saved_items(saved: &[Saved], theme: &Theme) -> (Vec<Item>, Vec<(String, Action)>) {
    let mut items = Vec::new();
    let mut actions = Vec::new();
    if saved.is_empty() {
        push_row(
            &mut items,
            &mut actions,
            "static:no-saved".to_owned(),
            Item::new("", ui::empty("No saved connections", theme)).nonselectable(),
            Action::Noop,
        );
    } else {
        for entry in saved {
            push_row(
                &mut items,
                &mut actions,
                format!("saved:{}", entry.name),
                Item::new("", ui::row("", &entry.name, &entry.kind, "", theme)),
                Action::EditSavedProfile(entry.name.clone()),
            );
        }
    }
    push_row(
        &mut items,
        &mut actions,
        "back".to_owned(),
        Item::new("", label_row("Back", theme)),
        Action::Back,
    );
    (items, actions)
}

/// One compact row: a bold SSID, right-aligned signal, and fixed security
/// position. `ui::truncate` only shortens what the user sees; the row id
/// carries the full SSID so Rofi's filter matches the whole name.
fn network_row(ssid: &str, signal: u8, security: &str, theme: &Theme) -> String {
    let ssid_width = 33;
    let name = ui::truncate(ssid, ssid_width);
    let name_pad = " ".repeat(ssid_width.saturating_sub(ui::display_width(&name)));
    let signal = format!("{signal:>3}%");
    format!(
        "<span foreground=\"{}\" weight=\"600\">{}</span>{name_pad}  \
         <span foreground=\"{}\">{signal}</span>   \
         <span foreground=\"{}\">{security:>7}</span>",
        theme.text,
        escape_markup(&name),
        theme.muted,
        theme.muted,
    )
}

fn label(text: &str, theme: &Theme) -> String {
    format!(
        "<span foreground=\"{}\" weight=\"600\">{}</span>",
        theme.text,
        escape_markup(text)
    )
}

fn label_row(text: &str, theme: &Theme) -> String {
    ui::row("", text, "", "", theme)
}

fn details_items(theme: &Theme) -> Vec<Item> {
    vec![Item::new("back", label_row("Back", theme))]
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
        iface: iface.to_owned(),
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
        let theme = Theme::gruvbox();
        let row = network_row(
            "A very long network name that should be shortened",
            92,
            "Secured",
            &theme,
        );
        assert!(row.contains("A very long network name"));
        assert!(row.contains("92%"));
        assert!(row.contains("Secured"));
    }
}
