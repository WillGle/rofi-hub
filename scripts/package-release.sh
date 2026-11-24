#!/usr/bin/env bash
# Build a GitHub-release-shaped bundle for the host architecture.
#
# The bundle contains the hub and every app. install.sh chooses which
# components to install, so one download serves hub-only, app-only, and
# all-in-one installs.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${VERSION:-$(sed -n '/^\[workspace.package\]/,/^\[/p' "${ROOT}/Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -n1)}"
ARCH="${ARCH:-$(uname -m)}"
case "$ARCH" in
    x86_64|amd64) ARCH=x86_64 ;;
    aarch64|arm64) ARCH=aarch64 ;;
    *) printf 'unsupported architecture: %s\n' "$ARCH" >&2; exit 1 ;;
esac
[ -n "$VERSION" ] || { printf 'cannot read workspace version\n' >&2; exit 1; }

cd "$ROOT"
cargo build --release --locked --package rofi-hub --package screen-time

STAGE="$(mktemp -d "${TMPDIR:-/tmp}/rofi-hub-release.XXXXXX")"
trap 'rm -rf -- "$STAGE"' EXIT INT TERM

mkdir -p "$STAGE/bin" "$STAGE/share/rofi-hub/providers" "$STAGE/share/rofi-hub/themes" \
    "$STAGE/share/rofi" "$STAGE/share/screen-time" "$STAGE/systemd" "$ROOT/dist"
install -m 755 target/release/rofi-hub "$STAGE/bin/rofi-hub"
install -m 755 target/release/screen-time "$STAGE/bin/screen-time"
install -m 755 crates/screen-time/share/rofi-screen-time "$STAGE/bin/rofi-screen-time"
install -m 644 themes/hub.rasi "$STAGE/share/rofi-hub/themes/hub.rasi"
install -m 644 providers/screen-time.toml "$STAGE/share/rofi-hub/providers/screen-time.toml"
install -m 644 crates/screen-time/share/rofi/screen-time.rasi "$STAGE/share/rofi/screen-time.rasi"
install -m 644 crates/screen-time/share/screen-time/config.example.toml "$STAGE/share/screen-time/config.example.toml"
install -m 644 crates/screen-time/systemd/screen-time.service "$STAGE/systemd/screen-time.service"
printf '%s\n' "$VERSION" >"$STAGE/VERSION"
install -m 644 LICENSE README.md CHANGELOG.md "$STAGE/"

ASSET="${RELEASE_ASSET:-rofi-hub}-${ARCH}-linux.tar.gz"
tar -czf "$ROOT/dist/$ASSET" -C "$STAGE" .
install -m 755 install.sh "$ROOT/dist/install.sh"
install -m 755 uninstall.sh "$ROOT/dist/uninstall.sh"
install -m 644 minisign.pub "$ROOT/dist/minisign.pub"
(
    cd "$ROOT/dist"
    sha256sum "$ASSET" install.sh uninstall.sh minisign.pub > SHA256SUMS
)
printf '%s\n' "$ROOT/dist/$ASSET"
