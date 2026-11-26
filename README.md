# rofi-apps

Independent Rofi apps and an optional launcher menu. Every app has its own
binary, provider manifest, release, version, and runtime dependencies. The
hub is no longer a runtime dependency: it only installs and manages app
packages.

Apps in this workspace:

| App | Version | Runtime tools it owns |
|---|---|---|
| `rofi-calendar` | 1.2.0 | Rofi |
| `rofi-clipboard` | 0.8.0 | Rofi, `cliphist`, `wl-copy` |
| `rofi-keybinds` | 1.0.0 | Rofi |
| `rofi-network` | 0.3.0 | Rofi, NetworkManager (`nmcli`) |
| `screen-time` | 0.5.0 | Rofi; owns its own user service |
| `rofi-hub-menu` | 0.3.0 | Optional provider menu |

`rofi-dmenu` is the shared library (`0.3.0`) used by the apps. It contains the
dmenu protocol, theme resolution, paths, and UI helpers. No app depends on the
optional menu or the installer.

> **Project status:** `screen-time-rofi` is retired and archived. This
> repository is the canonical home for Screen Time and the Rofi apps. See
> [migrating an existing install](docs/MIGRATING.md).

## UI

Every native app uses the same design language: zoned cards, 1px borders, a
shared palette, and an accent strip on the active row. The screenshots below
are real Rofi captures.

| Menu | Screen Time | Calendar |
|---|---|---|
| <img src="docs/screenshots/ui/hub-gruvbox.png?v=3" width="360" alt="Menu in Gruvbox Dark"> | <img src="docs/screenshots/ui/screen-time-gruvbox.png?v=3" width="360" alt="Screen Time in Gruvbox Dark"> | <img src="docs/screenshots/ui/calendar-gruvbox.png?v=3" width="360" alt="Calendar in Gruvbox Dark"> |

| Clipboard | Keybinds | Network |
|---|---|---|
| <img src="docs/screenshots/ui/clipboard-gruvbox.png?v=3" width="360" alt="Clipboard in Gruvbox Dark"> | <img src="docs/screenshots/ui/keybinds-gruvbox.png?v=3" width="360" alt="Keybinds in Gruvbox Dark"> | <img src="docs/screenshots/ui/network-gruvbox.png?v=4" width="360" alt="Network in Gruvbox Dark"> |

[Full screenshot matrix](docs/screenshots/ui/README.md) ·
[UI style guide](docs/UI_STYLE.md)

## Install

Only two channels are supported:

1. **Nix** — flake packages and modules (primary).
2. **`install.sh`** — signed universal release (secondary, non-Nix distros).

No `.deb`, `.rpm`, `.apk`, AUR, distro recipe, or source asset is published.

### NixOS

Add the flake input:

```nix
inputs.rofi-apps.url = "github:WillGle/rofi-hub";
```

Import the umbrella module and enable only the apps you want:

```nix
imports = [ inputs.rofi-apps.nixosModules.default ];

services.rofi-apps.calendar.enable = true;
services.rofi-apps.clipboard.enable = true;
services.rofi-apps.screen-time.enable = true;
services.rofi-apps.menu.enable = false; # optional
```

Each option installs only that app package and the app's own dependencies.
`screen-time` owns its user service; the other apps do not install services.
The old `services.rofi-hub.enable` option is kept as a deprecated alias that
maps the selected apps to `services.rofi-apps.<app>.enable`.

Individual modules are available as `nixosModules.rofi-calendar`,
`nixosModules.rofi-clipboard`, `nixosModules.rofi-keybinds`,
`nixosModules.rofi-network`, `nixosModules.screen-time`, and
`nixosModules.rofi-hub-menu`.

### Home Manager

```nix
{
  imports = [ inputs.rofi-apps.homeManagerModules.default ];

  services.rofi-apps.calendar.enable = true;
  services.rofi-apps.screen-time.enable = true;
}
```

The equivalent module names are under `homeManagerModules.*`. The deprecated
`homeManagerModules.rofi-hub` alias maps `services.rofi-hub.*` to the per-app
options and prints a warning.

### Nix profile

```bash
nix profile add github:WillGle/rofi-hub#rofi-calendar
nix profile add github:WillGle/rofi-hub#rofi-clipboard
nix profile add github:WillGle/rofi-hub#rofi-network
nix profile add github:WillGle/rofi-hub#screen-time
nix profile add github:WillGle/rofi-hub#rofi-hub-menu   # optional
```

`nix build .#rofi-calendar` builds exactly one app; its closure does not
contain the menu, the installer, or another app.

### Other Linux distributions

Install the signed per-app release as your desktop user. Rofi must already be
installed; `install.sh` only checks prerequisites, it never installs them.

