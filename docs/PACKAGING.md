# Packaging

> **Owner:** WillGle · **Baseline:** `v0.2.0` (`fc0e5aa`) · **Current:** `v0.2.1` · **Updated:** 2026-11-25

`rofi-hub` is primarily a **Nix product**: flake package, standalone NixOS
module, and Home Manager module. `install.sh` is the secondary path for non-Nix
distros. The Debian `.deb` and Fedora `.rpm` are additional release artifacts;
other recipes are experimental.

## Dependency matrix

| Component | Build | Runtime | Optional |
|---|---|---|---|
| `rofi-hub`, apps | Rust 1.89+, C compiler | `rofi >= 2.0` | — |
| `rofi-keybinds` | — | `rofi`, Hyprland config | Hyprland |
| `rofi-clipboard` | — | `rofi`, `cliphist`, `wl-clipboard` | Wayland |
| `screen-time` | Rust, C compiler, `tzdata` for tests | `rofi`, Hyprland, systemd user, `tzdata` | systemd, Hyprland |
| `install.sh` | `minisign` | `rofi`, systemd user | — |

SQLite is bundled by `rusqlite`; no system SQLite package is needed.

## Install layout

| Path | Content |
|---|---|
| `/usr/bin/` | `rofi-hub`, `rofi-calendar`, `rofi-keybinds`, `rofi-clipboard`, `screen-time`, `rofi-screen-time` |
| `/usr/share/rofi-hub/providers/` | provider manifests |
| `/usr/share/rofi-hub/themes/hub.rasi` | generated hub theme |
| `/usr/share/rofi/screen-time.rasi` | Screen Time theme |
| `/usr/share/screen-time/config.example.toml` | config example |
| `/usr/lib/systemd/user/screen-time.service` | user unit |
| `/usr/share/{applications,icons,metainfo,man,doc,licenses}/` | desktop integration |
| `/usr/share/rofi-hub/contrib/{openrc,runit}/` | non-systemd examples |

```bash
DESTDIR=/tmp/root PREFIX=/usr packaging/install-tree.sh --all
DESTDIR=/tmp/root PREFIX=/usr packaging/install-tree.sh --without apps
packaging/install-tree.sh --dry-run
```

Overrides: `PREFIX`, `DESTDIR`, `BINDIR`, `DATADIR`, `SYSTEMD_USERUNITDIR`,
`MANDIR`, `LICENSEDIR`, `DOCDIR`, `DESKTOPDIR`, `ICONDIR`, `APPDATADIR`,
`BASHCOMPLETIONSDIR`, `ZSHCOMPLETIONSDIR`, `FISHCOMPLETIONSDIR`, `CONTRIBDIR`.

## Split packages

| Package | Content | Dependency |
|---|---|---|
| `rofi-hub` | hub, calendar, keybinds, clipboard, providers | `rofi` |
| `rofi-hub-screen-time` | tracker, wrapper, provider, theme, unit | `rofi-hub` |
| `rofi-hub-bin` / `-git` (AUR) | binary / git build | `rofi` |

## Service managers

- **systemd user:** `screen-time.service` runs `@BINDIR@/screen-time daemon`,
  with `StateDirectory`/`RuntimeDirectory` and `ProtectSystem=strict`.
- **OpenRC/runit:** examples in `/usr/share/rofi-hub/contrib/`; run as the
  desktop user.
- Verify: `systemd-analyze verify /usr/lib/systemd/user/screen-time.service`.

## Provider and theme resolution

```text
Providers: ROFI_HUB_PROVIDER_DIR
  > $XDG_DATA_HOME/rofi-hub/providers
  > $XDG_DATA_DIRS/rofi-hub/providers
  > /usr/share/rofi-hub/providers

Apps: ROFI_THEME
  > ROFI_HUB_THEME_FILE
  > $XDG_CONFIG_HOME/rofi/<app>.rasi
  > $XDG_DATA_DIRS/rofi/<app>.rasi
  > generated theme

Screen Time: ROFI_THEME
  > ROFI_HUB_THEME_FILE / ROFI_HUB_THEME
  > $XDG_DATA_HOME/screen-time/rofi/screen-time.rasi
  > $XDG_DATA_DIRS/screen-time/rofi/screen-time.rasi
  > /usr/share/rofi/screen-time.rasi
```

