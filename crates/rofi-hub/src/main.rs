mod cli;
mod config;
mod hub;
mod theme;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();
    if let Err(error) = hub::run(cli) {
        // CLI callers get stderr. GUI paths notify from `open`/`launch`, where
        // a desktop notification is the only visible feedback.
        eprintln!("rofi-hub: {error:#}");
        std::process::exit(1);
    }
}
