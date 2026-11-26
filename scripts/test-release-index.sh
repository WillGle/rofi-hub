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
import re
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

check(index.get("schema") == 2, "index schema must be 2")
check(isinstance(index.get("installer", {}).get("version"), str), "installer.version missing")
architectures = index.get("architectures")
check(isinstance(architectures, list) and architectures, "architectures missing")
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

    match = re.search(r"-(x86_64|aarch64)\.tar\.gz$", entry["url"])
    check(match is not None, f"{app_id}: tarball URL has no architecture")
    if match:
        arch = match.group(1)
        check(arch in architectures, f"{app_id}: {arch} missing from architectures")
        check(entry.get(f"url_{arch}") == entry["url"], f"{app_id}: url_{arch} mismatch")
        check(entry.get(f"sha256_{arch}") == digest, f"{app_id}: sha256_{arch} mismatch")

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

# Two native bundles merge into one index without losing either architecture.
MERGE="$WORK/merge"
mkdir -p "$MERGE"
python3 - "$DIST/index.json" "$MERGE/index-x86_64.json" "$MERGE/index-aarch64.json" "$MERGE" <<'PY'
import hashlib
import json
import pathlib
import re
import shutil
import sys

source = pathlib.Path(sys.argv[1])
index = json.loads(source.read_text())
targets = {
    "x86_64": pathlib.Path(sys.argv[2]),
    "aarch64": pathlib.Path(sys.argv[3]),
}
out = pathlib.Path(sys.argv[4])
ARCH = re.compile(r"-(x86_64|aarch64)\.tar\.gz$")
indexes = {
    arch: {"schema": 2, "architectures": [arch], "installer": index["installer"], "apps": []}
    for arch in targets
}
for entry in index["apps"]:
    source_url = entry["url"]
    match = ARCH.search(source_url)
    if not match:
        raise SystemExit(f"cannot find architecture in {source_url}")
    for arch, path in targets.items():
        url = ARCH.sub(f"-{arch}.tar.gz", source_url)
        target_archive = out / url
        if not target_archive.exists():
            shutil.copy2(source.parent / source_url, target_archive)
        digest = hashlib.sha256(target_archive.read_bytes()).hexdigest()
        target = dict(entry)
        target["url"] = url
        target["sha256"] = digest
        target[f"url_{arch}"] = url
        target[f"sha256_{arch}"] = digest
        for key in [key for key in target if key.startswith(("url_", "sha256_")) and not key.endswith(arch)]:
            target.pop(key)
        indexes[arch]["apps"].append(target)
for arch, path in targets.items():
    path.write_text(json.dumps(indexes[arch], indent=2, sort_keys=True) + "\n")
PY
if "$ROOT/scripts/merge-release-index.py" "$MERGE/index.json" \
    "$MERGE/index-x86_64.json" "$MERGE/index-aarch64.json" >/dev/null; then
    pass "two native indexes merge"
else
    fail "two native indexes merge"
fi
if python3 - "$MERGE/index.json" <<'PY'
import json
import pathlib
import sys

index = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert index["schema"] == 2
assert index["architectures"] == ["aarch64", "x86_64"]
for entry in index["apps"]:
    assert entry["url_x86_64"].endswith("-x86_64.tar.gz"), entry
    assert entry["url_aarch64"].endswith("-aarch64.tar.gz"), entry
    assert entry["url_x86_64"] != entry["url_aarch64"], entry
PY
then
    pass "merged index keeps both architectures"
else
    fail "merged index keeps both architectures"
fi

# The installer must pick the tarball for the host architecture.
TMP_HOME2="$WORK/home-arch"
mkdir -p "$TMP_HOME2" "$WORK/state-arch" "$WORK/config-arch" "$WORK/data-arch" "$WORK/run-arch"
if HOME="$TMP_HOME2" XDG_STATE_HOME="$WORK/state-arch" XDG_CONFIG_HOME="$WORK/config-arch" \
    XDG_DATA_HOME="$WORK/data-arch" XDG_RUNTIME_DIR="$WORK/run-arch" \
    PATH="$WORK/bin:$PATH" ROFI_APPS_ARCH=aarch64 \
    "$ROOT/install.sh" --index "$MERGE/index.json" --allow-unsigned \
    --dry-run --verbose --app calendar >"$WORK/arch.log" 2>&1; then
    pass "installer accepts the aarch64 mapping"
else
    fail "installer accepts the aarch64 mapping"
fi
if grep -q -- "-aarch64.tar.gz" "$WORK/arch.log" &&
    ! grep -q -- "-x86_64.tar.gz" "$WORK/arch.log"; then
    pass "installer selects the aarch64 tarball"
else
    fail "installer selects the aarch64 tarball"
fi
if HOME="$TMP_HOME2" XDG_STATE_HOME="$WORK/state-arch" XDG_CONFIG_HOME="$WORK/config-arch" \
    XDG_DATA_HOME="$WORK/data-arch" XDG_RUNTIME_DIR="$WORK/run-arch" \
    PATH="$WORK/bin:$PATH" ROFI_APPS_ARCH=riscv64 \
    "$ROOT/install.sh" --index "$MERGE/index.json" --allow-unsigned \
    --dry-run --app calendar >"$WORK/arch-bad.log" 2>&1; then
    fail "installer rejects a host with no artifact"
else
    pass "installer rejects a host with no artifact"
fi
if grep -q "no artifact for riscv64" "$WORK/arch-bad.log"; then
    pass "missing-architecture error is clear"
else
    fail "missing-architecture error is clear"
fi

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