```bash
base=https://github.com/WillGle/rofi-hub/releases/latest/download
curl -fsSLO https://raw.githubusercontent.com/WillGle/rofi-hub/main/minisign.pub
curl -fsSLO "$base/install.sh" "$base/install.sh.minisig"
minisign -Vm install.sh -p minisign.pub   # key id F1A59235D536DB40
bash install.sh --app calendar
```

Commands:

```bash
bash install.sh --list                  # index + installed state
bash install.sh --app calendar          # one app
bash install.sh --app calendar,clipboard
bash install.sh --all                   # every non-optional app
bash install.sh --update calendar       # one app
bash install.sh --uninstall calendar    # one app
bash install.sh --app screen-time --no-service
```

State is per app under `~/.local/state/rofi-apps/installed/<id>`. Update or
uninstall one app without touching another. The installer refuses Nix-managed
paths, verifies checksums and minisign signatures by default, rejects path
traversal, and never writes themes, `ROFI_HUB_*` variables, or `PATH` wrappers.

## Use

Direct launch works without the menu:

```bash
rofi-calendar
rofi-clipboard
rofi-keybinds
rofi-network
rofi-screen-time
```

The optional menu reads provider manifests and execs apps with your normal
environment:

```bash
rofi-hub-menu list
rofi-hub-menu calendar
rofi-hub-menu --theme nord
```

Removing `rofi-hub-menu` never breaks an app.

### Theme

Theme ownership is split into three layers:

| Layer | Owner | File |
|---|---|---|
| Base theme | user / theme module | `~/.config/rofi/theme.rasi` or `ROFI_THEME` |
| App layout | app package | `$XDG_DATA_DIRS/rofi/<app>.rasi` |
| App override | user | `~/.config/rofi/<app>.rasi` |
| Menu theme | optional menu | `share/rofi/hub-menu.rasi` + `~/.config/rofi-hub-menu/theme` |

Resolution order for every app:

1. `~/.config/rofi/<app>.rasi`
2. `ROFI_THEME`
3. `~/.config/rofi/theme.rasi` + app layout
4. packaged `$XDG_DATA_DIRS/rofi/<app>.rasi`
5. built-in fallback

### Environment

| Variable | Effect |
|---|---|
| `ROFI_BIN` | Rofi executable used by the apps and the Screen Time wrapper. |
| `ROFI_CONFIG` | Rofi config passed with `-config`. |
| `ROFI_THEME` | Full global theme override. |
| `ROFI_APPS_INDEX` | Default release index for `install.sh`. |
| `ROFI_APPS_PREFIX` | Default install prefix for `install.sh`. |
| `ROFI_APPS_PUBKEY` | minisign public key for `install.sh`. |

`ROFI_HUB_THEME`, `ROFI_HUB_THEME_FILE`, `ROFI_HUB_PROVIDER`, and
`ROFI_HUB_PROVIDER_DIR` are removed from the app path.

### Menu configuration

Optional `~/.config/rofi-hub-menu/config.toml`:

```toml
[ui]
prompt = "Rofi Hub"

[theme]
mode = "gruvbox" # auto | nord | solarized | gruvbox | /path/to/theme.rasi

[launcher]
columns = 3
lines = 10
```

The menu owns this state. Apps never read it.

## Repository layout

```
crates/rofi-dmenu/       shared dmenu/theme/path/UI library
crates/rofi-calendar/    app.toml, provider, layout, source
crates/rofi-clipboard/
crates/rofi-keybinds/
crates/rofi-network/
crates/screen-time/      tracker, Rofi UI, and its user service
crates/rofi-hub-menu/    optional provider menu
installer/               release index schema
modules/nixos/           per-app NixOS modules
modules/home-manager/    per-app Home Manager modules
install.sh               signed per-app installer
uninstall.sh             thin uninstall wrapper
flake.nix                packages and modules
scripts/                 release, test, and dependency guards
```

## Development

```bash
nix develop
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
python3 scripts/check-deps.py
python3 scripts/check-app-sync.py
scripts/test-apps.sh
scripts/test-theme.sh
scripts/test-installer.sh
scripts/package-release.sh
scripts/test-bundle.sh dist
scripts/test-release-index.sh dist
scripts/test-migration.sh
nix flake check
```

`check-deps.py` enforces the one-way dependency graph: apps may depend on
`rofi-dmenu`, never on the menu or installer. `test-apps.sh` and
`test-theme.sh` run apps against a fake Rofi in a temporary XDG environment.
`test-installer.sh` covers installation, update/uninstall independence,
missing prerequisites, bad checksums, unsigned releases, path traversal,
permissions, rollback, services, and the optional menu. `test-nix.sh` verifies
per-app closures and module ownership.

## License

MIT. See [LICENSE](LICENSE).
