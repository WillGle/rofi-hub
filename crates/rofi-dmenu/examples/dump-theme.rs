//! Print the built-in Gruvbox fallback theme for one app profile.
//!
//! Used by release tooling to regenerate the packaged `share/rofi/<app>.rasi`
//! fallbacks. Run with `cargo run -p rofi-dmenu --example dump-theme -- <app>`.

use rofi_dmenu::{ChromeProfile, Theme};

fn main() {
    let app = std::env::args().nth(1).unwrap_or_else(|| "hub".to_owned());
    print!(
        "{}",
        Theme::gruvbox().rasi_with(ChromeProfile::for_app(&app))
    );
}
