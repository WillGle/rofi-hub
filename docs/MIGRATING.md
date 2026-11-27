# Migrating from the old rofi-hub

The old `rofi-hub` was an installer, a runtime menu, a dependency provider, a
theme owner, and a bundle at the same time. The new architecture separates
those roles:

- apps are independent packages;
- the menu is optional and never injects env or theme;
- the installer only copies/verifies/records files;
- themes belong to the user and the app.

Migration is incremental. The installer does not delete old files or old
state; the new app packages install next to them.

## 1. Update the Nix input (recommended)

Old:

```nix
imports = [ inputs.rofi-hub.nixosModules.default ];
services.rofi-hub.enable = true;
services.rofi-hub.apps = [ "screen-time" "calendar" ];
```

New:

```nix
imports = [ inputs.rofi-apps.nixosModules.default ];

services.rofi-apps.calendar.enable = true;
services.rofi-apps.screen-time.enable = true;
services.rofi-apps.menu.enable = false; # optional
```

The old option still evaluates through a deprecated compatibility module and
prints:

```text
warning: services.rofi-hub is deprecated; use services.rofi-apps.<app>.enable
```

Home Manager has the same mapping under `homeManagerModules.rofi-hub`.

## 2. Remove the old hub/menu from PATH

Apps no longer need `rofi-hub` or `rofi-hub-menu` to run. You can remove the
old binary and `~/.local/share/rofi-hub` at any time:

```bash
rm -f ~/.local/bin/rofi-hub
rm -rf ~/.local/share/rofi-hub
```

The menu and installer are not in any app's dependency closure.

## 3. Install per-app releases

With Nix, enable the app modules. Without Nix:

```bash
base=https://github.com/WillGle/rofi-hub/releases/latest/download
curl -fsSLO https://raw.githubusercontent.com/WillGle/rofi-hub/main/minisign.pub
curl -fsSLO "$base/install.sh" "$base/install.sh.minisig"
minisign -Vm install.sh -p minisign.pub
bash install.sh --app calendar
```

The new installer:

- keeps old binaries, configs, and databases;
- installs state under `~/.local/state/rofi-apps/`, not the old hub state;
- reads the legacy provider directory during the transition;
- does not write themes, env variables, or `PATH` wrappers;
- leaves `~/.config/rofi-hub` untouched. Apps no longer read it.

Provider discovery order:

```text
$XDG_DATA_HOME/rofi-apps/providers    # new
$XDG_DATA_HOME/rofi-hub/providers     # legacy, read-only transition
$XDG_DATA_DIRS/rofi-apps/providers
$XDG_DATA_DIRS/rofi-hub/providers
```

## 4. Move theme ownership to the user

The old `~/.config/rofi-hub/theme` and `ROFI_HUB_THEME*` variables are ignored
by apps. Use one of:

- `~/.config/rofi/theme.rasi` for the base palette/font;
- `~/.config/rofi/<app>.rasi` for one app's full override;
- `ROFI_THEME=/path/to/theme.rasi` for a full global override;
- `share/rofi/<app>.rasi` from the app package as the fallback layout.

The menu keeps its own theme at `~/.config/rofi-hub-menu/theme`.

Calendar, Clipboard, Keybinds, Network, and Atomic Tasks keep their `Ctrl+1–4`
theme lock in `~/.local/state/rofi-apps/<app>/theme-choice`; `Ctrl+4` removes
it. Screen Time also keeps its own `theme-choice` state and its own
`~/.config/screen-time/config.toml`; it no longer writes hub state.

## 5. Update scripts and keybinds

Old:

```text
rofi-hub calendar
rofi-hub clipboard
rofi-hub network
```

New:

```text
rofi-calendar
rofi-clipboard
rofi-keybinds
rofi-network
atomic-note
rofi-screen-time
```

The optional menu still provides a launcher:

```text
rofi-hub-menu list
rofi-hub-menu calendar
```

`rofi-hub` launcher aliases (`launcher`, `window`, `calc`, `emoji`, `power`)
now live in `rofi-hub-menu`.

## 6. Roll back

Nix rollback is the normal generation switch. Installer state is per app, so
you can remove one app without affecting another:

```bash
bash install.sh --uninstall calendar
```

Old hub files remain until you delete them explicitly. If you need the old
hub back, use an old release; new releases no longer publish distro packages.

## Versions

| App | Version |
|---|---|
| calendar | 1.2.0 |
| clipboard | 0.8.0 |
| keybinds | 1.0.0 |
| network | 0.3.0 |
| atomic-note | 0.1.0 |
| screen-time | 0.5.0 |
| rofi-dmenu | 0.3.0 |
| rofi-hub-menu | 0.3.0 |
| installer | 0.3.0 |

The `screen-time-rofi` repository remains archived; this repository is its
successor for Screen Time.
