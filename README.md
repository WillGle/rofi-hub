# rofi-hub

A small Rust workspace for Rofi desktop tools. The `rofi-hub` launcher shows
installed sub-apps in one keyboard-driven menu, remembers your theme, and
starts the selected app. The first app is **Screen Time**, a local Hyprland
tracker with daily totals, timelines, and session details.

One signed bundle contains the hub and every app. The installer can install the
all-in-one hub, only selected apps, or only the launcher.

> **Project status:** `screen-time-rofi` is retired and archived. This
> repository is the canonical home for Screen Time and future Rofi apps. See
> [migrating an existing install](docs/MIGRATING.md).

[![Memory and fast-switch coverage across UI-capable Linux trackers](crates/screen-time/docs/charts/memory-coverage.svg)](crates/screen-time/docs/EVALUATION.md)

## Quick start

On Linux with Hyprland, Rofi 2.x, minisign, and a systemd user session, download
and verify the release installer as your desktop user:

```bash
base=https://github.com/WillGle/rofi-hub/releases/latest/download
curl -fsSLO https://raw.githubusercontent.com/WillGle/rofi-hub/main/minisign.pub
curl -fsSLO "$base/install.sh" "$base/install.sh.minisig"
minisign -Vm install.sh -p minisign.pub   # key id F1A59235D536DB40
less install.sh
bash install.sh
~/.local/bin/rofi-hub
```

`install.sh` with no component flags installs the hub and every app.

```bash
# the hub plus only Screen Time (the current default)
bash install.sh --app screen-time

# Screen Time without the hub launcher
bash install.sh --app screen-time --no-hub

# list the apps in the bundle
bash install.sh --list-apps

# pick a theme while installing
bash install.sh --theme nord
```

Updates are opt-in and signed:

```bash
bash install.sh --check                 # report only
bash install.sh --update                # install a newer signed release
bash install.sh --auto-update=check     # daily check timer
bash install.sh --auto-update=apply     # daily signed auto-apply
bash install.sh --rollback              # restore the last verified backup
bash uninstall.sh                       # remove files; keep history
bash uninstall.sh --purge --yes         # remove update state and history
```

The updater stops the tracker, snapshots the database, verifies the release,
keeps a rollback copy, and restores files, data, and service state on failure.
See [data safety and updates](crates/screen-time/docs/AUTO_UPDATE.md).

## Nix and Home Manager

```bash
nix build .#                # all-in-one (hub + Screen Time)
nix build .#rofi-hub-only   # launcher only
nix build .#screen-time     # the tracker only
./result/bin/rofi-hub
```

With flakes and Home Manager:

```nix
{
  inputs.rofi-hub.url = "github:WillGle/rofi-hub";

  # in your Home Manager configuration
  imports = [ inputs.rofi-hub.homeManagerModules.default ];

  services.rofi-hub = {
    enable = true;
    apps = [ "screen-time" ];       # [] installs only the hub
    # providerDirs = [ "/home/me/.local/share/rofi-hub/providers" ];
  };
}
```

`services.screen-time.enable = true` is still available through
`homeManagerModules.screen-time` for standalone tracker installs.

## Use

### Hub

- `↑` / `↓` selects an app, `Enter` opens it, and `Esc` closes the menu.
- Type to filter.
- `Alt+1` Nord, `Alt+2` Solarized Light, `Alt+3` Gruvbox Dark, `Alt+4` Auto.
  The choice is saved under `~/.config/rofi-hub/theme`.
- `rofi-hub --theme nord` overrides the theme for one run.
- `rofi-hub theme list`, `rofi-hub theme set nord`, and `rofi-hub theme show`
  manage the saved theme from a terminal.
- `rofi-hub screen-time` opens one app directly.

### Screen Time

- `←` / `→` browse days, `T` returns to today, and `R` refreshes.
- `↑` / `↓` selects an app, `Enter` opens its details, and `Backspace` returns.
- `Alt+1` through `Alt+3` select Nord, Solarized Light, or Gruvbox Dark;
  `Alt+4` clears the saved choice and restores the configured theme.

See the [Screen Time user guide](crates/screen-time/docs/USER_GUIDE.md) for
distro dependencies, configuration, retention, and troubleshooting.

## Privacy and data

Screen Time stores app-level history only. Window titles are parsed just far
enough to find the event separator and are never persisted or rendered. Idle
and suspend gaps are excluded. The database lives at
`~/.local/state/screen-time/screen-time.db`; updates and backups live at
`~/.local/state/rofi-hub/`.

## Repository layout

```
crates/rofi-core/       provider manifests, Rofi dmenu, themes, paths
crates/rofi-hub/        the hub launcher
crates/screen-time/     the Screen Time tracker and its Rofi UI
providers/              provider manifests shipped in the bundle
themes/                 fallback hub theme
install.sh              signed bundle installer/updater
flake.nix               packages and the Home Manager module
```

## Environment

| Variable | Effect |
| --- | --- |
| `ROFI_BIN` | Rofi executable used by the hub and the Screen Time wrapper. |
| `ROFI_CONFIG` | Extra Rofi config passed with `-config`; both UIs honor it. |
| `ROFI_THEME` | Base theme used by the hub's `auto` mode and the Screen Time fallback. |
| `ROFI_HUB_THEME` | One-run theme override; does not change the saved choice. |
| `ROFI_HUB_PROVIDER_DIR` | Extra provider directories, colon-separated. |
| `ROFI_HUB_PROVIDER`, `ROFI_HUB_THEME`, `ROFI_HUB_THEME_FILE` | Set for the launched provider. |

Providers are resolved from `PATH`, then from the directory containing the
`rofi-hub` binary, so a `~/.local/bin` install works even when the graphical
session does not include that directory in `PATH`. The hub never kills other
Rofi processes; each app owns its PID file.

`rofi-hub list --json` is the stable registry for desktop integrations
(Hyprland, Waybar, keybind menus): read ids, names, commands, and availability
from it instead of hardcoding labels.

See [architecture](docs/ARCHITECTURE.md) for the provider contract and update
model, and [migration notes](docs/MIGRATING.md) if you used the retired
`screen-time-rofi` repository.

## Development

```bash
nix develop
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
shellcheck install.sh uninstall.sh scripts/*.sh
scripts/package-release.sh
scripts/test-installer.sh
nix flake check
```

The release workflow runs the same checks, builds x86_64 and aarch64 bundles,
signs `SHA256SUMS` and `install.sh`, and uploads them to the GitHub release.

## License

MIT. See [LICENSE](LICENSE).
