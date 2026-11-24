use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "rofi-hub",
    version,
    about = "A small Rofi launcher for screen-time and future sub-apps",
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    /// Provider id to open directly (for example, `screen-time`).
    pub provider: Option<String>,

    /// Theme override: auto, nord, solarized, gruvbox, or a `.rasi` path.
    #[arg(long, global = true, value_name = "MODE")]
    pub theme: Option<String>,

    /// Do not pass a generated theme to Rofi.
    #[arg(long, global = true)]
    pub no_theme: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Open the hub menu, or one provider by id.
    Open {
        /// Provider id to open directly.
        provider: Option<String>,
    },
    /// List installed providers.
    List {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show or change the hub theme.
    Theme {
        #[command(subcommand)]
        action: Option<ThemeCommand>,
    },
    /// Check Rofi, provider manifests, and themes.
    Doctor {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum ThemeCommand {
    /// Print the effective theme and where it came from.
    Show,
    /// Persist a theme choice: auto, nord, solarized, or gruvbox.
    Set {
        #[arg(value_name = "MODE")]
        mode: String,
    },
    /// List the built-in modes.
    List,
    /// Print the generated Rofi theme for the effective mode.
    Dump,
}
