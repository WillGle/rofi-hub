# Packaging rofi-hub

> **Owner:** WillGle · **Baseline:** `v0.2.0` (`fc0e5aa`) · **Updated:** 2026-11-25

`rofi-hub` is a Rust workspace: a launcher, three Rofi apps, and the Screen
Time tracker. One all-in-one package is the primary target. Split packages are
supported for distros that want lean dependencies.

## Dependency matrix

| Component | Build | Runtime | Optional |
|---|---|---|---|
| `rofi-hub`, `rofi-calendar` | Rust 1.89+, C compiler, `make` | `rofi >= 2.0` | — |
| `rofi-keybinds` | Rust 1.89+ | `rofi`, Hyprland config | Hyprland |
| `rofi-clipboard` | Rust 1.89+ | `rofi`, `cliphist`, `wl-clipboard` | Wayland |
| `screen-time` | Rust 1.89+, C compiler, `make` | `rofi`, Hyprland, systemd user, `tzdata` | systemd, Hyprland |
| `rofi-screen-time` | — | `bash`, `rofi`, `screen-time` | — |
| installer (`install.sh`) | `minisign` | `rofi`, systemd user | — |

SQLite is bundled through `rusqlite`; no system SQLite package is required.
`cliphist`, `wl-clipboard`, and Hyprland are optional because only the relevant
apps need them. Missing tools produce a clear error, never a panic.

## Install layout (`PREFIX=/usr`)

| Path | Content |
|---|---|
| `/usr/bin/rofi-hub` | hub |
| `/usr/bin/rofi-calendar` | calendar |
| `/usr/bin/rofi-keybinds` | keybinds |
| `/usr/bin/rofi-clipboard` | clipboard |
| `/usr/bin/screen-time` | tracker |
| `/usr/bin/rofi-screen-time` | shell wrapper |
| `/usr/share/rofi-hub/providers/*.toml` | provider manifests |
| `/usr/share/rofi-hub/themes/hub.rasi` | fallback hub theme |
| `/usr/share/rofi/screen-time.rasi` | Screen Time theme |
| `/usr/share/screen-time/config.example.toml` | config example |
| `/usr/lib/systemd/user/screen-time.service` | systemd user unit |
| `/usr/share/applications/rofi-hub.desktop` | desktop entry |
| `/usr/share/icons/hicolor/scalable/apps/rofi-hub.svg` | icon |
| `/usr/share/metainfo/rofi-hub.metainfo.xml` | AppStream |
| `/usr/share/man/man1/*.1.gz` | man pages |
| `/usr/share/bash-completion/completions/*` | bash completions |
| `/usr/share/zsh/site-functions/_rofi-hub`, `_screen-time` | zsh completions |
| `/usr/share/fish/vendor_completions.d/*.fish` | fish completions |
| `/usr/share/licenses/rofi-hub/LICENSE` | license |
| `/usr/share/doc/rofi-hub/*.md` | README, CHANGELOG, ARCHITECTURE, MIGRATING, PACKAGING |
| `/usr/share/rofi-hub/contrib/openrc/screen-time` | OpenRC service (contrib) |
| `/usr/share/rofi-hub/contrib/runit/screen-time/run` | runit service (contrib) |

The exact list and modes live in `packaging/MANIFEST`. `packaging/install-tree.sh`
is the only install source for package managers:

```bash
DESTDIR=/tmp/root PREFIX=/usr packaging/install-tree.sh --all
DESTDIR=/tmp/root PREFIX=/usr packaging/install-tree.sh --without apps
DESTDIR=/tmp/root PREFIX=/usr packaging/install-tree.sh --without screen-time
packaging/install-tree.sh --dry-run    # print `MODE PATH` records
```

Overridable variables: `PREFIX`, `DESTDIR`, `BINDIR`, `DATADIR`, `SYSCONFDIR`,
`SYSTEMD_USERUNITDIR`, `MANDIR`, `LICENSEDIR`, `DOCDIR`, `DESKTOPDIR`,
`ICONDIR`, `APPDATADIR`, `BASHCOMPLETIONSDIR`, `ZSHCOMPLETIONSDIR`,
`FISHCOMPLETIONSDIR`, `CONTRIBDIR`, `TARGETDIR`.

