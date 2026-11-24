#!/usr/bin/env bash
# Build .deb, .rpm, and .apk release artifacts from the install tree.
#
# Requires nfpm (https://nfpm.goreleaser.com). The staged tree is produced by
# packaging/install-tree.sh, so every artifact contains exactly the files in
# packaging/MANIFEST.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VERSION="${VERSION:-$(sed -n '/^\[workspace.package\]/,/^\[/p' Cargo.toml | sed -n 's/^version = "\(.*\)"/\1/p' | head -n1)}"
[ -n "$VERSION" ] || { printf 'build-package-artifacts: cannot read workspace version\n' >&2; exit 1; }

MACHINE="${ARCH:-$(uname -m)}"
case "$MACHINE" in
    x86_64|amd64)
        DEB_ARCH=amd64
        NFPM_ARCH=amd64
        RPM_ARCH=x86_64
        ;;
    aarch64|arm64)
        DEB_ARCH=arm64
        NFPM_ARCH=arm64
        RPM_ARCH=aarch64
        ;;
    *)
        printf 'build-package-artifacts: unsupported architecture: %s\n' "$MACHINE" >&2
        exit 1
        ;;
esac

DIST="${DIST:-$ROOT/dist}"
STAGE="${STAGE:-$DIST/stage}"
NFPM="${NFPM:-nfpm}"
command -v "$NFPM" >/dev/null 2>&1 || {
    printf 'build-package-artifacts: nfpm is required (set NFPM=/path/to/nfpm)\n' >&2
    exit 127
}

if [ "${SKIP_BUILD:-0}" -ne 1 ]; then
    if [ ! -x target/release/rofi-hub ] || [ ! -x target/release/screen-time ]; then
        cargo build --release --locked --workspace
    fi
fi

mkdir -p "$DIST"
rm -rf "$STAGE"
DESTDIR="$STAGE" PREFIX=/usr packaging/install-tree.sh --all

DEB="$DIST/rofi-hub_${VERSION}_${DEB_ARCH}.deb"
RPM="$DIST/rofi-hub-${VERSION}-1.${RPM_ARCH}.rpm"
APK="$DIST/rofi-hub-${VERSION}-r0.apk"

ARCH="$NFPM_ARCH" VERSION="$VERSION" RELEASE=1 STAGE="$STAGE" \
    "$NFPM" package -f packaging/nfpm.yaml -p deb -t "$DEB"
ARCH="$NFPM_ARCH" VERSION="$VERSION" RELEASE=1 STAGE="$STAGE" \
    "$NFPM" package -f packaging/nfpm.yaml -p rpm -t "$RPM"
ARCH="$NFPM_ARCH" VERSION="$VERSION" RELEASE=0 STAGE="$STAGE" \
    "$NFPM" package -f packaging/nfpm.yaml -p apk -t "$APK"

(
    cd "$DIST"
    sha256sum "$(basename "$DEB")" "$(basename "$RPM")" "$(basename "$APK")" > SHA256SUMS.packages
    cat SHA256SUMS.packages
)

printf 'build-package-artifacts: wrote %s, %s, %s\n' "$DEB" "$RPM" "$APK"
