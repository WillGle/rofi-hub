mod cli;
mod config;
mod hub;
mod theme;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();
    if let Err(error) = hub::run(cli) {
        eprintln!("rofi-hub: {error:#}");
        rofi_core::notify::notify_error("Rofi Hub", &format!("{error:#}"));
        std::process::exit(1);
    }
}
