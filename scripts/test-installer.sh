#!/usr/bin/env bash
# Installer integration and fault-injection tests in a temporary HOME.
#
# Covers per-app install, update/uninstall independence, prerequisite failure,
# checksum/signature rejection, path traversal, state permissions, services,
# optional menu handling, and rollback.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-apps-installer.XXXXXX")"
KEEP="${KEEP_TEST_WORK:-0}"
cleanup() {
    if [[ "$KEEP" = 1 ]]; then
        printf 'test workdir kept at %s\n' "$WORK" >&2
    else
        rm -rf -- "$WORK"
    fi
}
trap cleanup EXIT INT TERM
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); printf 'ok   %s\n' "$1"; }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %s\n' "$1" >&2; }

assert_file() {
    if [[ -e "$1" ]]; then pass "$2"; else fail "$2 (missing $1)"; fi
}
assert_no_file() {
    if [[ ! -e "$1" ]]; then pass "$2"; else fail "$2 (unexpected $1)"; fi
}
assert_eq() {
    if [[ "$1" = "$2" ]]; then pass "$3"; else fail "$3 (expected '$1', got '$2')"; fi
}
assert_mode() {
    local expected="$1" path="$2" label="$3" actual
    actual="$(stat -c '%a' "$path" 2>/dev/null || printf missing)"
    assert_eq "$expected" "$actual" "$label"
}
assert_contains() {
    local needle="$1" file="$2" label="$3"
    if grep -qF -- "$needle" "$file" 2>/dev/null; then
        pass "$label"
    else
        fail "$label (missing '$needle' in $file)"
    fi
}
assert_success() {
    local label="$1"; shift
    local output
    if output="$("$@" 2>&1)"; then
        pass "$label"
    else
        fail "$label"
        printf '%s\n' "$output" | tail -n 12 >&2
    fi
}
assert_failure() {
    local label="$1"; shift
    if "$@" >/dev/null 2>&1; then fail "$label (unexpected success)"; else pass "$label"; fi
}

command -v minisign >/dev/null 2>&1 || { printf 'minisign is required\n' >&2; exit 1; }
command -v python3 >/dev/null 2>&1 || { printf 'python3 is required\n' >&2; exit 1; }

DIST="$WORK/dist"
DIST="$DIST" "$ROOT/scripts/package-release.sh" >/dev/null

