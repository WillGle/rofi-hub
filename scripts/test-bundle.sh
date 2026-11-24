#!/usr/bin/env bash
# Verify a release-shaped bundle (tarball or extracted directory).
#
# Checks the VERSION file, every provider manifest, provider binaries, the
# hub's `list --json` registry, and the Screen Time support files.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BUNDLE="${1:-}"
if [ -z "$BUNDLE" ]; then
    printf 'usage: %s <bundle.tar.gz|extracted-dir>\n' "$0" >&2
    exit 64
fi
[ -e "$BUNDLE" ] || { printf 'test-bundle.sh: not found: %s\n' "$BUNDLE" >&2; exit 66; }

WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-hub-bundle.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT INT TERM
if [ -d "$BUNDLE" ]; then
    cp -a -- "$BUNDLE/." "$WORK/"
else
    tar -xzf "$BUNDLE" -C "$WORK"
fi

VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/p' "$ROOT/Cargo.toml" \
    | sed -n 's/^version = "\(.*\)"/\1/p' | head -n1)"
[ -n "$VERSION" ] || { printf 'test-bundle.sh: cannot read workspace version\n' >&2; exit 1; }
ACTUAL="$(cat "$WORK/VERSION" 2>/dev/null || true)"
[ "$ACTUAL" = "$VERSION" ] || {
    printf 'test-bundle.sh: VERSION is %q, expected %q\n' "$ACTUAL" "$VERSION" >&2
    exit 1
}

python3 - "$WORK" "$ROOT" <<'PY'
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path

stage = Path(sys.argv[1])
root = Path(sys.argv[2])
providers_dir = stage / "share/rofi-hub/providers"
hub = stage / "bin/rofi-hub"

assert hub.is_file() and os.access(hub, os.X_OK), f"missing executable {hub}"
assert (stage / "share/rofi-hub/themes/hub.rasi").is_file(), "missing hub theme"
for name in ("LICENSE", "README.md", "CHANGELOG.md"):
    assert (stage / name).is_file(), f"missing {name}"
for name in (
    "docs/ARCHITECTURE.md",
    "docs/MIGRATING.md",
    "docs/PACKAGING.md",
    "packaging/install-tree.sh",
    "packaging/rofi-hub.desktop",
    "packaging/rofi-hub.svg",
    "packaging/rofi-hub.metainfo.xml",
    "packaging/systemd/screen-time.service.in",
    "packaging/openrc/screen-time",
    "packaging/runit/screen-time/run",
):
    assert (stage / name).is_file(), f"missing {name}"
assert os.access(stage / "packaging/install-tree.sh", os.X_OK), "install-tree.sh is not executable"
for name in (
    "rofi-hub.1",
    "rofi-calendar.1",
    "rofi-keybinds.1",
    "rofi-clipboard.1",
    "screen-time.1",
    "rofi-screen-time.1",
):
    assert (stage / "packaging/man" / name).is_file(), f"missing man page {name}"
for name in ("rofi-hub.bash", "_rofi-hub", "rofi-hub.fish", "screen-time.bash", "_screen-time", "screen-time.fish"):
    assert (stage / "packaging/completions" / name).is_file(), f"missing completion {name}"

bundle_providers = {}
for path in sorted(providers_dir.glob("*.toml")):
    data = tomllib.loads(path.read_text())
    provider_id = data["id"]
    assert path.stem == provider_id, f"{path} does not match id {provider_id}"
    assert provider_id not in bundle_providers, f"duplicate provider {provider_id}"
    program = data["exec"][0]
    binary = stage / "bin" / program
    assert binary.is_file(), f"provider {provider_id} missing binary {binary}"
    assert os.access(binary, os.X_OK), f"provider {provider_id} binary is not executable"
    bundle_providers[provider_id] = data

