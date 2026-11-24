# Architecture

Rofi Hub is a Cargo workspace with one launcher and one crate per app. The
launcher is deliberately small: it discovers declarative manifests, renders
them with `rofi -dmenu`, and starts the selected program. Apps keep their own
UI, daemons, and storage.

## Crates

| Crate | Purpose |
| --- | --- |
| `crates/rofi-core` | Manifest parsing, `rofi -dmenu` wrapper, Pango row encoding, XDG paths, palettes, notifications. |
| `crates/rofi-hub` | The `rofi-hub` launcher: menu, theme shortcuts, `list`, `doctor`, and theme commands. |
| `crates/rofi-calendar` | Month sheet with a date search box. |
| `crates/rofi-keybinds` | Hyprland shortcut reference; `exec` rows are runnable. |
| `crates/rofi-clipboard` | `cliphist` history picker backed by `wl-copy`. |
| `crates/screen-time` | The Screen Time tracker daemon, SQLite store, static Rofi timeline, doctor, snapshots, and updater integration. |

`screen-time` depends on `rofi-core` for Pango escaping and paths, but its
rendering is unchanged: it still emits a script-mode payload and its own
96-column timeline.

## Provider contract

A provider is a TOML manifest installed in one of these directories, in order:

1. `$ROFI_HUB_PROVIDER_DIR` (colon-separated)
2. `$XDG_DATA_HOME/rofi-hub/providers`
3. `rofi-hub/providers` under each `$XDG_DATA_DIRS` entry

Earlier directories win when two manifests share an `id`. A malformed manifest
is reported as a warning and skipped; it never prevents the hub from starting.

```toml
id = "screen-time"                  # lowercase letters, digits, '-' or '_'
name = "Screen Time"                # shown in the menu
description = "Daily app timelines, totals, and session details"
category = "Productivity"           # used for deterministic ordering
icon = "preferences-system-time"    # icon-theme name or path
keywords = ["time", "usage"]
exec = ["rofi-screen-time"]         # argument vector, never a shell string
version = "0.5.0"
```

`exec` is executed directly with `std::process::Command`. The hub does not
evaluate shell syntax. It passes `ROFI_HUB_PROVIDER`, `ROFI_HUB_THEME`, and,
when a generated theme exists, `ROFI_HUB_THEME_FILE` to the child. `ROFI_BIN`,
`ROFI_CONFIG`, and `ROFI_THEME` are inherited, and the Screen Time wrapper
honors the same variables.

`rofi-hub list --json` is the supported registry for Hyprland, Waybar, and
keybind menus. Those consumers should read ids, names, commands, and
availability from the hub instead of matching command strings; that avoids
label drift when a provider is renamed.

## Menu and theming

The hub builds one dmenu row per provider. The row `id` is the manifest id;
`rofi -format s` returns the text before the first NUL, which maps straight
back to the provider. Rows carry Pango display text and optional icons.

The runner resolves `ROFI_BIN` first (an invalid override is an error, not a
silent fallback), passes `ROFI_CONFIG` with `-config`, and uses a hub-specific
`-pid` file. It never calls `pkill`/`killall`; a second launch therefore cannot
close an askpass prompt or the Screen Time popup. Provider commands are
resolved from `PATH`, then from the directory containing the hub binary, so a
user install works even when `~/.local/bin` is missing from the session
`PATH`.

Theme modes are `auto`, `nord`, `solarized`, and `gruvbox`. Resolution order:

1. `--theme`
2. `ROFI_HUB_THEME`
3. `~/.config/rofi-hub/theme` (the saved menu choice)
4. `~/.config/rofi-hub/config.toml` `[theme] mode`
5. `auto`

`auto` uses `ROFI_THEME` or `~/.config/rofi/theme.rasi` when present. Otherwise
the hub generates a complete theme in `$XDG_RUNTIME_DIR/rofi-hub/theme.rasi`
(falling back to the state directory). The installed
`share/rofi-hub/themes/hub.rasi` is a readable fallback, not a file the hub
rewrites.

