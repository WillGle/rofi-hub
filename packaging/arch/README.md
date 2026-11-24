# Arch / AUR

Three recipes share the same install tree:

| Directory | Package | Source |
|---|---|---|
| `.` | `rofi-hub` | tagged source tarball, `cargo build --release` |
| `rofi-hub-bin` | `rofi-hub-bin` | GitHub release tarball, `packaging/install-tree.sh` from the tarball |
| `rofi-hub-git` | `rofi-hub-git` | live git checkout |

## Validated locally

Inside `archlinux:base-devel`:

```bash
# source package
cd packaging/arch
makepkg -f --skipchecksums --noconfirm
namcap PKGBUILD
namcap rofi-hub-0.2.0-1-x86_64.pkg.tar.zst
pacman -U --noconfirm rofi-hub-0.2.0-1-x86_64.pkg.tar.zst
rofi-hub list --json
pacman -Rns --noconfirm rofi-hub

# binary package (using a locally built release tarball named as in source_x86_64)
cd packaging/arch/rofi-hub-bin
makepkg -f --skipchecksums --noconfirm
namcap rofi-hub-bin-0.2.0-1-x86_64.pkg.tar.zst
pacman -U --noconfirm rofi-hub-bin-0.2.0-1-x86_64.pkg.tar.zst
pacman -Rns --noconfirm rofi-hub-bin
```

`namcap` reports warnings for the contrib OpenRC/runit scripts and implicit
glibc/libgcc dependencies; it reports no errors. The source recipe sets
`options=('!lto')` because makepkg's GCC LTO objects for bundled SQLite cannot
be consumed by rustc's lld link step.

## Before publishing to AUR

1. Tag a release that contains `packaging/` (the v0.2.0 tag predates it).
2. Run `updpkgsums` in `rofi-hub` and `rofi-hub-bin` to replace the source
   checksums (the `-bin` recipe currently uses `SKIP` until assets exist).
3. Regenerate metadata: `makepkg --printsrcinfo > .SRCINFO`.
4. Verify `pacman -U` and `pacman -Rns` in a clean container.
5. Push `PKGBUILD` and `.SRCINFO` to the three AUR repositories. This step
   needs AUR SSH credentials and is not automated in this repository.

`rofi-hub-git` gets its `pkgver` from `git describe`; it is intentionally not
pinned to the release tag.