The script refuses to run if a selected binary or asset is missing, checks the
`DESTDIR` root before writing, never touches `$HOME`, and never calls
`systemctl`. The systemd unit is installed from
`packaging/systemd/screen-time.service.in` with `@BINDIR@` replaced. The
existing `install.sh` keeps its own `sed` substitution for user-local installs.

## Split packages

| Package | Content | Main dependencies |
|---|---|---|
| `rofi-hub` | hub, calendar, keybinds, clipboard, providers | `rofi` |
| `rofi-hub-screen-time` | screen-time, wrapper, provider, theme, unit | `rofi-hub` (or standalone), systemd/Hyprland optional |
| `rofi-hub-bin` (AUR) | binary release tarball | `rofi` |
| `rofi-hub-git` (AUR) | build from git | `rofi` |

Binary names never change, so existing Hyprland/Waybar bindings keep working.
`-bin` and `-git` declare `provides`/`conflicts` against `rofi-hub`.

## Service managers

**systemd user** is the primary integration. The unit starts
`@BINDIR@/screen-time daemon`, uses `StateDirectory`/`RuntimeDirectory`, and
only makes `%S/screen-time` writable under `ProtectSystem=strict`.

```bash
systemctl --user daemon-reload
systemctl --user enable --now screen-time.service
systemd-analyze verify /usr/lib/systemd/user/screen-time.service
```

**OpenRC** and **runit** are shipped as contrib files under
`/usr/share/rofi-hub/contrib/`. They must run as the desktop user, not root.
Distros without systemd can still install the hub and apps; Screen Time is
optional. See `packaging/openrc/screen-time` and
`packaging/runit/screen-time/run`.

## Provider and theme resolution

Provider manifests are searched in this order (first id wins):

1. `$ROFI_HUB_PROVIDER_DIR` (colon-separated)
2. `$XDG_DATA_HOME/rofi-hub/providers`
3. each `$XDG_DATA_DIRS/rofi-hub/providers`
4. `/usr/share/rofi-hub/providers`

`XDG_DATA_DIRS` defaults to `/usr/local/share:/usr/share`; `/usr/share` is
appended when a session overrides the list without it, so a distro package is
always visible.

Rofi themes for apps (`rofi-core`):

```text
ROFI_THEME
  > ROFI_HUB_THEME_FILE
  > $XDG_CONFIG_HOME/rofi/<app>.rasi
  > $XDG_DATA_DIRS/rofi/<app>.rasi
  > /usr/share/rofi/<app>.rasi
```

The hub menu in `auto` mode uses `ROFI_THEME`, then
`$XDG_CONFIG_HOME/rofi/theme.rasi`, then the installed
`/usr/share/rofi-hub/themes/hub.rasi`, then a generated runtime theme. Direct
launches of calendar, keybinds, and clipboard use the same installed theme (and
the saved mode), so all apps share Screen Time's flat chrome.

`rofi-screen-time`:

```text
ROFI_THEME
  > SCREEN_TIME_THEME
  > $XDG_DATA_HOME/screen-time/rofi/screen-time.rasi
  > $XDG_DATA_DIRS/screen-time/rofi/screen-time.rasi
  > $XDG_DATA_DIRS/rofi/screen-time.rasi
  > /usr/share/screen-time/rofi/screen-time.rasi
  > /usr/share/rofi/screen-time.rasi
```

## Release binary baseline

Release tarballs are built inside `debian:12`, whose glibc is **2.36**. The
current release binaries require at most `GLIBC_2.34` (verified with
`objdump -T`), so they run on Debian 12 and newer without a newer-glibc host.
Distro packages build from source and use their own toolchain; they do not
depend on this baseline. A musl static build is not the default: `zbus` and
bundled SQLite must be validated on musl first, and the flake still builds with
nixpkgs' glibc.

Verify a binary before publishing:

```bash
ldd --version | head -n1                 # inside the build container
objdump -T target/release/rofi-hub | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -n1
```

## Release artifacts

`scripts/build-package-artifacts.sh` stages the install tree, then calls
`nfpm` to produce:

- `rofi-hub_<version>_amd64.deb`
- `rofi-hub-<version>-1.x86_64.rpm`
- `rofi-hub-<version>-r0.apk`

Checksums are written to `dist/SHA256SUMS`; release signatures use the existing
minisign key. Validate with:

```bash
scripts/test-package-artifacts.sh dist/*.deb dist/*.rpm dist/*.apk
dpkg-deb --info dist/*.deb && dpkg -c dist/*.deb
rpm -qip dist/*.rpm && rpm -qlp dist/*.rpm
tar -tzf dist/*.apk
```

