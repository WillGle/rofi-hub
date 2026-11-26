# UI screenshot matrix

Real Rofi captures of the six native apps in four theme modes, generated
headlessly with Xvfb, Rofi 2.x, and JetBrainsMono Nerd Font.

The README shows the Gruvbox row. Every app also has `nord`, `solarized`, and
`auto` captures in this directory.

| App | Gruvbox Dark | Nord | Solarized Light | Auto |
|---|---|---|---|---|
| Menu | [![Menu Gruvbox](hub-gruvbox.png?v=3)](hub-gruvbox.png) | [![Menu Nord](hub-nord.png?v=3)](hub-nord.png) | [![Menu Solarized](hub-solarized.png?v=3)](hub-solarized.png) | [![Menu Auto](hub-auto.png?v=3)](hub-auto.png) |
| Screen Time | [![Screen Time Gruvbox](screen-time-gruvbox.png?v=3)](screen-time-gruvbox.png) | [![Screen Time Nord](screen-time-nord.png?v=3)](screen-time-nord.png) | [![Screen Time Solarized](screen-time-solarized.png?v=3)](screen-time-solarized.png) | [![Screen Time Auto](screen-time-auto.png?v=3)](screen-time-auto.png) |
| Calendar | [![Calendar Gruvbox](calendar-gruvbox.png?v=5)](calendar-gruvbox.png) | [![Calendar Nord](calendar-nord.png?v=5)](calendar-nord.png) | [![Calendar Solarized](calendar-solarized.png?v=5)](calendar-solarized.png) | [![Calendar Auto](calendar-auto.png?v=5)](calendar-auto.png) |
| Clipboard | [![Clipboard Gruvbox](clipboard-gruvbox.png?v=3)](clipboard-gruvbox.png) | [![Clipboard Nord](clipboard-nord.png?v=3)](clipboard-nord.png) | [![Clipboard Solarized](clipboard-solarized.png?v=3)](clipboard-solarized.png) | [![Clipboard Auto](clipboard-auto.png?v=3)](clipboard-auto.png) |
| Keybinds | [![Keybinds Gruvbox](keybinds-gruvbox.png?v=3)](keybinds-gruvbox.png) | [![Keybinds Nord](keybinds-nord.png?v=3)](keybinds-nord.png) | [![Keybinds Solarized](keybinds-solarized.png?v=3)](keybinds-solarized.png) | [![Keybinds Auto](keybinds-auto.png?v=3)](keybinds-auto.png) |
| Network | [![Network Gruvbox](network-gruvbox.png?v=5)](network-gruvbox.png) | [![Network Nord](network-nord.png?v=5)](network-nord.png) | [![Network Solarized](network-solarized.png?v=5)](network-solarized.png) | [![Network Auto](network-auto.png?v=5)](network-auto.png) |

The `auto` column falls back to Gruvbox when no matugen palette or
`static.env` is present. See [UI style](../../UI_STYLE.md) for the palette,
component grammar, and `ChromeProfile` table.

## Calendar and Network redesign

The §17 redesign captures are in [`redesign/`](redesign/). Each pair keeps the
pre-redesign image from the previous commit and the real Rofi capture after the
change, at 1280×800 with the same font and themes:

| App | Before | After |
|---|---|---|
| Calendar | [Gruvbox](redesign/calendar-before-gruvbox.png) | [Gruvbox](redesign/calendar-after-gruvbox.png), [Nord](redesign/calendar-after-nord.png), [Solarized Light](redesign/calendar-after-solarized.png), [Auto](redesign/calendar-after-auto.png) |
| Network | [Gruvbox](redesign/network-before-gruvbox.png) | [Gruvbox](redesign/network-after-gruvbox.png), [Nord](redesign/network-after-nord.png), [Solarized Light](redesign/network-after-solarized.png), [Auto](redesign/network-after-auto.png) |

Interaction states:

- Calendar date entry: [focused date](redesign/calendar-focus-gruvbox.png),
  [invalid date](redesign/calendar-invalid.png).
- Calendar narrow window: [1024×700](redesign/calendar-narrow.png).
- Network Manage: [Gruvbox](redesign/network-manage-gruvbox.png),
  [Nord](redesign/network-manage-nord.png),
  [Solarized Light](redesign/network-manage-solarized.png).
- Network Connection details:
  [Gruvbox](redesign/network-details-gruvbox.png),
  [Nord](redesign/network-details-nord.png),
  [Solarized Light](redesign/network-details-solarized.png).
- Network narrow window: [1024×700](redesign/network-narrow.png).
- Network empty state: [only active network](redesign/network-only-active.png),
  [Wi-Fi off + Ethernet](redesign/network-wifi-off.png).
- Long SSID truncation: [1024×700](redesign/network-long-ssid.png).
- Full-name filter still matches the truncated SSID:
  [filtered list](redesign/network-filter-long.png).

## Regenerating

The captures use the real app binaries through Rofi dmenu, not mockups. The
generation environment needs an X11-capable Rofi, `Xvfb`, and ImageMagick:

```bash
cargo build --release --locked --workspace

# Install the app payloads into a throwaway prefix.
DIST=$(mktemp -d)
DIST="$DIST" scripts/package-release.sh
scripts/test-bundle.sh "$DIST"   # verifies install.sh before capture
HOME=/tmp/ui-home PREFIX=/tmp/ui-stage \
  bash install.sh --index "$DIST/index.json" --allow-unsigned --all

Xvfb :99 -screen 0 1280x800x24 &
env -u WAYLAND_DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_SESSION_TYPE \
  DISPLAY=:99 HOME=/tmp/ui-home XDG_CONFIG_HOME=/tmp/ui-config \
  XDG_DATA_HOME=/tmp/ui-data XDG_RUNTIME_DIR=/tmp/ui-run \
  PATH="/tmp/ui-stage/bin:$PATH" \
  rofi-calendar &
DISPLAY=:99 import -window root docs/screenshots/ui/calendar-gruvbox.png
```

Use the app binary directly (`rofi-calendar`, `rofi-clipboard`,
`rofi-keybinds`, `rofi-network`, `rofi-screen-time`) or the optional menu
(`rofi-hub-menu`). Theme selection uses the user base theme
`~/.config/rofi/theme.rasi`, an app override `~/.config/rofi/<app>.rasi`,
`ROFI_THEME`, or the menu's own `rofi-hub-menu theme set` state. Apps do not
read `ROFI_HUB_THEME`.

The redesign captures use `xdotool` for the date-entry, Manage, and details
states, and set a full `ROFI_THEME` (from
`cargo run -p rofi-dmenu --example dump-theme -- <app> <mode>`) plus
`~/.config/theme/static.env` for the Nord/Solarized markup palette. Auto uses
the built-in Gruvbox fallback. `scripts/test-performance.sh` measures the same
binaries with the fake runtime tools.

The committed `fixtures/` directory contains the fake `cliphist`, `wl-copy`,
and `nmcli` scripts plus a small `hyprland.conf` for deterministic captures.
Network captures use `fixtures/nmcli`: example SSIDs, TEST-NET-1 IPv4
(`192.0.2.0/24`), and the locally administered MAC
`02:00:00:00:00:01`.
