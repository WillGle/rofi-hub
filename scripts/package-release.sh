#!/usr/bin/env bash
# Build the signed-release-shaped dist directory: per-app tarballs, a release
# index, and checksums. No distro packages.
#
# Each tarball contains exactly one app payload:
#   app.toml
#   bin/<binary>
#   share/rofi/<id>.rasi
#   share/rofi-apps/providers/<id>.toml
#   share/rofi-apps/apps/<id>/app.toml
#   systemd/<id>.service and extra data files for service apps
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALLER_VERSION="${INSTALLER_VERSION:-0.3.0}"
ARCH="${ARCH:-$(uname -m)}"
case "$ARCH" in
    x86_64 | amd64) ARCH=x86_64 ;;
    aarch64 | arm64) ARCH=aarch64 ;;
    *) printf 'unsupported architecture: %s\n' "$ARCH" >&2; exit 1 ;;
esac

DIST="${DIST:-$ROOT/dist}"
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/rofi-apps-release.XXXXXX")"
trap 'rm -rf -- "$STAGE"' EXIT INT TERM

# app_id:crate:bin
APPS=(
    "calendar:rofi-calendar:rofi-calendar"
    "clipboard:rofi-clipboard:rofi-clipboard"
    "keybinds:rofi-keybinds:rofi-keybinds"
    "network:rofi-network:rofi-network"
    "atomic-note:atomic-note:atomic-note"
    "screen-time:screen-time:screen-time"
    "hub-menu:rofi-hub-menu:rofi-hub-menu"
)

cd "$ROOT"
cargo build --release --locked \
    --package rofi-calendar \
    --package rofi-clipboard \
    --package rofi-keybinds \
    --package rofi-network \
    --package screen-time \
    --package rofi-hub-menu

rm -rf -- "$DIST"
mkdir -p "$DIST"

for spec in "${APPS[@]}"; do
    IFS=: read -r id crate bin <<<"$spec"
    version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$ROOT/crates/$crate/app.toml")"
    [ -n "$version" ] || { printf 'cannot read version for %s\n' "$crate" >&2; exit 1; }

    app_stage="$STAGE/$id"
    mkdir -p "$app_stage/bin" "$app_stage/share/rofi" \
        "$app_stage/share/rofi-apps/providers" "$app_stage/share/rofi-apps/apps/$id"
    executable="$ROOT/target/release/$bin"
    if [ "$id" = "atomic-note" ]; then
        executable="$ROOT/crates/$crate/share/$bin"
    fi
    install -m 755 "$executable" "$app_stage/bin/$bin"
    install -m 644 "$ROOT/crates/$crate/app.toml" "$app_stage/app.toml"
    install -m 644 "$ROOT/crates/$crate/app.toml" "$app_stage/share/rofi-apps/apps/$id/app.toml"
    install -m 644 "$ROOT/crates/$crate/share/rofi/$id.rasi" \
        "$app_stage/share/rofi/$id.rasi"

    if [ -f "$ROOT/crates/$crate/providers/$id.toml" ]; then
        install -m 644 "$ROOT/crates/$crate/providers/$id.toml" \
            "$app_stage/share/rofi-apps/providers/$id.toml"
    fi

    if [ "$id" = "screen-time" ]; then
        install -m 755 "$ROOT/crates/screen-time/share/rofi-screen-time" \
            "$app_stage/bin/rofi-screen-time"
        install -Dm644 "$ROOT/crates/screen-time/systemd/screen-time.service" \
            "$app_stage/systemd/screen-time.service"
        install -Dm644 "$ROOT/crates/screen-time/share/screen-time/config.example.toml" \
            "$app_stage/share/screen-time/config.example.toml"
    fi

    archive="$DIST/${bin}-${version}-${ARCH}.tar.gz"
    tar -czf "$archive" -C "$app_stage" .
    printf 'built %s\n' "$(basename -- "$archive")"
done

python3 - "$ROOT" "$DIST" "$ARCH" "$INSTALLER_VERSION" <<'PY'
import hashlib
import json
import pathlib
import sys
import tomllib

root = pathlib.Path(sys.argv[1])
dist = pathlib.Path(sys.argv[2])
arch = sys.argv[3]
installer_version = sys.argv[4]
specs = [
    ("calendar", "rofi-calendar", False),
    ("clipboard", "rofi-clipboard", False),
    ("keybinds", "rofi-keybinds", False),
    ("network", "rofi-network", False),
    ("atomic-note", "atomic-note", False),
    ("screen-time", "screen-time", False),
    ("hub-menu", "rofi-hub-menu", True),
]
apps = []
for app_id, crate, optional in specs:
    metadata = tomllib.loads((root / "crates" / crate / "app.toml").read_text())
    version = metadata["version"]
    name = f"{crate}-{version}-{arch}.tar.gz"
    path = dist / name
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    entry = {
        "id": app_id,
        "version": version,
        "url": name,
        "sha256": digest,
        "requires": metadata.get("requires", []),
        f"url_{arch}": name,
        f"sha256_{arch}": digest,
    }
    if optional:
        entry["optional"] = True
    apps.append(entry)

index = {
    "schema": 2,
    "architectures": [arch],
    "installer": {"version": installer_version},
    "apps": apps,
}
text = json.dumps(index, indent=2, sort_keys=True) + "\n"
(dist / "index.json").write_text(text)
(dist / f"index-{arch}.json").write_text(text)
print(f"wrote {dist / 'index.json'} ({len(apps)} apps, {arch})")
PY

install -m 755 "$ROOT/install.sh" "$DIST/install.sh"
install -m 755 "$ROOT/uninstall.sh" "$DIST/uninstall.sh"
install -m 644 "$ROOT/minisign.pub" "$DIST/minisign.pub"
(
    cd "$DIST"
    sha256sum ./*.tar.gz index.json "index-${ARCH}.json" install.sh uninstall.sh minisign.pub >SHA256SUMS
)

printf 'release assembled in %s\n' "$DIST"
printf 'sign with: scripts/sign-release.sh %s\n' "$DIST"