`auto` resolves `matugen > static.env > Gruvbox`. `ROFI_THEME` remains an
explicit escape hatch.

## Release binary baseline

Release tarballs build in `debian:12` (glibc 2.36); current binaries require at
most `GLIBC_2.34`. The musl path is validated in `alpine:edge` with
`cargo test --workspace` and `test-install-tree.sh`; Alpine/distro packages build
from source. Nix uses nixpkgs' glibc.

```bash
ldd --version | head -n1
objdump -T target/release/rofi-hub | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -1
```

## Release artifacts

A `v0.2.x` tag publishes:

- `rofi-hub-<arch>-linux.tar.gz` + `install.sh`/`uninstall.sh`;
- `rofi-hub_<version>_<arch>.deb`, `rofi-hub-<version>-1.<arch>.rpm`;
- experimental `.apk` and source asset for community recipes;
- signed `SHA256SUMS`.

`.deb`/`.rpm` build in Debian 12. The published `.apk` comes from `abuild`, not
`nfpm`. Validate:

```bash
scripts/test-package-artifacts.sh dist/*.deb dist/*.rpm dist/*.apk
scripts/test-bundle.sh dist/rofi-hub-x86_64-linux.tar.gz
minisign -Vm dist/SHA256SUMS -p minisign.pub
```

## Per-distro commands

**Supported**

```bash
# Debian / Ubuntu
sudo dpkg -i rofi-hub_<version>_amd64.deb && sudo apt-get remove rofi-hub

# Fedora
rpmlint packaging/fedora/rofi-hub.spec
sudo dnf install ./rofi-hub-<version>-1.x86_64.rpm

# Nix
nix flake check && nix build .#
```

**Experimental**

```bash
cd packaging/arch && makepkg -si --noconfirm && namcap PKGBUILD
cd packaging/alpine && abuild -r && apkbuild-lint APKBUILD
cd packaging/opensuse && rpmlint rofi-hub.spec
cd packaging/gentoo && pkgcheck scan
cd packaging/void && xbps-src pkg rofi-hub
cd packaging/slackbuilds && ./rofi-hub.SlackBuild
cd packaging/alt && rpmlint rofi-hub.spec
```

## Packaging status

| Target | Version | Status | Validation |
|---|---|---|---|
| Nix flake / NixOS / Home Manager | 0.2.1 | ✅ primary | `nix flake check`/`nix build` |
| `install.sh` tarball (non-Nix) | 0.2.1 | ✅ secondary | bundle/upgrade/rollback |
| Debian/Ubuntu `.deb` | 0.2.1 | ➕ additional artifact | `dpkg-deb`/`dpkg -i` |
| Fedora `.rpm` / spec | 0.2.1 | ➕ additional artifact | `rpmlint`/`dnf` |
| nixpkgs | — | ⏸ deferred | — |
| Debian official | 0.2.1 | 🧪 experimental | `dpkg-buildpackage`/`lintian` |
| Arch/AUR | 0.2.1 | 🧪 experimental | `makepkg`/`namcap` |
| Alpine `.apk` | 0.2.1 | 🧪 experimental | `abuild`/`apk add` |
| openSUSE/Gentoo/Void/SlackBuilds/ALT | 0.2.1 | 🧪 experimental | distro lint/build |

AUR is not a supported release gate. Publication needs an AUR SSH key.
Repology has no supported repository package yet: https://repology.org/project/rofi-hub/versions

## Release checklist

- [ ] bump version and `CHANGELOG.md`
- [ ] `cargo test --workspace --locked`, `test-installer.sh`, `test-install-tree.sh`
- [ ] `package-source.sh` + `update-source-checksums.py`
- [ ] build and test artifacts
- [ ] tag `vX.Y.Z`; sign and publish
- [ ] update experimental recipes only if shipping them
- [ ] update status table/README
- [ ] check Repology after a repository package exists