# Test signing key. Production releases pin minisign.pub instead.
minisign -G -W -p "$WORK/pub" -s "$WORK/sec" >/dev/null 2>&1
for file in "$DIST"/index.json "$DIST"/*.tar.gz; do
    minisign -Sm "$file" -s "$WORK/sec" -x "$file.minisig" >/dev/null
done

mkdir -p "$WORK/tools"
for tool in python3 tar sha256sum awk cut find sed basename dirname readlink \
    mktemp cp mv rm mkdir chmod cat sort head minisign flock wc tr env bash gzip grep uname; do
    path="$(command -v "$tool" 2>/dev/null || true)"
    [[ -n "$path" ]] && ln -sf "$path" "$WORK/tools/$tool"
done

ENV_N=0
new_env() {
    ENV_N=$((ENV_N + 1))
    E="$WORK/env$ENV_N"
    mkdir -p "$E/home" "$E/bin" "$E/state" "$E/config" "$E/data" "$E/run"
    cat >"$E/bin/rofi" <<'EOF'
#!/bin/sh
printf '%s\n' "$@" >"${ROFI_LOG:-/dev/null}"
env | grep -E '^ROFI_HUB_' >"${ROFI_ENV_LOG:-/dev/null}" || true
exit "${ROFI_STATUS:-1}"
EOF
    for tool in cliphist wl-copy nmcli; do
        cat >"$E/bin/$tool" <<EOF
#!/bin/sh
exit 0
EOF
        chmod 755 "$E/bin/$tool"
    done
    cat >"$E/bin/systemctl" <<'EOF'
#!/bin/sh
printf '%s\n' "$*" >>"${SYSTEMCTL_LOG:-/dev/null}"
exit 0
EOF
    chmod 755 "$E/bin/rofi" "$E/bin/systemctl"
    export ROFI_LOG="$E/rofi.args"
    export ROFI_ENV_LOG="$E/rofi.env"
    export SYSTEMCTL_LOG="$E/systemctl.log"
}

in_env() {
    env HOME="$E/home" \
        XDG_STATE_HOME="$E/state" \
        XDG_CONFIG_HOME="$E/config" \
        XDG_DATA_HOME="$E/data" \
        XDG_RUNTIME_DIR="$E/run" \
        PATH="$E/bin:$WORK/tools" \
        ROFI_LOG="${ROFI_LOG:-}" \
        ROFI_ENV_LOG="${ROFI_ENV_LOG:-}" \
        SYSTEMCTL_LOG="${SYSTEMCTL_LOG:-}" \
        "$@"
}

install_app() {
    in_env "$ROOT/install.sh" --index "$DIST/index.json" --pubkey "$WORK/pub" "$@"
}

# ---------------------------------------------------------------------------
# INS-01, INS-12, INS-13, APP-01/02/06: one app, no hub, no dependencies
# ---------------------------------------------------------------------------
new_env
assert_success "INS-01 install calendar" install_app --app calendar
assert_file "$E/home/.local/bin/rofi-calendar" "INS-01 binary installed"
assert_file "$E/data/rofi-apps/providers/calendar.toml" "INS-01 provider installed"
assert_file "$E/data/rofi/calendar.rasi" "INS-01 layout fallback installed"
assert_mode 700 "$E/state/rofi-apps" "INS-13 state root is 0700"
assert_mode 600 "$E/state/rofi-apps/installed/calendar" "INS-13 state file is 0600"
assert_no_file "$E/home/.local/bin/rofi-hub" "INS-01 no hub binary"
assert_no_file "$E/home/.local/bin/rofi-hub-menu" "INS-01 no menu binary"
assert_no_file "$E/home/.local/bin/cliphist" "INS-12 no cliphist"
assert_no_file "$E/home/.local/bin/wl-copy" "INS-12 no wl-copy"
assert_no_file "$E/config/rofi-hub" "INS-01 no hub theme state"
assert_no_file "$E/config/rofi-hub-menu" "INS-01 no menu state"

# The app resolves its own theme and does not read hub variables.
assert_success "APP-01 calendar runs with fake rofi" in_env "$E/home/.local/bin/rofi-calendar"
assert_contains "-theme" "$E/rofi.args" "APP-01 app passes a resolved theme"
assert_contains "$E/run/rofi/theme-calendar.rasi" "$E/rofi.args" "APP-01 app-scoped runtime theme"
if [[ -s "$E/rofi.env" ]]; then
    fail "APP-06 app saw ROFI_HUB_* variables"
else
    pass "APP-06 app saw no ROFI_HUB_* variables"
fi

# Remove the optional hub/menu entirely; the app must still run.
rm -f "$E/home/.local/bin/rofi-hub" "$E/home/.local/bin/rofi-hub-menu"
rm -rf "$E/data/rofi-hub"
assert_success "APP-02 calendar still runs without hub" in_env "$E/home/.local/bin/rofi-calendar"

# ---------------------------------------------------------------------------
# INS-10 / INS-11: unknown app and invalid schema
# ---------------------------------------------------------------------------
new_env
assert_failure "INS-10 unknown app is rejected" install_app --app unknown-app
assert_no_file "$E/home/.local/bin/unknown-app" "INS-10 nothing installed"

# The installer must refuse Nix-managed paths instead of corrupting a profile.
new_env
if env HOME="$E/home" \
    XDG_STATE_HOME="$E/state" XDG_CONFIG_HOME="$E/config" \
    XDG_DATA_HOME="$E/data" PATH="$E/bin:$WORK/tools" \
    "$ROOT/install.sh" --index "$DIST/index.json" --pubkey "$WORK/pub" \
    --prefix /nix/store/rofi-apps-refusal-test --app calendar >/dev/null 2>&1; then
    fail "refuses Nix-managed prefix"
else
    pass "refuses Nix-managed prefix"
fi
assert_no_file "$E/data/rofi-apps/providers/calendar.toml" "Nix-refusal installed no files"

BAD="$WORK/bad-schema"
cp -a "$DIST" "$BAD"
python3 - "$BAD/index.json" <<'PY'
import json, sys
path = sys.argv[1]
data = json.load(open(path))
entry = data["apps"][0]
for key in ("url", "sha256"):
    entry.pop(key, None)
for key in [key for key in entry if key.startswith(("url_", "sha256_"))]:
    entry.pop(key)
json.dump(data, open(path, "w"))
PY
minisign -Sm "$BAD/index.json" -s "$WORK/sec" -x "$BAD/index.json.minisig" >/dev/null
assert_failure "INS-11 invalid index schema is rejected" \
    in_env "$ROOT/install.sh" --index "$BAD/index.json" --pubkey "$WORK/pub" --app calendar

# ---------------------------------------------------------------------------
# INS-04: missing prerequisite aborts before any install
# ---------------------------------------------------------------------------
NO_ROFI="$WORK/tools-no-rofi"
mkdir -p "$NO_ROFI"
for tool in python3 tar sha256sum awk cut find sed basename dirname readlink \
    mktemp cp mv rm mkdir chmod cat sort head minisign flock wc tr env bash gzip grep uname; do
    path="$(command -v "$tool" 2>/dev/null || true)"
    [[ -n "$path" ]] && ln -sf "$path" "$NO_ROFI/$tool"
done
new_env
assert_failure "INS-04 missing rofi aborts" \
    env HOME="$E/home" XDG_STATE_HOME="$E/state" XDG_CONFIG_HOME="$E/config" \
        XDG_DATA_HOME="$E/data" PATH="$NO_ROFI" \
        "$ROOT/install.sh" --index "$DIST/index.json" --pubkey "$WORK/pub" --app calendar
assert_no_file "$E/home/.local/bin/rofi-calendar" "INS-04 no partial install"

# ---------------------------------------------------------------------------
# INS-05 / SEC-01: checksum and unsigned rejection
# ---------------------------------------------------------------------------
BAD2="$WORK/bad-checksum"
cp -a "$DIST" "$BAD2"
CALENDAR_ARCHIVE="$(python3 - "$DIST/index.json" <<'PY'
import json, sys
index = json.load(open(sys.argv[1]))
for app in index["apps"]:
    if app["id"] == "calendar":
        for key, value in app.items():
            if key.startswith("url"):
                print(value)
                raise SystemExit
PY
)"
printf '\0' >>"$BAD2/$CALENDAR_ARCHIVE"
assert_failure "INS-05 bad checksum is rejected" \
    in_env "$ROOT/install.sh" --index "$BAD2/index.json" --pubkey "$WORK/pub" --app calendar

new_env
UNSIGNED="$WORK/unsigned"
cp -a "$DIST" "$UNSIGNED"
rm -f "$UNSIGNED/index.json.minisig"
assert_failure "SEC-01 unsigned index is rejected" \
    in_env "$ROOT/install.sh" --index "$UNSIGNED/index.json" --app calendar
assert_success "INS-01 --allow-unsigned permits local testing" \
    in_env "$ROOT/install.sh" --index "$UNSIGNED/index.json" --allow-unsigned --app calendar

# ---------------------------------------------------------------------------
# SEC-02: path traversal
# ---------------------------------------------------------------------------
new_env
EVIL="$WORK/evil"
mkdir -p "$EVIL/stage/bin"
cp "$ROOT/crates/rofi-calendar/app.toml" "$EVIL/stage/app.toml"
printf '#!/bin/sh\nexit 0\n' >"$EVIL/stage/bin/rofi-calendar"
python3 - "$EVIL/stage" "$EVIL/evil.tar.gz" <<'PY'
import io, sys, tarfile
from pathlib import Path
stage = Path(sys.argv[1])
out = sys.argv[2]
with tarfile.open(out, "w:gz") as tar:
    for path in sorted(stage.rglob("*")):
        tar.add(path, arcname=str(path.relative_to(stage)))
    data = b"escape\n"
    info = tarfile.TarInfo("../escape")
    info.size = len(data)
    tar.addfile(info, io.BytesIO(data))
PY
sha="$(sha256sum "$EVIL/evil.tar.gz" | awk '{print $1}')"
python3 - "$DIST/index.json" "$EVIL/index.json" "$sha" <<'PY'
import json, sys
index = json.load(open(sys.argv[1]))
for app in index["apps"]:
    if app["id"] == "calendar":
        app["url"] = "evil.tar.gz"
        app["sha256"] = sys.argv[3]
        for key in [key for key in app if key.startswith("url_")]:
            app[key] = "evil.tar.gz"
        for key in [key for key in app if key.startswith("sha256_")]:
            app[key] = sys.argv[3]
json.dump(index, open(sys.argv[2], "w"))
PY
# Build a dedicated index directory so relative URLs resolve to the evil tar.
EVIL_DIR="$WORK/evil-index"
mkdir -p "$EVIL_DIR"
cp "$EVIL/evil.tar.gz" "$EVIL_DIR/evil.tar.gz"
cp "$EVIL/index.json" "$EVIL_DIR/index.json"
minisign -Sm "$EVIL_DIR/index.json" -s "$WORK/sec" -x "$EVIL_DIR/index.json.minisig" >/dev/null
minisign -Sm "$EVIL_DIR/evil.tar.gz" -s "$WORK/sec" -x "$EVIL_DIR/evil.tar.gz.minisig" >/dev/null
assert_failure "SEC-02 path traversal is rejected" \
    in_env "$ROOT/install.sh" --index "$EVIL_DIR/index.json" --pubkey "$WORK/pub" --app calendar
assert_no_file "$E/escape" "SEC-02 no escaped file"

# ---------------------------------------------------------------------------
# INS-08/09: update and uninstall are per-app
# ---------------------------------------------------------------------------
new_env
assert_success "install calendar + clipboard" install_app --app calendar,clipboard
clipboard_state_before="$(sha256sum "$E/state/rofi-apps/installed/clipboard" | awk '{print $1}')"
assert_success "INS-08 update calendar only" install_app --update calendar
clipboard_state_after="$(sha256sum "$E/state/rofi-apps/installed/clipboard" | awk '{print $1}')"
assert_eq "$clipboard_state_before" "$clipboard_state_after" "INS-08 clipboard state unchanged"
assert_file "$E/home/.local/bin/rofi-clipboard" "INS-08 clipboard binary still present"

assert_success "INS-09 uninstall calendar" install_app --uninstall calendar
assert_no_file "$E/home/.local/bin/rofi-calendar" "INS-09 calendar removed"
assert_no_file "$E/state/rofi-apps/installed/calendar" "INS-09 calendar state removed"
assert_file "$E/home/.local/bin/rofi-clipboard" "INS-09 clipboard still present"
assert_file "$E/state/rofi-apps/installed/clipboard" "INS-09 clipboard state intact"

# ---------------------------------------------------------------------------
# INS-07: failed update restores the previous install
# ---------------------------------------------------------------------------
new_env
assert_success "rollback: initial install" install_app --app calendar
if in_env env ROFI_APPS_INJECT_FAILURE=after-bin \
    "$ROOT/install.sh" --index "$DIST/index.json" --pubkey "$WORK/pub" --update calendar >/dev/null 2>&1; then
    fail "INS-07 injected failure aborts update"
else
    pass "INS-07 injected failure aborts update"
fi
assert_file "$E/home/.local/bin/rofi-calendar" "INS-07 previous binary remains"
assert_no_file "$E/state/rofi-apps/installed/.in-progress-calendar" "INS-07 no in-progress state"
assert_eq "1.2.0" "$(sed -n 's/^version=//p' "$E/state/rofi-apps/installed/calendar")" "INS-07 state version untouched"
assert_success "INS-07 app still runs after rollback" in_env "$E/home/.local/bin/rofi-calendar"

# ---------------------------------------------------------------------------
# INS-03 / INS-14: app-owned service and --no-service
# ---------------------------------------------------------------------------
new_env
assert_success "install screen-time without service" install_app --app screen-time --no-service
assert_no_file "$E/config/systemd/user/screen-time.service" "INS-03 --no-service installs no unit"
assert_no_file "$E/systemctl.log" "INS-03 --no-service does not call systemctl"
assert_file "$E/home/.local/bin/screen-time" "screen-time binary installed"
assert_file "$E/home/.local/bin/rofi-screen-time" "screen-time wrapper installed"

new_env
assert_success "install screen-time with service" install_app --app screen-time
assert_file "$E/config/systemd/user/screen-time.service" "INS-14 unit comes from app package"
assert_contains "enable --now screen-time.service" "$E/systemctl.log" "INS-14 unit enabled"

# ---------------------------------------------------------------------------
# INS-02: --all installs every non-optional app and skips the menu
# ---------------------------------------------------------------------------
new_env
assert_success "INS-02 install all" install_app --all
for bin in rofi-calendar rofi-clipboard rofi-keybinds rofi-network screen-time; do
    assert_file "$E/home/.local/bin/$bin" "INS-02 $bin installed"
done
assert_no_file "$E/home/.local/bin/rofi-hub-menu" "INS-02 optional menu skipped"
assert_no_file "$E/home/.local/bin/rofi-hub" "INS-02 no hub binary"
for provider in calendar clipboard keybinds network screen-time; do
    assert_file "$E/data/rofi-apps/providers/$provider.toml" "INS-02 provider $provider"
done

# --list reports installed state.
new_env
assert_success "install calendar for --list" install_app --app calendar
if in_env "$ROOT/install.sh" --index "$DIST/index.json" --pubkey "$WORK/pub" --list >"$WORK/list.txt" 2>&1; then
    pass "--list exits zero"
else
    fail "--list exits zero"
fi
assert_contains "calendar" "$WORK/list.txt" "--list includes calendar"
assert_contains "installed" "$WORK/list.txt" "--list reports installed"

# ---------------------------------------------------------------------------
# SEC-03: menu manifests use argv, never a shell
# ---------------------------------------------------------------------------
TRANSCRIPT="$WORK/transcript"
cat >"$WORK/shell-probe.sh" <<EOF
#!/bin/sh
printf 'ran' >"$TRANSCRIPT"
EOF
chmod 755 "$WORK/shell-probe.sh"
PROVIDER_ROOT="$WORK/provider-root"
mkdir -p "$PROVIDER_ROOT/rofi-apps/providers"
cat >"$PROVIDER_ROOT/rofi-apps/providers/probe.toml" <<EOF
id = "probe"
name = "Probe"
description = "shell metachar test"
exec = ["$WORK/shell-probe.sh", "; touch $WORK/pwned"]
EOF
rm -f "$WORK/pwned"
env HOME="$WORK/home" XDG_CONFIG_HOME="$WORK/config" XDG_DATA_HOME="$PROVIDER_ROOT" \
    PATH="$E/bin:$WORK/tools" "$ROOT/target/release/rofi-hub-menu" probe >/dev/null 2>&1 || true
sleep 0.2
assert_no_file "$WORK/pwned" "SEC-03 exec is argv, not shell interpolation"

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
