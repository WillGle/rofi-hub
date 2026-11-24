#!/usr/bin/env bash
# Build the source archive used by the Arch/Alpine/Gentoo recipes.
#
# The GitHub tag archive does not contain packaging/ for releases that
# predate the packaging work. This archive is the release asset recipe URLs
# point at: a complete, taggable tree with Cargo.lock and packaging/.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VERSION="${VERSION:-$(sed -n '/^\[workspace.package\]/,/^\[/p' Cargo.toml | sed -n 's/^version = "\(.*\)"/\1/p' | head -n1)}"
[ -n "$VERSION" ] || { printf 'package-source: cannot read workspace version\n' >&2; exit 1; }
DIST="${DIST:-$ROOT/dist}"
NAME="rofi-hub-$VERSION"
ASSET="$DIST/$NAME-source.tar.gz"
EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct 2>/dev/null || date +%s)}"

mkdir -p "$DIST"
tmp="$DIST/.$NAME-source.tar.gz.$$"
trap 'rm -f -- "$tmp"' EXIT INT TERM

# Keep the tree taggable and reproducible: no build output, no local state,
# no git metadata, no user goal file. packaging/ install assets are included;
# distro recipe directories are excluded because their checksums describe
# this archive, which would otherwise be self-referential.
tar --sort=name --mtime="@$EPOCH" --owner=0 --group=0 --numeric-owner \
    --exclude='./target' --exclude='./dist' --exclude='./result' \
    --exclude='./.git' --exclude='./.direnv' --exclude='./state' \
    --exclude='./goal.md' \
    --exclude='./packaging/arch' --exclude='./packaging/alpine' \
    --exclude='./packaging/gentoo' --exclude='./packaging/void' \
    --exclude='./packaging/slackbuilds' --exclude='./packaging/fedora' \
    --exclude='./packaging/opensuse' --exclude='./packaging/alt' \
    --exclude='./packaging/debian' \
    --transform="s,^\./,${NAME}/," --transform="s,^\.,${NAME}," \
    -cf - . | gzip -9n >"$tmp"
mv -f -- "$tmp" "$ASSET"
trap - EXIT INT TERM

case "$ASSET" in
    "$ROOT"/*) display="${ASSET#"$ROOT"/}" ;;
    *) display="$ASSET" ;;
esac
printf 'source archive: %s\n' "$display"
printf 'sha256: %s\n' "$(sha256sum "$ASSET" | awk '{print $1}')"
printf 'sha512: %s\n' "$(sha512sum "$ASSET" | awk '{print $1}')"
tar -tzf "$ASSET" | sed -n '1,5p'
