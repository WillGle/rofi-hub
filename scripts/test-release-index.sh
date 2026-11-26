#!/usr/bin/env bash
# Validate the release index, per-app tarball payloads, and signatures.
#
# Usage: scripts/test-release-index.sh [DIST]
# If DIST is omitted, package-release.sh builds a fresh one.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-release-index.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT INT TERM
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); printf 'ok   %s\n' "$1"; }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %s\n' "$1" >&2; }

assert() {
    local condition="$1" label="$2"
    if ((condition)); then pass "$label"; else fail "$label"; fi
}

DIST="${1:-}"
if [[ -z "$DIST" ]]; then
    DIST="$WORK/dist"
    DIST="$DIST" "$ROOT/scripts/package-release.sh" >/dev/null
fi
[[ -f "$DIST/index.json" ]] || { printf 'test-release-index: missing %s/index.json\n' "$DIST" >&2; exit 1; }

command -v minisign >/dev/null 2>&1 || { printf 'test-release-index: minisign is required\n' >&2; exit 1; }

python3 - "$ROOT" "$DIST" <<'PY'
import hashlib
import json
import pathlib
import sys
import tarfile
import tomllib

root = pathlib.Path(sys.argv[1])
dist = pathlib.Path(sys.argv[2])
index = json.loads((dist / "index.json").read_text())
errors = []

def check(condition, message):
    if not condition:
        errors.append(message)

check(index.get("schema") == 1, "index schema must be 1")
check(isinstance(index.get("installer", {}).get("version"), str), "installer.version missing")
apps = index.get("apps")
check(isinstance(apps, list) and apps, "apps must be a non-empty array")
ids = set()
for entry in apps or []:
    app_id = entry.get("id")
    check(app_id not in ids, f"duplicate app id {app_id}")
    ids.add(app_id)
    crate = {
        "calendar": "rofi-calendar",
        "clipboard": "rofi-clipboard",
        "keybinds": "rofi-keybinds",
        "network": "rofi-network",
        "screen-time": "screen-time",
        "hub-menu": "rofi-hub-menu",
    }.get(app_id)
    check(crate is not None, f"unexpected app id {app_id}")
    if crate is None:
        continue
    metadata = tomllib.loads((root / "crates" / crate / "app.toml").read_text())
    check(entry.get("version") == metadata["version"], f"{app_id}: index version mismatch")
    archive = dist / entry["url"]
    check(archive.is_file(), f"{app_id}: missing tarball {entry['url']}")
    if not archive.is_file():
        continue
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    check(entry.get("sha256") == digest, f"{app_id}: sha256 mismatch")
    check(entry.get("requires", []) == metadata.get("requires", []), f"{app_id}: requires mismatch")

    with tarfile.open(archive, "r:gz") as tar:
        names = {member.name.removeprefix("./") for member in tar.getmembers() if member.isfile()}
        for required in ("app.toml", f"bin/{metadata['bin']}"):
            check(required in names, f"{app_id}: tarball missing {required}")
        if metadata.get("provider"):
            provider_name = f"share/rofi-apps/providers/{app_id}.toml"
            check(provider_name in names, f"{app_id}: tarball missing provider")
            source = root / "crates" / crate / metadata["provider"]
            provider = tomllib.loads(source.read_text())
            check(provider.get("version") == metadata["version"], f"{app_id}: provider version mismatch")
            check(provider.get("exec", [])[:1] == [metadata["bin"]], f"{app_id}: provider exec mismatch")
        check(
            f"share/rofi/{app_id}.rasi" in names,
            f"{app_id}: tarball missing layout fallback",
        )
        check(
            not any(name.endswith((".deb", ".rpm", ".apk")) for name in names),
            f"{app_id}: distro artifact in payload",
        )

if errors:
    print("release-index: FAILED", file=sys.stderr)
    for error in errors:
        print(f"  - {error}", file=sys.stderr)
    raise SystemExit(1)
print(f"release-index: {len(apps)} app payloads valid")
PY
pass "index schema, versions, checksums, and payloads"

# Signed index/tarball verification and tamper rejection.
minisign -G -W -p "$WORK/pub" -s "$WORK/sec" >/dev/null 2>&1
for file in "$DIST"/index.json "$DIST"/*.tar.gz; do
    minisign -Sm "$file" -s "$WORK/sec" -x "$file.minisig" >/dev/null
done
if minisign -Vm "$DIST/index.json" -p "$WORK/pub" -x "$DIST/index.json.minisig" >/dev/null 2>&1; then
    pass "signed index verifies"
else
    fail "signed index verifies"
fi

cp "$DIST/index.json" "$WORK/index-good.json"
printf '\n' >>"$DIST/index.json"
if minisign -Vm "$DIST/index.json" -p "$WORK/pub" -x "$DIST/index.json.minisig" >/dev/null 2>&1; then
    fail "tampered index is rejected"
else
    pass "tampered index is rejected"
fi
cp "$WORK/index-good.json" "$DIST/index.json"

# Installer must reject an unsigned index by default.
TMP_HOME="$WORK/home"
mkdir -p "$TMP_HOME" "$WORK/bin" "$WORK/state" "$WORK/config" "$WORK/data"
printf '#!/bin/sh\nexit 1\n' >"$WORK/bin/rofi"
chmod 755 "$WORK/bin/rofi"
rm -f "$DIST/index.json.minisig"
if HOME="$TMP_HOME" XDG_STATE_HOME="$WORK/state" XDG_CONFIG_HOME="$WORK/config" \
    XDG_DATA_HOME="$WORK/data" PATH="$WORK/bin:$PATH" \
    "$ROOT/install.sh" --index "$DIST/index.json" --app calendar >/dev/null 2>&1; then
    fail "unsigned index is rejected"
else
    pass "unsigned index is rejected"
fi

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
