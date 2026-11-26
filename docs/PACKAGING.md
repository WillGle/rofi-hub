# Packaging and releases

Only two delivery channels exist:

1. **Nix** — flake packages and modules (primary).
2. **`install.sh`** — signed universal release for other distributions
   (secondary).

There are no distro packages: no `.deb`, `.rpm`, `.apk`, AUR, Debian official,
Gentoo, openSUSE, Void, Slackware, ALT, or source asset for community recipes.

## Release layout

`scripts/package-release.sh` builds a `dist/` directory:

```text
dist/
  index.json
  rofi-calendar-1.2.0-x86_64.tar.gz
  rofi-clipboard-0.8.0-x86_64.tar.gz
  rofi-keybinds-1.0.0-x86_64.tar.gz
  rofi-network-0.3.0-x86_64.tar.gz
  screen-time-0.5.0-x86_64.tar.gz
  rofi-hub-menu-0.3.0-x86_64.tar.gz   # optional app
  SHA256SUMS
  install.sh
  uninstall.sh
  minisign.pub
```

`index.json` follows `installer/index.schema.json`:

```json
{
  "schema": 1,
  "installer": { "version": "0.3.0" },
  "apps": [
    {
      "id": "calendar",
      "version": "1.2.0",
      "url": "rofi-calendar-1.2.0-x86_64.tar.gz",
      "sha256": "...",
      "requires": ["rofi"]
    }
  ]
}
```

Each tarball contains one app payload:

```text
app.toml
bin/<binary>
share/rofi/<id>.rasi
share/rofi-apps/providers/<id>.toml
share/rofi-apps/apps/<id>/app.toml
```

Screen Time adds `bin/rofi-screen-time`, `systemd/screen-time.service`, and
`share/screen-time/config.example.toml`. The service unit is shipped by the
app, not by the installer or the menu.

## Signing

Use the offline minisign key:

```bash
MINISIGN_SECRET_KEY=/secure/minisign.key scripts/sign-release.sh dist
```

This signs `index.json`, every `*.tar.gz`, and `SHA256SUMS`. The installer
verifies `index.json.minisig` and `<tarball>.minisig` by default. Unsigned
artifacts are rejected unless `--allow-unsigned` is passed for a local test.

## Install paths

```text
~/.local/bin/<binary>
~/.local/share/rofi/<id>.rasi
~/.local/share/rofi-apps/providers/<id>.toml
~/.local/share/rofi-apps/apps/<id>/app.toml
~/.config/systemd/user/<id>.service     # only when app.toml service = true
~/.local/state/rofi-apps/installed/<id>
~/.local/state/rofi-apps/version
```

`--prefix` changes the binary prefix. State, data, and config stay in the XDG
directories. `--no-service` installs files without installing or enabling a
unit.

## Installer guarantees

- Reads `index.json` and `app.toml`; there is no hardcoded app list.
- `--list`, `--app`, `--all`, `--update`, `--uninstall`, `--no-service`.
- Prerequisite check is `command -v`; nothing is installed automatically.
- Checksum and signature verification are mandatory by default.
- Preflight checks every selected app before changing any file.
- A failed install restores the previous files.
- Per-app state; update/uninstall one app without touching another.
- No theme/config writes, no `ROFI_HUB_*`, no `PATH` wrapper.
- Rejects Nix-managed paths.
- Rejects absolute paths, `..` components, links, and devices in tarballs.
- State directory 0700, state files 0600.

## Nix packages

```bash
nix build .#rofi-calendar
nix build .#rofi-clipboard
nix build .#rofi-keybinds
nix build .#rofi-network
nix build .#screen-time
nix build .#rofi-hub-menu
nix build .#rofi-hub-installer
```

Each app package installs only that app. Runtime dependencies are wrapped on
the app's own `PATH`; the package does not carry the menu, installer, or
another app. `packages.rofi-hub-apps` is a compatibility bundle only.

aarch64 artifacts are built natively on the aarch64 CI runner
(`ubuntu-24.04-arm`). The flake also exposes cross-built packages on x86_64
(`nix build .#rofi-calendar-aarch64`), which is useful for local verification;
these pull the target GTK/Rofi stack and can take a while. A direct cross
`cargo build --target aarch64-unknown-linux-gnu` is enough to check the Rust
binaries without rebuilding the GUI stack. The release path itself always
builds natively on the arm64 runner.

## Verification

```bash
scripts/test-release-index.sh dist   # schema, versions, checksums, payloads
scripts/test-bundle.sh dist          # install.sh end-to-end
scripts/test-installer.sh            # install/update/failure injection
scripts/test-nix.sh                  # per-app closures and modules
scripts/test-install-matrix.sh       # Debian/Ubuntu/Arch/Fedora containers
scripts/test-performance.sh          # app startup and theme-cache budget
```

Tag builds require all checks, build x86_64 and aarch64 artifacts, sign the
index and tarballs, verify them, and only then publish the GitHub release.
