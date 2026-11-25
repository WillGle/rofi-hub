# UI screenshot matrix

Real Rofi captures of the six native apps in four theme modes, generated
headlessly with Xvfb, Arch's Rofi 2.x, and JetBrainsMono Nerd Font.

The README shows the Gruvbox row. Every app also has `nord`, `solarized`, and
`auto` captures in this directory.

| App | Gruvbox Dark | Nord | Solarized Light | Auto |
|---|---|---|---|---|
| Hub | [![Hub Gruvbox](hub-gruvbox.png?v=3)](hub-gruvbox.png) | [![Hub Nord](hub-nord.png?v=3)](hub-nord.png) | [![Hub Solarized](hub-solarized.png?v=3)](hub-solarized.png) | [![Hub Auto](hub-auto.png?v=3)](hub-auto.png) |
| Screen Time | [![Screen Time Gruvbox](screen-time-gruvbox.png?v=3)](screen-time-gruvbox.png) | [![Screen Time Nord](screen-time-nord.png?v=3)](screen-time-nord.png) | [![Screen Time Solarized](screen-time-solarized.png?v=3)](screen-time-solarized.png) | [![Screen Time Auto](screen-time-auto.png?v=3)](screen-time-auto.png) |
| Calendar | [![Calendar Gruvbox](calendar-gruvbox.png?v=3)](calendar-gruvbox.png) | [![Calendar Nord](calendar-nord.png?v=3)](calendar-nord.png) | [![Calendar Solarized](calendar-solarized.png?v=3)](calendar-solarized.png) | [![Calendar Auto](calendar-auto.png?v=3)](calendar-auto.png) |
| Clipboard | [![Clipboard Gruvbox](clipboard-gruvbox.png?v=3)](clipboard-gruvbox.png) | [![Clipboard Nord](clipboard-nord.png?v=3)](clipboard-nord.png) | [![Clipboard Solarized](clipboard-solarized.png?v=3)](clipboard-solarized.png) | [![Clipboard Auto](clipboard-auto.png?v=3)](clipboard-auto.png) |
| Keybinds | [![Keybinds Gruvbox](keybinds-gruvbox.png?v=3)](keybinds-gruvbox.png) | [![Keybinds Nord](keybinds-nord.png?v=3)](keybinds-nord.png) | [![Keybinds Solarized](keybinds-solarized.png?v=3)](keybinds-solarized.png) | [![Keybinds Auto](keybinds-auto.png?v=3)](keybinds-auto.png) |
| Network | [![Network Gruvbox](network-gruvbox.png?v=3)](network-gruvbox.png) | [![Network Nord](network-nord.png?v=3)](network-nord.png) | [![Network Solarized](network-solarized.png?v=3)](network-solarized.png) | [![Network Auto](network-auto.png?v=3)](network-auto.png) |

The `auto` column falls back to Gruvbox when no matugen palette or
`static.env` is present. See [UI style](../../UI_STYLE.md) for the palette,
component grammar, and `ChromeProfile` table.

## Regenerating

The captures use the real app binaries through Rofi dmenu, not mockups. The
generation environment needs an X11-capable Rofi, `Xvfb`, and ImageMagick:

```bash
cargo build --release --locked --workspace
DESTDIR=/tmp/ui-stage PREFIX=/usr packaging/install-tree.sh --all
Xvfb :99 -screen 0 1280x800x24 &
env -u WAYLAND_DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_SESSION_TYPE \
  DISPLAY=:99 HOME=/tmp/ui-home XDG_CONFIG_HOME=/tmp/ui-config \
  XDG_DATA_HOME=/tmp/ui-data XDG_RUNTIME_DIR=/tmp/ui-run \
  ROFI_HUB_THEME=gruvbox \
  PATH="$PWD/docs/screenshots/ui/fixtures:/tmp/ui-stage/usr/bin:$PATH" \
  rofi-hub &
DISPLAY=:99 import -window root docs/screenshots/ui/hub-gruvbox.png
```

Set `ROFI_HUB_THEME` to `nord`, `solarized`, `gruvbox`, or unset it for `auto`.
The committed `fixtures/` directory contains the fake `cliphist`, `wl-copy`,
and `nmcli` scripts plus a small `hyprland.conf` for deterministic non-hub
captures. Network captures use `fixtures/nmcli`: example SSIDs, TEST-NET-1
IPv4 (`192.0.2.0/24`), and the locally administered MAC
`02:00:00:00:00:01`.
