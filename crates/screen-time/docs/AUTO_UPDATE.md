# Data safety and updates

Screen Time keeps local history on your machine. This page describes the two
supported installation channels, how updates work, and how to protect the
database when removing or moving the app.

## Channels

There is exactly one owner for each installation:

| Channel | Update mechanism | Notes |
|---|---|---|
| Nix / Home Manager | `nix flake update rofi-hub`, then rebuild | The flake input and lock choose the version. The per-app module installs the package and its service. |
| `install.sh` | `install.sh --update screen-time` | Per-app signed release. The installer verifies the index, archive checksum, and minisign signature before replacing files. |
| Flatpak (future) | A separate packaging project | A window tracker needs host/Hyprland access; this is not part of the current release. |

Do not mix channels for the same binary or unit. The standalone installer
refuses Nix/Home-Manager-owned paths and never adds automatic update units.
`install.sh --theme` and `install.sh --nixos` do not exist.

## What an update owns

The app package owns:

- `~/.local/bin/screen-time` and `~/.local/bin/rofi-screen-time`;
- `~/.config/systemd/user/screen-time.service` (unless `--no-service`);
- the provider manifest and the fallback Rofi layout under
  `~/.local/share/rofi-apps/` and `~/.local/share/rofi/`;
- the per-app state file `~/.local/state/rofi-apps/installed/screen-time`.

The app never touches the live database when installing or updating files. The
database, config, and migration backups stay under
`${XDG_STATE_HOME:-~/.local/state}/screen-time/` and
`${XDG_CONFIG_HOME:-~/.config}/screen-time/`.

Every snapshot and manifest is created with `0700` directories and `0600`
files; the app sets `umask 077` rather than relying on yours.

## Update and uninstall

```bash
# One app only. Other apps are not touched.
bash install.sh --update screen-time
bash install.sh --uninstall screen-time

# Install without enabling the user service.
bash install.sh --app screen-time --no-service

# List index entries and the installed version.
bash install.sh --list
```

`install.sh --update screen-time` is a no-op when the installed version matches
the index, and it refuses to install an archive without a valid signature or
checksum. Updates do not restart the daemon automatically in automatic mode;
the service is enabled with `systemctl --user enable --now screen-time.service`
unless `--no-service` is passed.

The installer keeps one state file per app and restores the previous files if
an install or update fails. Uninstall removes only files recorded in that app's
state; the database, config, and migration backups are left for the user.

There is no installer-owned update timer. If you want notifications, use your
system scheduler or your Nix configuration; the app does not download or run
code in the background.

## Release verification

Releases are signed with [minisign](https://jedisct1.github.io/minisign/). The
public key is pinned in `minisign.pub`. Release assets include:

- per-app tarballs for `x86_64` and `aarch64`;
- one `index.json` that maps each app to both architectures;
- `install.sh`, `uninstall.sh`, and `minisign.pub`;
- `SHA256SUMS` and `.minisig` files next to the index and each tarball.

Release tags should be signed and CI builds from the tag. The installer
downloads the index and its detached signature, verifies them against the
pinned key, selects the tarball for `uname -m`, then verifies the archive
checksum and signature. `--allow-unsigned` exists only for a local tarball or
offline test.

```bash
minisign -Vm install.sh -p minisign.pub
```

## Database safety

The database schema migrates automatically at startup. Before a migration the
app writes a snapshot under
`${XDG_STATE_HOME:-~/.local/state}/screen-time/backups/migrations/`. Migration
backups are never pruned automatically; after verifying a migration, remove old
copies manually:

```bash
ls -lh ~/.local/state/screen-time/backups/migrations/
rm ~/.local/state/screen-time/backups/migrations/pre-v1-YYYYMMDD-HHMMSS.db
```

`screen-time doctor` checks the database, service state, and configuration. Run
it after an update and before deleting any backup.
