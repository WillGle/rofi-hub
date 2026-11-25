# UI style

> **Owner:** WillGle · **Baseline:** `v0.2.1` · **Updated:** 2026-11-25

Every native app in `rofi-hub` uses one visual language: a flat terminal/TUI
dashboard inspired by Gruvbox Dark (hard contrast). The palette lives in
`rofi_core::Theme`; layout chrome comes from `rofi_core::ChromeProfile`; rows
and headers come from `rofi_core::ui`.

## Palette

| Token | Gruvbox | Use |
|---|---|---|
| `base` | `#1d2021` | window and list background |
| `mantle` | `#282828` | selected row, panel |
| `surface_alt` | `#32302f` | hover/pressed |
| `border` | `#3c3836` | dividers (sparingly) |
| `text` | `#fbf1c7` | primary values |
| `muted` | `#a89984` | metadata, hints |
| `accent` | `#fabd2f` | labels, markers, selected strip |
| `power` | `#d5c4a1` | system/power |
| `work` | `#b8bb26` | Work category |
| `browser` | `#83a598` | Browser category |
| `communication` | `#fe8019` | Communication category |
| `entertainment` | `#d3869b` | Entertainment category |
| `other` | `#928374` | Unknown/Other |
| `danger` | `#fb4934` | errors and missing providers |

`Nord`, `Solarized Light`, and `Auto` are complete role presets in
`rofi_core::Theme`. `Auto` resolves `matugen > static.env > Gruvbox`; it no
longer reads an implicit `~/.config/rofi/theme.rasi`. `ROFI_THEME` and
`--theme /path.rasi` remain explicit escape hatches, but row markup may not
match a third-party `.rasi` exactly.

## Typography and spacing

- Font: JetBrainsMono Nerd Font, 12pt; hero 26pt, hints 10-11pt.
- Window padding 20px; row padding `3px 10px`; section row `6px 10px`.
- No border radius, shadow, or scrollbar.
- Selected row: `mantle` plus a 2px `accent` left strip.
- Section: `▌ TITLE` in accent on the left, muted meta on the right.
- Footer: one muted line joined with `·`.

## Components

`rofi_core::ui` provides `header`, `section`, `row`, `row_with_leading`,
`hint`, `status`, `empty`, and `display_width`. `display_width` ignores Pango
markup and entities and measures combining marks, Vietnamese text, emoji, and
Nerd Font glyphs with `unicode-width`.

`rofi_core::category::Category` maps Work, Browser, Communication,
Entertainment, System, and Other to the theme roles above.

## Profiles

| App | Width | Lines | Icons | Header |
|---|---:|---:|---|---|
| Hub | 96ch | 20 | yes | `ROFI HUB` + app count |
| Screen Time | 144ch (dynamic) | 24 | no | hero duration + date |
| Calendar | 64ch | 12 | no | `CALENDAR` + month/year |
| Clipboard | 96ch | 20 | no | `CLIPBOARD` + item count |
| Keybinds | 110ch | 20 | no | `SHORTCUTS` + binding count |
| Network | 90ch | 20 | yes | `NETWORK` + state |

`rofi-hub theme dump --app <app>` prints the generated `.rasi` for a profile.
The installed `themes/hub.rasi` and
`crates/screen-time/share/rofi/screen-time.rasi` are checked against the
generator by `rofi-core` tests.

## Theme state

The global mode is `~/.config/rofi-hub/theme`. `Alt+1..4` in any native app
updates it; the hub passes `ROFI_HUB_THEME` and `ROFI_HUB_THEME_FILE` to
providers. Screen Time reads the hub values first, then the global state, then
the legacy `theme-choice` file once for migration, then its own config. A
standalone Screen Time install without the hub still works from its config.

## Third-party Rofi modes

`drun`, `window`, `calc`, `emoji`, and `power` receive the shared config and
generated theme, so their window chrome and palette match. Their row layout is
owned by Rofi or the plugin and is not part of this contract.

## Validation

```bash
cargo test -p rofi-core
cargo test --workspace
rofi-hub theme dump --app hub
rofi-hub theme dump --app screen-time
scripts/test-install-tree.sh
```

Golden markup and `rasi` generation are intentionally updated when the palette
or component grammar changes.
