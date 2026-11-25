# Packaging

This directory is the source of truth for distro packages. Nothing here writes
to `$HOME`, calls `systemctl`, or runs `install.sh`: a package manager stages a
complete tree with `DESTDIR` and owns the files.

## Quick start

```bash
# Build the workspace once.
cargo build --release --locked --workspace

# Stage a package-shaped tree under /usr.
DESTDIR="$(mktemp -d)" PREFIX=/usr packaging/install-tree.sh --all
find "$DESTDIR" -type f | sort

# Compare the staged tree, modes, unit, desktop file, AppStream data, and
# runtime behavior against packaging/MANIFEST.
DESTDIR="$DESTDIR" scripts/test-install-tree.sh
```

`install-tree.sh` is the only install path used by every recipe in this
directory. The environment variables and flags are documented in
`docs/PACKAGING.md` and in `install-tree.sh --help`.

## Layout

```text
packaging/
├── MANIFEST                  exact file list and modes for --all
├── install-tree.sh           PREFIX/DESTDIR installer shared by all recipes
├── nfpm.yaml                 .deb/.rpm/.apk metadata
├── rofi-hub.desktop          desktop entry
├── rofi-hub.svg              hicolor icon
├── rofi-hub.metainfo.xml     AppStream metadata
├── systemd/                  systemd user unit template
├── openrc/                   OpenRC service (contrib, for non-systemd distros)
├── runit/                    runit service (contrib)
├── man/                      roff man pages
├── completions/              bash, zsh, and fish completions
├── arch/                     PKGBUILD (source, -bin, -git)
├── debian/                   Debian source-package skeleton
├── fedora/                   Fedora spec
├── alpine/                   APKBUILD
├── gentoo/                   GURU ebuild and metadata
├── opensuse/                 openSUSE spec
├── void/                     xbps-src template
├── slackbuilds/              SlackBuild, .info, slack-desc
└── alt/                      ALT Sisyphus spec
```

## Recipes

| Target | Recipe | Local build |
|---|---|---|
| Arch/AUR | `arch/PKGBUILD`, `arch/rofi-hub-bin/PKGBUILD`, `arch/rofi-hub-git/PKGBUILD` | `makepkg -si`, `namcap` |
| Debian/Ubuntu | `debian/` (official track), release `.deb` from `nfpm.yaml` | `dpkg-buildpackage`, `lintian` |
| Fedora | `fedora/rofi-hub.spec` | `fedpkg mockbuild`, `rpmlint` |
| Alpine | `alpine/APKBUILD` | `abuild -r`, `apkbuild-lint` |
| Gentoo | `gentoo/rofi-hub-0.2.0.ebuild` | `pkgcheck scan`, `ebuild ... manifest` |
| openSUSE | `opensuse/rofi-hub.spec` | `osc build`, `rpmlint` |
| Void | `void/template` | `xbps-src pkg rofi-hub` |
| Slackware | `slackbuilds/rofi-hub.SlackBuild` | `sbopkg` |
| ALT | `alt/rofi-hub.spec` | `rpmbuild`, `rpmlint` |

Release `.deb`, `.rpm`, and `.apk` artifacts are produced by
`scripts/build-package-artifacts.sh` from the same staged tree and checked by
`scripts/test-package-artifacts.sh`. The `.apk` that is published comes from
`abuild` in `alpine:edge`; `nfpm` produces the `.deb` and `.rpm`. A tag also
publishes `rofi-hub-<version>-source.tar.gz`, which the source recipes use, and
a signed `SHA256SUMS` covering every asset.

## Validation

```bash
scripts/test-install-tree.sh          # V2: layout, modes, unit, assets, runtime
python3 scripts/check-packaging-sync.py
scripts/build-package-artifacts.sh    # requires nfpm on PATH
scripts/test-package-artifacts.sh dist/*.deb dist/*.rpm dist/*.apk
```

The full matrix runs in `.github/workflows/release.yml` (`install-tree` on
x86_64 and aarch64, `distro-build`, `artifacts`, `apk-package`,
`release-smoke`). Pull requests skip the container matrix; main and tag builds
run it. See `docs/PACKAGING.md`.
