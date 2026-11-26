//! Print a built-in theme for one app profile.
//!
//! Used by release tooling and the screenshot matrix to regenerate the
//! packaged `share/rofi/<app>.rasi` fallbacks and mode-specific captures.
//! Run with `cargo run -p rofi-dmenu --example dump-theme -- <app> [mode]`.

use rofi_dmenu::{ChromeProfile, Mode, Theme};

fn main() {
    let mut args = std::env::args().skip(1);
    let app = args.next().unwrap_or_else(|| "hub".to_owned());
    let mode = args
        .next()
        .and_then(|value| value.parse::<Mode>().ok())
        .unwrap_or(Mode::Gruvbox);
    print!(
        "{}",
        Theme::for_mode(mode).rasi_with(ChromeProfile::for_app(&app))
    );
}