## Per-distro commands

### Arch / AUR

```bash
cd packaging/arch
makepkg -si --noconfirm
namcap PKGBUILD
namcap rofi-hub-*.pkg.tar.zst
makepkg --printsrcinfo > .SRCINFO
```

### Debian / Ubuntu

Release artifact:

```bash
sudo dpkg -i rofi-hub_0.2.0_amd64.deb
sudo apt-get -f install
sudo apt-get remove rofi-hub
```

Official track (skeleton in `packaging/debian/`): `dpkg-buildpackage -us -uc`,
then `lintian` and `sbuild`. A Debian Rust sponsor is required; this track does
not block release artifacts.

### Fedora

```bash
cd packaging/fedora
rpmlint rofi-hub.spec
fedpkg --release rawhide mockbuild
```

### Alpine

```bash
cd packaging/alpine
abuild -r
apkbuild-lint APKBUILD
```

### Gentoo (GURU)

```bash
cd packaging/gentoo
pkgcheck scan
ebuild rofi-hub-0.2.0.ebuild manifest
```

### openSUSE

```bash
cd packaging/opensuse
rpmlint rofi-hub.spec
osc build
```

### Void, Slackware, ALT

See `packaging/void/template`, `packaging/slackbuilds/`, and
`packaging/alt/rofi-hub.spec`.

## Packaging status

| Target | Version | Status | Maintainer | Validation |
|---|---|---|---|---|
| Arch/AUR source | 0.2.0 | ✅ validated; AUR push pending | WillGle | `makepkg`/`namcap`/`pacman -U`/`-Rns` |
| Arch/AUR `-bin` | 0.2.0 | ✅ validated; AUR push pending | WillGle | `makepkg`/`namcap`/`pacman -U`/`-Rns` |
| Arch/AUR `-git` | 0.2.0 | ⏳ recipe; needs AUR push | WillGle | `makepkg` |
| Debian/Ubuntu `.deb` | 0.2.0 | ✅ artifact validated | WillGle | `dpkg-deb`/`dpkg -i` |
| Debian official | 0.2.0 | ⏳ skeleton | WillGle | `sbuild`/`lintian` |
| Fedora `.spec` | 0.2.0 | ✅ validated | WillGle | `rpmlint`/`rpmbuild`/`dnf install`/`remove` |
| Alpine `APKBUILD` | 0.2.0 | ⏳ recipe | WillGle | `abuild` |
| Gentoo GURU | 0.2.0 | ⏳ recipe | WillGle | `pkgcheck` |
| openSUSE | 0.2.0 | ⏳ recipe | WillGle | `osc`/`rpmlint` |
| Void | 0.2.0 | ⏳ recipe | WillGle | `xbps-src` |
| SlackBuilds | 0.2.0 | ⏳ recipe | WillGle | `sbopkg` |
| ALT Sisyphus | 0.2.0 | ⏳ recipe | WillGle | `rpmlint` |
| Nix flake | 0.2.0 | ✅ | WillGle | `nix flake check`/`nix build`/smoke |
| nixpkgs | — | ⏸ deferred | — | — |
| Flatpak/Snap/Scoop | — | ❌ N/A | — | — |

The Repology badge is added once at least one package outside the flake is
published:

```markdown
[![Packaging status](https://repology.org/badge/vertical-allrepos/rofi-hub.svg)](https://repology.org/project/rofi-hub/versions)
```

## Release checklist

- [ ] bump workspace and Screen Time versions and update `CHANGELOG.md`
- [ ] `cargo test --workspace --locked`, `scripts/test-installer.sh`,
      `scripts/test-install-tree.sh`
- [ ] tag `vX.Y.Z`, sign and publish the release
- [ ] update AUR `pkgver` and hashes
- [ ] update Debian/Fedora/Alpine/Gentoo/openSUSE recipes
- [ ] run `python3 scripts/check-packaging-sync.py`
- [ ] update this status table and the README badge/table
- [ ] check Repology after the first outside package

## Removing a package

Package managers remove only files listed in the package database. User data
under `~/.config/screen-time`, `~/.local/state/screen-time`, and
`~/.local/state/rofi-hub` is never touched. Upgrades and rollbacks use the
installer's verified backups; package managers use their normal downgrade path.
