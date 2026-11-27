# UI style

> **Owner:** WillGle · **Baseline:** 0.3.0 · **Updated:** 2026-11-27

[AGENTS.md](../AGENTS.md#required-ui-rules) defines the mandatory rules for every
app. This document defines how to apply them. Use `rofi_dmenu::Theme` for color
roles, `ChromeProfile` for app geometry, and `rofi_dmenu::ui` for shared
components where the app uses Rust. Script-backed apps use the same visual
roles and honor user theme overrides. Gruvbox Dark is the fallback palette.

## Palette

| Token | Gruvbox | Use |
|---|---|---|
| `base` | `#1d2021` | window background |
| `mantle` | `#282828` | card/panel surface |
| `surface_alt` | `#32302f` | section band/selected row |
| `border` | `#3c3836` | card borders/dividers |
| `text` | `#fbf1c7` | primary text |
| `muted` | `#a89984` | metadata/hints |
| `accent` | `#fabd2f` | labels/selection outline |
| `power` | `#d5c4a1` | system |
| `work` | `#b8bb26` | Work |
| `browser` | `#83a598` | Browser |
| `communication` | `#fe8019` | Communication |
| `entertainment` | `#d3869b` | Entertainment |
| `other` | `#928374` | Other |
| `danger` | `#fb4934` | errors/missing |

`Auto` = user matugen palette > static.env > Gruvbox. `ROFI_THEME` and
`~/.config/rofi/<app>.rasi` are explicit overrides. Apps never read hub state.

## Typography and spacing

- JetBrainsMono Nerd Font; 12pt body, 18pt dmenu hero, 26pt fixed-height
  Screen Time hero.
- Window inset 16px, zone spacing 12px, input padding `8px 12px` and standard row padding `6px 10px`.
- Radii: window 12px, input/header/list cards 8px, selected rows 6px.
- A selected data or action row uses a `surface_alt` fill, a 6px radius, and a visible `accent` outline around the whole row. Keep the label in the row foreground. This applies to lists, timeline rows, and individual cells in multi-column lists. Calendar’s current-day marker represents a date and has its own treatment.
- Section headers use a fixed band with an accent marker, a heading label,
  and optional muted metadata. Make headers non-actionable. If Rofi moves its
  cursor onto a header, the cursor must produce no visible change in the band,
  text, or border. Keep normal, alternate, and selected styles identical for
  that header. An action must never use the header style.
- Give actions a label and a recognizable marker or button treatment. Use a
  marked warning and an explicit label for disruptive actions. Keep list labels
  in the theme's row foreground so selection remains legible. Use weight,
  labels, or glyphs alongside color to distinguish data and actions.
- Row titles, detail text, and metadata inherit the row foreground. Metadata
  may use Pango `alpha` for hierarchy, but must not hardcode `text` or `muted`
  colors: Rofi applies the same markup to normal and selected rows. Keep only
  the leading marker's semantic role color.
- Rofi dmenu filters the text before the first NUL, not the display markup.
  Put the stable index first and the visible search fields after it when a row
  must be searchable (for example Keybinds).
- Section headers are fixed. Mark their rows active with `-a` (or equivalent
  metadata) and define the same background, text, and border for
  `element normal.active`, `alternate.active`, and `selected.active`.
- Every clickable toolbar control, including a control that expands more actions, uses a visible outline around its label and shortcut. Use a `mantle` fill, 8px radius, and `6px 10px` padding for text buttons. Use `border` for routine controls, `accent` for the primary action, and `danger` for destructive actions. Passive shortcut hints use plain text.
- Use real RASI buttons for mouse actions. A `button` uses `content` and
  `action`; keep the two quoted values on separate lines when they are passed
  through `-theme-str`, because Rofi 2.0 treats multiple quoted strings on one
  line as one string.
- Center aligned content as a block using widget alignment. Give grouped
  controls and rows enough space at the desktop's actual scale, including the
  full outline of clickable buttons.
- Footer: one muted line separated by `·`; action chips use `[key label]` in
  prompts/confirmations. Primary screens use RASI buttons for mouse actions.
- Menu launcher: `[launcher] columns = N` (1–6) adds a `-theme-str`
  `listview { columns: N; }` override; the default launcher is 3 columns ×
  10 rows maximum. Text-heavy menus stay single-column. The emoji picker is
  fixed at 4 columns × 15 rows maximum; the power menu is fixed at 3 columns ×
  2 rows for its six actions.

## Components

`rofi_dmenu::ui`: `header` (three-line), `header_inline` (fixed-height apps),
`section`, `row`, `row_with_leading`, `entry_row`, `detail_row`,
`inline_detail`, `toolbar`, `hint`, `status`, `empty`, `truncate`,
`widest`, `widest_text`, `width_override`, `display_width`.

`display_width` ignores markup/entities and handles Vietnamese, emoji, and Nerd
Font glyphs. `rofi_dmenu::Category` maps Work/Browser/Communication/
Entertainment/System/Other to palette roles.

## Profiles

| App | Width | Lines | Icons | Layout |
|---|---:|---:|---|---|
| Hub menu | 96ch | 20 | no | input, message, searchable provider list; height stays fixed while filtering |
| Screen Time | 144ch/dynamic | 18 | no | header, timeline, inline app details |
| Calendar | 44ch | 1 | no | centered month message, date input, compact icon navigation plus Today |
| Clipboard | 110ch (auto-fit) | 16 | no | input, message, four RASI action buttons, list |
| Keybinds | 110ch | 10 | no | input, message, 3-column shortcut list (10 rows max), searchable ids |
| Network | 74ch | 13 | no | status, action buttons, filter, Wi-Fi and actions (12 candidates) |
| Atomic Note | 54% | up to 12 | no | action bar, filter, task status, expandable task actions, task list |

## Controls and detail panels

- **Calendar.** Keep the centered month grid and date input. Reserve six week
  rows so four-, five-, and six-week months keep the same popup height and
  button positions. Put Prev, Today, and Next in one row of RASI buttons
  outside the filterable list, with the same actions as Left, Ctrl+t, and
  Right. Distinguish today's date from a date lookup; the buttons stay visible
  while the date text is incomplete.
- **Network.** Keep nearby Wi-Fi and actions in one list. Show 12
  candidates plus the section header in the main view. Use a colored dot and bold SSID for a network,
  and an arrow or warning marker for an action. The Details button and `Alt+i`
  toggle the one panel above the list. Put IP, gateway, VPN, DNS and supplemental
  link fields there once. With details expanded, keep more than four nearby
  networks visible and leave the list scrollable. Saved connections expand
  under their row.
- **Screen Time.** Keep the day summary, timeline and the selected app's
  expanded detail on the same screen. Show peak activity once in the DAY
  ACTIVITY header, never in both the summary and the header; when there is no
  peak, keep the zero state readable. Open with the first available app
  selected. `Ctrl+PgUp` and `Ctrl+PgDn` move through app groups. The row under
  the cursor is separate from the expanded detail; after a theme change restore
  that app on the same page, falling back to the first app row only when it is
  gone. Read-only rows and section titles retain their appearance during
  navigation.
- **Clipboard.** Show the full history total once. Report the matching count
  live while the user types, including zero matches, without repeating the
  total in the section band. Use RASI buttons for Copy, Delete, Clear, and
  Refresh. They
  share the keyboard paths: Enter copies, `Alt+d` deletes the highlighted entry,
  `Alt+x` clears the entire history after confirmation, and `Alt+r` refreshes.
- **Hub Menu.** Keep the typed filter, selected provider id, and window height
  across `Ctrl+1–4`. Apply the selected palette over the resolved theme instead
  of replacing the base layout.
- **Keybinds.** Match both the shortcut and its description. Keep a stable row
  index for dispatch; section headers and empty results never run a command.
  Keep the keyboard hint in the fixed message so it survives filtering, and
  keep each match's section header in the filtered list.
- **Atomic Tasks.** Keep the task list in view with Add, Open, Clear, and an
  Actions control. Keep the filter and task status in place when Actions opens,
  with the task-specific controls immediately below the status. `Enter` or
  Actions expands Done, Edit, Move, and the four priorities for the task under
  the cursor. Use no second task menu; keep the
  existing `Alt+d`, `Alt+e`, `Alt+Up`, `Alt+1–4`, undo, lock, and Waybar
  behavior. Preserve `Alt+1–4` for priorities.

Use `Ctrl+1` for Nord, `Ctrl+2` for Solarized Light, `Ctrl+3` for Gruvbox Dark
and `Ctrl+4` to restore the configured theme. Apply the chosen colors over the
configured theme and keep its typography, dimensions, spacing, and widget order.
Keep typed input, the selected item, and expanded details intact while switching
colors. Applying a palette must never submit an action.

`rofi-hub-menu theme dump --app <app>` prints the generated `.rasi`. Installed
`share/rofi/<app>.rasi` fallbacks are checked against the generator.

## Theme ownership

- Base theme: `~/.config/rofi/theme.rasi`.
- App layout: `$XDG_DATA_DIRS/rofi/<app>.rasi`.
- App override: `~/.config/rofi/<app>.rasi`.
- App theme lock: `~/.local/state/rofi-apps/<app>/theme-choice` for Calendar,
  Clipboard, Keybinds, Network, and Atomic Tasks; Screen Time keeps its own
  `theme-choice` in its state directory.
- Menu theme: `share/rofi/hub-menu.rasi` plus
  `~/.config/rofi-hub-menu/theme`.

Precedence: app override, `ROFI_THEME`, base + layout, packaged fallback,
built-in fallback. There is no `ROFI_HUB_THEME` or `ROFI_HUB_THEME_FILE`.
Screen Time owns its `theme-choice` state; the menu owns its own. Calendar,
Clipboard, Keybinds, Network, and Atomic Tasks persist their palette lock per
app and `Ctrl+4` removes that lock. The
menu writes a palette layer over the resolved menu theme, so its font, geometry,
filter, selection, and list content survive `Ctrl+1–4`; `Ctrl+4` returns to the
configured theme rather than a hardcoded palette.

## Screenshots and validation

For each UI change, run the relevant existing checks and exercise the changed
path in Rofi with a disposable home or fixture data. The [screenshot
matrix](screenshots/ui/README.md) describes the Xvfb workflow. Use the desktop
theme and scale when reproducing a reported visual issue. Current §18
interaction captures and fixtures are listed in the same screenshot document.

- For a shared component change, open the affected app through its normal launcher and compare the component with another app under the same palette, font, and scale. Check both against the written rules, including styles applied by launcher arguments. Use a disposable configuration without app-specific overrides for this comparison.
- For a layout change, verify that all labels, shortcut hints, and button outlines fit the
  window, the main action is visible, and the list remains readable when
  filtered or scrolled.
- For a header or selection change, hover and click the header and navigate
  through it with the keyboard. Its appearance must stay fixed and Enter must
  run no action on it. Check adjacent selectable rows for readable text.
- For a theme change, compare the same view with the same font and scale before
  and after each shortcut. The window dimensions and layout must match. Verify
  that Enter still targets the same item after switching.
- For an inline panel, open and close it with its button and shortcut. Confirm
  that the primary screen remains available and the same control or data is
  represented only once.

Document the specific cases checked and any remaining unverified behavior.