Menu shortcuts mirror Screen Time: `Alt+1` Nord, `Alt+2` Solarized Light,
`Alt+3` Gruvbox Dark, `Alt+4` Auto. A shortcut writes the separate `theme`
file, so `config.toml` is never rewritten by the UI.

## Installer components

The release bundle always contains the hub and every app. `install.sh` decides
what to install:

- default: hub and all apps (`screen-time`, `calendar`, `keybinds`, `clipboard`);
- `--app NAME`: hub and the named app (repeatable, comma-separated);
- `--no-hub`: apps only;
- `--all`: explicit all-in-one.

The selection is recorded at `~/.local/state/rofi-hub/components`. A later bare
`install.sh` or `--update` reuses it, so a hub-only user is not given a tracker
daemon by surprise. `--uninstall` removes the recorded selection unless flags
narrow it; `--purge` also deletes update state and the Screen Time database.

Installed paths:

| Component | Paths |
| --- | --- |
| hub | `~/.local/bin/rofi-hub`, `~/.local/share/rofi-hub/themes/`, `~/.local/share/rofi-hub/providers/*.toml` |
| calendar | `~/.local/bin/rofi-calendar`, `~/.local/share/rofi-hub/providers/calendar.toml` |
| keybinds | `~/.local/bin/rofi-keybinds`, `~/.local/share/rofi-hub/providers/keybinds.toml` |
| clipboard | `~/.local/bin/rofi-clipboard`, `~/.local/share/rofi-hub/providers/clipboard.toml` |
| screen-time | `~/.local/bin/screen-time`, `~/.local/bin/rofi-screen-time`, `~/.local/share/screen-time/`, `~/.config/systemd/user/screen-time.service` |
| update state | `~/.local/state/rofi-hub/` |
| screen-time data | `~/.local/state/screen-time/screen-time.db` |

## Update model

`install.sh` is the updater. The release pipeline publishes one tarball per
architecture plus `install.sh`, `uninstall.sh`, `minisign.pub`, `SHA256SUMS`,
and minisign signatures for `SHA256SUMS` and `install.sh`.

On install or update the installer:

1. resolves the stable release tag;
2. takes the update lock;
3. verifies `SHA256SUMS` and `install.sh` signatures with the pinned key;
4. stops the Screen Time service if it was active;
5. checkpoints the WAL and writes a `VACUUM INTO` database snapshot when the
   tracker is being installed or updated;
6. backs up every file it may replace, plus the current component list;
7. atomically installs the selected components;
8. runs `screen-time check-data` and `doctor` while the service is stopped;
9. starts the service only if it was active or this is the first install;
10. marks the backup verified and prunes old verified backups.

Any failure after the snapshot restores the previous files, database, and
service state. An interrupted update is recovered on the next run from
`update-in-progress.json` and the backup's `components` file.

Auto-update is opt-in. `--auto-update=check` installs a daily check timer;
`--auto-update=apply` installs a daily signed apply timer. Both run the saved
installer copy under `~/.local/state/rofi-hub/install.sh` with
`ROFI_HUB_AUTO_UPDATE=1`, and the apply path adds session-lock, battery, major
version, and exponential-backoff gates.

## Nix

`flake.nix` exposes:

- `packages.default` and `packages.rofi-hub`: the all-in-one symlink join;
- `packages.rofi-hub-only`: the launcher with no provider baked in;
- `packages.rofi-hub-apps`: the launcher plus calendar, keybinds, and clipboard;
- `packages.rofi-apps`: the built-in apps without the launcher;
- `packages.screen-time`: the tracker, including its provider manifest;
- `homeManagerModules.default`: installs the hub and the selected apps, and
  enables the tracker service when `screen-time` is selected;
- `homeManagerModules.screen-time`: the old standalone tracker module.

The all-in-one wrapper puts every app on `PATH` and sets
`ROFI_HUB_PROVIDER_DIR` to the package provider directories. User manifests are
still scanned after those directories.