assert bundle_providers, "bundle has no providers"
repo_providers = {
    path.stem: tomllib.loads(path.read_text())
    for path in sorted((root / "providers").glob("*.toml"))
}
assert set(bundle_providers) == set(repo_providers), (
    f"bundle providers {sorted(bundle_providers)} do not match "
    f"repo providers {sorted(repo_providers)}"
)
for provider_id, data in repo_providers.items():
    bundled = bundle_providers[provider_id]
    assert bundled["version"] == data["version"], (
        f"{provider_id} version {bundled['version']} != repo {data['version']}"
    )
    assert bundled["exec"] == data["exec"], (
        f"{provider_id} exec {bundled['exec']} != repo {data['exec']}"
    )

env = os.environ.copy()
env["ROFI_HUB_PROVIDER_DIR"] = str(providers_dir)
listed = subprocess.run(
    [str(hub), "list", "--json"],
    capture_output=True,
    text=True,
    env=env,
)
assert listed.returncode == 0, f"rofi-hub list failed: {listed.stderr}"
registry = {item["id"]: item for item in json.loads(listed.stdout)}
assert set(registry) == set(bundle_providers), (
    f"registry {sorted(registry)} != providers {sorted(bundle_providers)}"
)
for provider_id, data in bundle_providers.items():
    item = registry[provider_id]
    assert item["exec"] == data["exec"]
    assert item["available"] is True, f"{provider_id} is not available: {item}"

version_line = subprocess.run(
    [str(hub), "--version"], capture_output=True, text=True, env=env
)
assert version_line.returncode == 0
assert version_line.stdout.strip() == f"rofi-hub {(stage / 'VERSION').read_text().strip()}"

if "screen-time" in bundle_providers:
    tracker = stage / "bin/screen-time"
    assert tracker.is_file() and os.access(tracker, os.X_OK), "missing screen-time"
    tracker_version = subprocess.run(
        [str(tracker), "--version"], capture_output=True, text=True, env=env
    )
    assert tracker_version.returncode == 0
    assert tracker_version.stdout.strip() == (
        f"screen-time {bundle_providers['screen-time']['version']}"
    )
    for relative in (
        "share/rofi/screen-time.rasi",
        "share/screen-time/config.example.toml",
        "systemd/screen-time.service",
    ):
        assert (stage / relative).is_file(), f"missing {relative}"

print(f"bundle ok: {len(bundle_providers)} providers ({', '.join(sorted(bundle_providers))})")
PY

# A release tarball must install itself through packaging/install-tree.sh. This
# is the path used by rofi-hub-bin packages, separate from install.sh.
INSTALL_ROOT="$WORK/install-root"
DESTDIR="$INSTALL_ROOT" PREFIX=/usr "$WORK/packaging/install-tree.sh" --all
find "$INSTALL_ROOT" -type f | sed "s|^${INSTALL_ROOT}||" | sort >"$WORK/installed-files"
awk 'NF && $1 !~ /^#/ {print $2}' "$WORK/packaging/MANIFEST" | sort >"$WORK/manifest-files"
diff -u "$WORK/manifest-files" "$WORK/installed-files" ||
    { printf 'test-bundle.sh: release install tree differs from MANIFEST\n' >&2; exit 1; }
ROFI_HUB_PROVIDER_DIR="$INSTALL_ROOT/usr/share/rofi-hub/providers" \
    "$INSTALL_ROOT/usr/bin/rofi-hub" list --json >"$WORK/installed-list.json"
python3 - "$WORK/installed-list.json" <<'PY'
import json
import sys

listed = {item["id"]: item for item in json.load(open(sys.argv[1], encoding="utf-8"))}
assert set(listed) == {"calendar", "clipboard", "keybinds", "screen-time"}, listed
assert all(item["available"] for item in listed.values()), listed
print("release install-tree ok:", ", ".join(sorted(listed)))
PY
"$INSTALL_ROOT/usr/bin/screen-time" --version >/dev/null
printf 'test-bundle.sh: release install tree ok\n'
