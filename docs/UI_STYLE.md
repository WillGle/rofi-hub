# UI style

> **Owner:** WillGle · **Baseline:** `v0.2.1` · **Updated:** 2026-11-25

All native apps use one flat Gruvbox Dark (hard contrast) language. Tokens live
in `rofi_core::Theme`; layout in `ChromeProfile`; components in `rofi_core::ui`.

## Palette

| Token | Gruvbox | Use |
|---|---|---|
| `base` | `#1d2021` | window/list background |
| `mantle` | `#282828` | selected row/panel |
| `surface_alt` | `#32302f` | hover/pressed |
| `border` | `#3c3836` | dividers |
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

- JetBrainsMono Nerd Font; 12pt body, 26pt hero, 10–11pt hints.
- Window padding 20px; row padding `3px 10px`.
- No radius, shadow, scrollbar, or hover.
- Selected row: `mantle` + 2px `accent` left strip.
- Section: `▌ TITLE` accent left, muted meta right.
- Footer: one muted line separated by `·`.

## Components

`rofi_core::ui`: `header` (three-line), `header_inline` (fixed-height apps),
`section`, `row`, `row_with_leading`, `hint`, `status`, `empty`,
`display_width`.

`display_width` ignores markup/entities and handles Vietnamese, emoji, and Nerd
Font glyphs. `rofi_core::Category` maps Work/Browser/Communication/
Entertainment/System/Other to palette roles.

## Profiles

| App | Width | Lines | Icons |
|---|---:|---:|---|
| Hub | 96ch | 20 | no |
| Screen Time | 144ch/dynamic | 24 | no |
| Calendar | 64ch | 12 | no |
| Clipboard | 96ch | 20 | no |
| Keybinds | 110ch | 20 | no |
| Network | 90ch | 20 | no |

`rofi-hub theme dump --app <app>` prints the generated `.rasi`. Installed
`themes/hub.rasi` and Screen Time fallback are checked against the generator.

## Theme state

`~/.config/rofi-hub/theme` is shared. `Alt+1..4` updates it; the hub exports
`ROFI_HUB_THEME` + `ROFI_HUB_THEME_FILE`; Screen Time reads hub values, global
state, legacy `theme-choice` once, then its own config.

## Screenshots and validation

- Matrix: [`docs/screenshots/ui/`](screenshots/ui/README.md).
- `cargo test -p rofi-core`, `cargo test --workspace`,
  `scripts/test-install-tree.sh`, `nix flake check`.
