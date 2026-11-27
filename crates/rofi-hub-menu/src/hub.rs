use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use rofi_dmenu::manifest::ManifestSet;
use rofi_dmenu::{
    Category, DmenuOptions, Item, MenuInput, Mode, Outcome, ProviderManifest, Rofi, Theme, ui,
};
use rofi_dmenu::{notify, paths, which};
use serde_json::json;

use crate::cli::{Cli, Command as CliCommand, ThemeCommand};
use crate::config::Config;
use crate::theme::{self, Choice};
use rofi_dmenu::ChromeProfile;

/// Mirrors screen-time: Ctrl+1..3 pick a palette, Ctrl+4 restores auto.
const THEME_BINDINGS: [(Mode, &str); 4] = [
    (Mode::Nord, "Control+1"),
    (Mode::Solarized, "Control+2"),
    (Mode::Gruvbox, "Control+3"),
    (Mode::Auto, "Control+4"),
];

/// The Rofi emoji plugin is denser than a text list: four columns with at most
/// fifteen visible rows keeps the picker compact on a 1280×800 desktop.
const EMOJI_COLUMNS: u32 = 4;
const EMOJI_LINES: u32 = 15;

/// Power actions are short labels, so three columns keep the whole menu on one
/// compact screen.
const POWER_COLUMNS: u32 = 3;
const POWER_LINES: u32 = 2;

pub fn run(cli: Cli) -> Result<()> {
    let (config, warnings) = Config::load();
    for warning in &warnings {
        eprintln!("rofi-hub-menu: warning: {warning}");
    }

    let Cli {
        provider,
        theme,
        no_theme,
        command,
    } = cli;
    let theme = theme.as_deref();

    match command {
        Some(CliCommand::Open {
            provider: subcommand_provider,
        }) => open(theme, no_theme, &config, subcommand_provider.or(provider)),
        Some(CliCommand::List { json }) => list(json),
        Some(CliCommand::Theme { action }) => theme_command(theme, &config, action),
        Some(CliCommand::Doctor { json }) => doctor(theme, &config, json, &warnings),
        Some(CliCommand::Launcher) => show_mode(
            theme,
            no_theme,
            &config,
            "drun",
            &["-p".to_owned(), "Apps".to_owned()],
            config.launcher_columns(),
            config.launcher_lines(),
        ),
        Some(CliCommand::Window) => show_mode(theme, no_theme, &config, "window", &[], None, None),
        Some(CliCommand::Calc) => show_calc(theme, no_theme, &config),
        Some(CliCommand::Emoji) => show_mode(
            theme,
            no_theme,
            &config,
            "emoji",
            &["-modi".to_owned(), "emoji".to_owned()],
            Some(EMOJI_COLUMNS),
            Some(EMOJI_LINES),
        ),
        Some(CliCommand::Power) => show_power(theme, no_theme, &config),
        Some(CliCommand::Show { mode, args }) => {
            show_mode(theme, no_theme, &config, &mode, &args, None, None)
        }
        None => open(theme, no_theme, &config, provider),
    }
}

fn show_mode(
    theme_override: Option<&str>,
    no_theme: bool,
    config: &Config,
    mode: &str,
    extra: &[String],
    columns: Option<u32>,
    lines: Option<u32>,
) -> Result<()> {
    let choice = theme::resolve(theme_override, config)?;
    let rofi = Rofi::find()?;
    let mut command = Command::new(rofi.binary());
    command.arg("-show").arg(mode);
    if let Some(path) = rofi_config() {
        command.arg("-config").arg(path);
    }
    if !no_theme && let Some(path) = &choice.path {
        command.arg("-theme").arg(path);
    }
    if columns.is_some() || lines.is_some() {
        let mut rules = String::from("listview {");
        if let Some(columns) = columns {
            rules.push_str(&format!(" columns: {columns};"));
        }
        if let Some(lines) = lines {
            rules.push_str(&format!(" lines: {lines};"));
        }
        rules.push_str(" }");
        command.arg("-theme-str").arg(rules);
    }
    command.args(extra);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    let status = command
        .status()
        .with_context(|| format!("cannot start Rofi mode `{mode}`"))?;
    match status.code() {
        // Rofi uses 1 for "cancelled" in show mode.
        Some(0 | 1) | None => Ok(()),
        Some(code) => bail!("Rofi mode `{mode}` exited with status {code}"),
    }
}

