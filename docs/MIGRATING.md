# Migrating from screen-time-rofi

Screen Time is now the first app in the `rofi-hub` workspace. The tracker code,
database schema, Rofi timeline, and theme behavior are preserved. What changed
is where the code lives and which installer owns updates.

## What moved

| Before | After |
| --- | --- |
| repository root | `crates/screen-time/` |
| `install.sh` for one app | root `install.sh` with component selection |
| `flake.nix` for one package | workspace flake with hub and app packages |
| `homeManagerModules.default` | `homeManagerModules.default` for the hub; `homeManagerModules.screen-time` for the tracker |
| release asset `screen-time-<arch>-linux.tar.gz` | `rofi-hub-<arch>-linux.tar.gz` |

Installed runtime paths did not move:

- `~/.local/bin/screen-time`
- `~/.local/bin/rofi-screen-time`
- `~/.local/state/screen-time/screen-time.db`
- `~/.config/systemd/user/screen-time.service`
- `~/.local/share/screen-time/`

## Existing installs

Run the new installer as your desktop user:

```bash
base=https://github.com/WillGle/rofi-hub/releases/latest/download
curl -fsSLO https://raw.githubusercontent.com/WillGle/rofi-hub/main/minisign.pub
curl -fsSLO "$base/install.sh" "$base/install.sh.minisig"
minisign -Vm install.sh -p minisign.pub
bash install.sh --app screen-time
```

The installer:

- keeps the existing database, config, and theme;
- snapshots the database before replacing binaries;
- takes over from the legacy `screen-time-update-*` timers (they are disabled
  and removed, and only the new `rofi-hub-update-*` units are installed);
- records the installed components so later updates stay the same shape.

Legacy update state under `~/.local/state/screen-time/updates` is left in place
for reference but is no longer used. Rollback and pruning use
`~/.local/state/rofi-hub/backups`.

## Home Manager

The old import path changes:

```nix
# before
imports = [ inputs.screen-time-rofi.homeManagerModules.default ];
services.screen-time.enable = true;

# after: hub + tracker
imports = [ inputs.rofi-hub.homeManagerModules.default ];
services.rofi-hub.enable = true;
services.rofi-hub.apps = [ "screen-time" ];

# after: tracker only
imports = [ inputs.rofi-hub.homeManagerModules.screen-time ];
services.screen-time.enable = true;
```

If you used a custom `package`, keep passing it through
`services.rofi-hub.package` (which must contain `rofi-hub` and, when selected,
`screen-time`) or through `services.screen-time.package`.

## Themes

Screen Time still reads `~/.config/screen-time/config.toml` and still supports
`Alt+1..4` in its own popup. The hub adds its own theme state at
`~/.config/rofi-hub/theme` and honors `Alt+1..4` in the launcher menu.
