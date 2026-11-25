# UI style

> **Owner:** WillGle · **Baseline:** `v0.2.1` · **Updated:** 2026-11-26

All native apps use one zoned Gruvbox Dark (hard contrast) language. Tokens live
in `rofi_core::Theme`; layout in `ChromeProfile`; components in `rofi_core::ui`.

## Palette

| Token | Gruvbox | Use |
|---|---|---|
| `base` | `#1d2021` | window background |
| `mantle` | `#282828` | card/panel surface |
| `surface_alt` | `#32302f` | section band/selected row |
| `border` | `#3c3836` | card borders/dividers |
| `text` | `#fbf1c7` | primary text |
| `muted` | `#a89984` | metadata/hints |
| `accent` | `#fabd2f` | labels/selection strip |
| `power` | `#d5c4a1` | system |
| `work` | `#b8bb26` | Work |
| `browser` | `#83a598` | Browser |
| `communication` | `#fe8019` | Communication |
| `entertainment` | `#d3869b` | Entertainment |
| `other` | `#928374` | Other |
| `danger` | `#fb4934` | errors/missing |

`Auto` = matugen > static.env > Gruvbox. `ROFI_THEME` / `--theme path.rasi`
remain explicit overrides.

## Typography and spacing

- JetBrainsMono Nerd Font; 12pt body, 18pt dmenu hero, 26pt fixed-height
  Screen Time hero.
- Window inset 12px; cards use 6px/8px padding; rows `4px 10px`.
- Radii: window 12px, input/header/list cards 8px, rows 4px.
- Selected row: `surface_alt` + `accent` left strip and border.
- Section: `.active()` band, `▌ TITLE` accent left, muted meta right.
- Footer: one muted line separated by `·`; action chips use `[key label]`.
- App launcher: `[launcher] columns = N` (1–6) adds a `-theme-str`
  `listview { columns: N; }` override; text-heavy apps stay single-column.

## Components

`rofi_core::ui`: `header` (three-line), `header_inline` (fixed-height apps),
`section`, `row`, `row_with_leading`, `entry_row`, `detail_row`,
`inline_detail`, `toolbar`, `hint`, `status`, `empty`, `truncate`,
`widest`, `widest_text`, `width_override`, `display_width`.

`display_width` ignores markup/entities and handles Vietnamese, emoji, and Nerd
Font glyphs. `rofi_core::Category` maps Work/Browser/Communication/
Entertainment/System/Other to palette roles.

## Profiles

| App | Width | Lines | Icons |
|---|---:|---:|---|
| Hub | 96ch | 20 | no |
| Screen Time | 144ch/dynamic | 18 | no |
| Calendar | 64ch (auto-fit) | 12 | no |
| Clipboard | 110ch (auto-fit) | 16 | no |
| Keybinds | 110ch | 16 | no |
| Network | 90ch (auto-fit) | 12 | yes |

`rofi-hub theme dump --app <app>` prints the generated `.rasi`. Installed
`themes/hub.rasi` and Screen Time fallback are checked against the generator.

## Theme state

`~/.config/rofi-hub/theme` is shared. `Alt+1..4` updates it; the hub exports
`ROFI_HUB_THEME` and an app-scoped `ROFI_HUB_THEME_FILE`; Screen Time reads hub
values, global state, legacy `theme-choice` once, then its own config.

## Screenshots and validation

- Matrix: [`docs/screenshots/ui/`](screenshots/ui/README.md).
- `cargo test -p rofi-core`, `cargo test --workspace`,
  `scripts/test-install-tree.sh`, `nix flake check`.
