# UI screenshot matrix

Real Rofi captures of the six native apps in four theme modes, generated
headlessly with Xvfb, Arch's Rofi 2.x, and JetBrainsMono Nerd Font.

The README shows the Gruvbox row. Every app also has `nord`, `solarized`, and
`auto` captures in this directory.

| App | Gruvbox Dark | Nord | Solarized Light | Auto |
|---|---|---|---|---|
| Hub | [![Hub Gruvbox](hub-gruvbox.png)](hub-gruvbox.png) | [![Hub Nord](hub-nord.png)](hub-nord.png) | [![Hub Solarized](hub-solarized.png)](hub-solarized.png) | [![Hub Auto](hub-auto.png)](hub-auto.png) |
| Screen Time | [![Screen Time Gruvbox](screen-time-gruvbox.png)](screen-time-gruvbox.png) | [![Screen Time Nord](screen-time-nord.png)](screen-time-nord.png) | [![Screen Time Solarized](screen-time-solarized.png)](screen-time-solarized.png) | [![Screen Time Auto](screen-time-auto.png)](screen-time-auto.png) |
| Calendar | [![Calendar Gruvbox](calendar-gruvbox.png)](calendar-gruvbox.png) | [![Calendar Nord](calendar-nord.png)](calendar-nord.png) | [![Calendar Solarized](calendar-solarized.png)](calendar-solarized.png) | [![Calendar Auto](calendar-auto.png)](calendar-auto.png) |
| Clipboard | [![Clipboard Gruvbox](clipboard-gruvbox.png)](clipboard-gruvbox.png) | [![Clipboard Nord](clipboard-nord.png)](clipboard-nord.png) | [![Clipboard Solarized](clipboard-solarized.png)](clipboard-solarized.png) | [![Clipboard Auto](clipboard-auto.png)](clipboard-auto.png) |
| Keybinds | [![Keybinds Gruvbox](keybinds-gruvbox.png)](keybinds-gruvbox.png) | [![Keybinds Nord](keybinds-nord.png)](keybinds-nord.png) | [![Keybinds Solarized](keybinds-solarized.png)](keybinds-solarized.png) | [![Keybinds Auto](keybinds-auto.png)](keybinds-auto.png) |
| Network | [![Network Gruvbox](network-gruvbox.png)](network-gruvbox.png) | [![Network Nord](network-nord.png)](network-nord.png) | [![Network Solarized](network-solarized.png)](network-solarized.png) | [![Network Auto](network-auto.png)](network-auto.png) |

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
DISPLAY=:99 HOME=/tmp/ui-home XDG_CONFIG_HOME=/tmp/ui-config \
  XDG_DATA_HOME=/tmp/ui-data XDG_RUNTIME_DIR=/tmp/ui-run \
  ROFI_HUB_THEME=gruvbox PATH=/tmp/ui-stage/usr/bin:$PATH \
  rofi-hub &
DISPLAY=:99 import -window root docs/screenshots/ui/hub-gruvbox.png
```

Set `ROFI_HUB_THEME` to `nord`, `solarized`, `gruvbox`, or unset it for `auto`.
Fake `cliphist`, `wl-copy`, `nmcli`, and a small `hyprland.conf` provide
deterministic data for the non-hub apps.