fn show_calc(theme: Option<&str>, no_theme: bool, config: &Config) -> Result<()> {
    let mut extra = vec![
        "-modi".to_owned(),
        "calc".to_owned(),
        "-no-show-match".to_owned(),
        "-no-sort".to_owned(),
    ];
    if which::find("wl-copy").is_some() {
        extra.push("-calc-command".to_owned());
        extra.push("echo -n '{result}' | wl-copy".to_owned());
    }
    show_mode(theme, no_theme, config, "calc", &extra, None, None)
}

fn show_power(theme: Option<&str>, no_theme: bool, config: &Config) -> Result<()> {
    let Some(menu) = which::find("rofi-power-menu") else {
        bail!("rofi-power-menu is required for `rofi-hub power`");
    };
    show_mode(
        theme,
        no_theme,
        config,
        "power-menu",
        &["-modi".to_owned(), format!("power-menu:{}", menu.display())],
        Some(POWER_COLUMNS),
        Some(POWER_LINES),
    )
}

fn open(
    theme_override: Option<&str>,
    no_theme: bool,
    config: &Config,
    provider: Option<String>,
) -> Result<()> {
    let mut choice = theme::resolve(theme_override, config)?;

    let manifests = ManifestSet::discover();
    for warning in manifests.warnings() {
        eprintln!("rofi-hub-menu: warning: {warning}");
    }

    let explicit_provider = provider.is_some();
    if let Some(id) = provider {
        let provider = manifests.find(&id).ok_or_else(|| {
            let available = manifests
                .providers()
                .iter()
                .map(|provider| provider.id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::anyhow!(
                "unknown provider `{id}`; available: {}",
                if available.is_empty() {
                    "none".to_owned()
                } else {
                    available
                }
            )
        })?;
        return launch(provider, &choice);
    }

    if manifests.is_empty() {
        notify::notify_error("Rofi Hub", "No provider manifests were found");
        bail!(
            "no provider manifests found in {}",
            paths::provider_dirs()
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(":")
        );
    }

    let rofi = match Rofi::find() {
        Ok(rofi) => rofi,
        Err(error) => {
            if !explicit_provider {
                notify::notify_error("Rofi Hub", "Rofi is required but was not found in PATH");
            }
            return Err(error.into());
        }
    };
    let providers = manifests.providers();
    let mut selected_id: Option<String> = None;
    let mut filter = String::new();

    loop {
        let mut items = hub_items(providers, &choice.theme);
        items.push(
            Item::new(
                "hint",
                ui::hint(
                    &[
                        ("\u{2191}/\u{2193}", "select"),
                        ("Enter", "open"),
                        ("Ctrl+1\u{2013}4", "theme"),
                        ("esc", "close"),
                    ],
                    &choice.theme,
                ),
            )
            .nonselectable(),
        );
        let mut options = DmenuOptions::new(config.prompt());
        if !no_theme {
            options.theme = choice.path.clone();
        }
        options.config = rofi_config();
        let selected_row = selected_id
            .as_deref()
            .and_then(|id| items.iter().position(|item| item.id == id))
            .unwrap_or_else(|| {
                items
                    .iter()
                    .position(|item| !item.nonselectable)
                    .unwrap_or(0)
            });
        let active = ui::active_rows(&items);
        options.extra_args = vec![
            "-mesg".to_owned(),
            format!(
                "{}\n{}",
                ui::header(
                    "ROFI HUB",
                    &format!("{} apps", providers.len()),
                    &choice.label(),
                    &choice.theme
                ),
                String::new()
            ),
        ];
        if !active.is_empty() {
            options.extra_args.extend(["-a".to_owned(), active]);
        }
        options
            .theme_str
            .push(ui::static_header_theme(&choice.theme));
        // Keep the popup height when a filter reduces the list. Menu theme
        // changes reopen the same list and must not resize the window.
        options
            .theme_str
            .push("listview { dynamic: false; }".to_owned());
        let runtime = paths::rofi_runtime_dir();
        let _ = std::fs::create_dir_all(&runtime);
        options.pid = Some(runtime.join("rofi.pid"));
        for (index, (_, key)) in THEME_BINDINGS.iter().enumerate() {
            options.extra_args.push(format!("-kb-custom-{}", index + 1));
            options.extra_args.push((*key).to_owned());
        }
        let input = MenuInput {
            text: filter.clone(),
            row: Some(selected_row),
        };
        input.restore(&mut options, selected_row);

        let (outcome, current_input) = rofi.dmenu_with_input(&items, &options)?;
        match outcome {
            Outcome::Selected(id) if id == "hint" => return Ok(()),
            Outcome::Selected(id) if id.starts_with("section-") => {
                // Headers are static bands, never actions. Keep the typed
                // filter and the selected provider while reopening.
                retain_input(&items, &current_input, &mut selected_id, &mut filter);
            }
            Outcome::Selected(id) => {
                let provider = providers
                    .iter()
                    .find(|provider| provider.id == id)
                    .with_context(|| format!("rofi selected unknown provider `{id}`"))?;
                return launch(provider, &choice);
            }
            Outcome::Cancelled => return Ok(()),
            Outcome::Custom(index) if (index as usize) < THEME_BINDINGS.len() => {
                let mode = THEME_BINDINGS[index as usize].0;
                theme::save_mode(mode)?;
                choice = if mode == Mode::Auto {
                    // Auto means "no menu override": let the configured
                    // [theme] value resolve again instead of forcing the
                    // built-in auto palette for this session.
                    theme::resolve(None, config)?
                } else {
                    theme::resolve(Some(mode.as_str()), config)?
                };
                retain_input(&items, &current_input, &mut selected_id, &mut filter);
            }
            Outcome::Custom(_) => return Ok(()),
        }
    }
}

/// Keep the provider id instead of the visible row index. The next launch
/// resolves that id against the rebuilt list, so separating headers, counts,
/// or categories cannot move the cursor to another provider.
fn retain_input(
    items: &[Item],
    input: &MenuInput,
    selected_id: &mut Option<String>,
    filter: &mut String,
) {
    filter.clone_from(&input.text);
    *selected_id = input
        .row
        .and_then(|row| items.get(row))
        .filter(|item| !item.nonselectable && item.id != "hint")
        .map(|item| item.id.clone());
}

fn hub_items(providers: &[ProviderManifest], theme: &Theme) -> Vec<Item> {
    let mut groups: Vec<(String, Vec<&ProviderManifest>)> = Vec::new();
    for provider in providers {
        let category = provider
            .category
            .clone()
            .unwrap_or_else(|| "Other".to_owned());
        if let Some(group) = groups.iter_mut().find(|(name, _)| *name == category) {
            group.1.push(provider);
        } else {
            groups.push((category, vec![provider]));
        }
    }

    groups.sort_by_key(
        |(category, _)| match category.to_ascii_lowercase().as_str() {
            "productivity" => 0,
            "utilities" => 1,
            "system" => 2,
            _ => 3,
        },
    );
    let mut items = Vec::with_capacity(providers.len() + groups.len());
    for (category, providers) in groups {
        let label = category.to_ascii_uppercase();
        items.push(
            Item::new(
                format!("section-{}", label.to_ascii_lowercase()),
                ui::section(&label, &providers.len().to_string(), 84, theme),
            )
            .active()
            .nonselectable(),
        );
        for provider in providers {
            items.push(provider_item(provider, theme));
        }
    }
    items
}

fn provider_item(provider: &ProviderManifest, theme: &Theme) -> Item {
    let available = exec_available(&provider.exec);
    let mut description = provider.description.clone().unwrap_or_default();
    if !available {
        description = format!("not found: {}", provider.exec[0]);
    }
    let category = provider
        .category
        .as_deref()
        .map(Category::from_label)
        .unwrap_or(Category::Other);
    let mut item = Item::new(
        &provider.id,
        ui::row_with_leading(
            "\u{25cf}",
            category.color(theme),
            &provider.name,
            &description,
            provider.category.as_deref().unwrap_or(category.label()),
            theme,
        ),
    );
    if let Some(category) = &provider.category {
        item = item.meta(category.clone());
    }
    if !available {
        item = item.urgent();
    }
    item
}

/// Rofi's `-config` override, when the user set a readable `ROFI_CONFIG`.
pub fn rofi_config() -> Option<PathBuf> {
    let raw = env::var_os("ROFI_CONFIG")?;
    let value = raw.to_string_lossy();
    let path = paths::expand_tilde(value.trim());
    path.is_file().then_some(path)
}

/// Resolve a provider command. Prefer `PATH`, then the directory containing
/// the hub binary. The second step keeps a user install working even when
/// `~/.local/bin` is missing from a graphical session's `PATH`.
fn resolve_program(program: &str) -> Option<PathBuf> {
    if program.contains('/') {
        let path = PathBuf::from(program);
        return which::is_executable(&path).then_some(path);
    }
    if let Some(path) = which::find(program) {
        return Some(path);
    }
    let own = env::current_exe().ok()?;
    let sibling = own.parent()?.join(program);
    which::is_executable(&sibling).then_some(sibling)
}

pub fn exec_available(exec: &[String]) -> bool {
    exec.first()
        .and_then(|program| resolve_program(program))
        .is_some()
}

fn launch(provider: &ProviderManifest, _choice: &Choice) -> Result<()> {
    let Some(program) = provider
        .exec
        .first()
        .and_then(|program| resolve_program(program))
    else {
        let message = format!("{} is not installed or not in PATH", provider.exec[0]);
        notify::notify_error("Rofi Hub Menu", &message);
        bail!("{}: {message}", provider.name);
    };

    // Launch with the user's clean environment. The menu does not inject a
    // theme, provider, or PATH: each app resolves its own theme and config.
    let mut command = Command::new(program);
    command
        .args(&provider.exec[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    match command.spawn() {
        Ok(_) => Ok(()),
        Err(source) => {
            let message = format!("cannot start {}: {source}", provider.exec[0]);
            notify::notify_error("Rofi Hub Menu", &message);
            Err(rofi_dmenu::Error::Spawn {
                command: provider.exec[0].clone(),
                source,
            }
            .into())
        }
    }
}

fn list(json_output: bool) -> Result<()> {
    let manifests = ManifestSet::discover();
    for warning in manifests.warnings() {
        eprintln!("rofi-hub-menu: warning: {warning}");
    }

    if json_output {
        let providers: Vec<_> = manifests
            .providers()
            .iter()
            .map(|provider| {
                json!({
                    "id": provider.id,
                    "name": provider.name,
                    "description": provider.description,
                    "category": provider.category(),
                    "icon": provider.icon,
                    "exec": provider.exec,
                    "version": provider.version,
                    "available": exec_available(&provider.exec),
                    "manifest": provider.path.as_ref().map(|path| path.display().to_string()),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&providers)?);
        return Ok(());
    }

    println!("{:<16} {:<24} {:<32} STATUS", "ID", "NAME", "COMMAND");
    for provider in manifests.providers() {
        let status = if exec_available(&provider.exec) {
            "ok"
        } else {
            "missing"
        };
        println!(
            "{:<16} {:<24} {:<32} {}",
            provider.id,
            provider.name,
            provider.exec.join(" "),
            status
        );
    }
    Ok(())
}

fn theme_command(
    theme_override: Option<&str>,
    config: &Config,
    action: Option<ThemeCommand>,
) -> Result<()> {
    match action.unwrap_or(ThemeCommand::Show) {
        ThemeCommand::Show => {
            let choice = theme::resolve(theme_override, config)?;
            println!("mode:   {}", choice.mode.as_str());
            println!("name:   {}", choice.label());
            println!("source: {}", choice.source.as_str());
            match &choice.path {
                Some(path) => println!("theme:  {}", path.display()),
                None => println!("theme:  (rofi default)"),
            }
            Ok(())
        }
        ThemeCommand::Set { mode } => {
            let mode = theme::parse_mode(&mode).map_err(|error| {
                anyhow::anyhow!(
                    "{error}; use one of: {}",
                    Mode::ALL
                        .iter()
                        .map(|mode| mode.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            theme::save_mode(mode)?;
            println!("theme set to {} ({})", mode.label(), mode.as_str());
            Ok(())
        }
        ThemeCommand::List => {
            let saved = theme::saved_mode();
            for mode in Mode::ALL {
                let marker = if saved == Some(mode) { "*" } else { " " };
                println!("{marker} {:<10} {}", mode.as_str(), mode.label());
            }
            println!("\n* = saved choice; set with `rofi-hub theme set MODE`");
            Ok(())
        }
        ThemeCommand::Dump { app } => {
            let choice = theme::resolve(theme_override, config)?;
            print!("{}", choice.theme.rasi_with(ChromeProfile::for_app(&app)));
            Ok(())
        }
    }
}

fn doctor(
    theme_override: Option<&str>,
    config: &Config,
    json_output: bool,
    config_warnings: &[String],
) -> Result<()> {
    let manifests = ManifestSet::discover();
    let rofi = Rofi::find().ok();
    let rofi_version = rofi.as_ref().and_then(Rofi::version);
    let choice = theme::resolve(theme_override, config).ok();
    let providers: Vec<_> = manifests
        .providers()
        .iter()
        .map(|provider| {
            json!({
                "id": provider.id,
                "name": provider.name,
                "exec": provider.exec,
                "available": exec_available(&provider.exec),
                "manifest": provider.path.as_ref().map(|path| path.display().to_string()),
            })
        })
        .collect();
    let all_available = manifests
        .providers()
        .iter()
        .all(|provider| exec_available(&provider.exec));
    let ok = rofi.is_some()
        && !manifests.is_empty()
        && all_available
        && choice.as_ref().is_some_and(|choice| choice.path.is_some());

    let warnings: Vec<String> = config_warnings
        .iter()
        .cloned()
        .chain(manifests.warnings().iter().cloned())
        .collect();

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": ok,
                "rofi": {
                    "path": rofi.as_ref().map(|rofi| rofi.binary().display().to_string()),
                    "version": rofi_version,
                },
                "config": {
                    "path": crate::config::config_path().display().to_string(),
                    "exists": crate::config::config_path().is_file(),
                },
                "provider_dirs": paths::provider_dirs().iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
                "providers": providers,
                "theme": choice.as_ref().map(|choice| json!({
                    "mode": choice.mode.as_str(),
                    "label": choice.label(),
                    "source": choice.source.as_str(),
                    "path": choice.path.as_ref().map(|path| path.display().to_string()),
                })),
                "warnings": warnings,
            }))?
        );
    } else {
        match &rofi {
            Some(rofi) => println!(
                "rofi           {} ({})",
                rofi.binary().display(),
                rofi_version.as_deref().unwrap_or("unknown version")
            ),
            None => println!("rofi           missing"),
        }
        let config_path = crate::config::config_path();
        println!(
            "config         {} ({})",
            config_path.display(),
            if config_path.is_file() {
                "found"
            } else {
                "defaults"
            }
        );
        println!(
            "provider dirs  {}",
            paths::provider_dirs()
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(":")
        );
        if providers.is_empty() {
            println!("providers      none");
        } else {
            println!("providers      {}", providers.len());
            for provider in manifests.providers() {
                let status = if exec_available(&provider.exec) {
                    "ok"
                } else {
                    "missing"
                };
                println!(
                    "  {:<14} {:<6} {}",
                    provider.id,
                    status,
                    provider.exec.join(" ")
                );
            }
        }
        match &choice {
            Some(choice) => {
                println!("theme          {}", choice.label());
                if let Some(path) = &choice.path {
                    println!("theme file     {}", path.display());
                }
            }
            None => println!("theme          invalid override"),
        }
        for warning in &warnings {
            println!("warning        {warning}");
        }
        println!(
            "status         {}",
            if ok { "ok" } else { "problems found" }
        );
    }

    if !ok {
        std::process::exit(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(exec: Vec<&str>, icon: Option<&str>) -> ProviderManifest {
        ProviderManifest {
            id: "screen-time".to_owned(),
            name: "Screen & Time".to_owned(),
            description: Some("Daily <totals>".to_owned()),
            category: Some("Productivity".to_owned()),
            icon: icon.map(str::to_owned),
            keywords: Vec::new(),
            exec: exec.into_iter().map(str::to_owned).collect(),
            version: Some("1.0.0".to_owned()),
            path: None,
        }
    }

    #[test]
    fn rows_escape_markup_and_show_categories() {
        let item = provider_item(&manifest(vec!["/bin/sh"], None), &Theme::builtin());
        assert_eq!(item.id, "screen-time");
        assert!(item.display.contains("Screen &amp; Time"));
        assert!(item.display.contains("Daily &lt;totals&gt;"));
        assert!(item.display.contains("Productivity"));
        assert!(item.display.contains("\u{25cf}"));
        assert!(!item.urgent);
    }

    #[test]
    fn hub_items_group_by_category() {
        let providers = vec![
            manifest(vec!["/bin/sh"], None),
            ProviderManifest {
                id: "clipboard".to_owned(),
                name: "Clipboard".to_owned(),
                description: Some("History".to_owned()),
                category: Some("Utilities".to_owned()),
                icon: None,
                keywords: Vec::new(),
                exec: vec!["/bin/sh".to_owned()],
                version: Some("1.0.0".to_owned()),
                path: None,
            },
        ];
        let items = hub_items(&providers, &Theme::gruvbox());
        assert!(items[0].nonselectable);
        assert!(items[0].display.contains("PRODUCTIVITY"));
        assert!(items.iter().any(|item| item.display.contains("UTILITIES")));
    }

    #[test]
    fn missing_executables_are_urgent() {
        let item = provider_item(
            &manifest(vec!["definitely-not-installed-xyz"], None),
            &Theme::builtin(),
        );
        assert!(item.urgent);
        assert!(item.display.contains("not found"));
    }

    #[test]
    fn exec_detection_handles_paths_and_names() {
        assert!(exec_available(&["/bin/sh".to_owned()]));
        assert!(exec_available(&["sh".to_owned()]));
        assert!(!exec_available(&["/definitely/not/here".to_owned()]));
        assert!(!exec_available(&[String::new()]));
    }
}
