#!/usr/bin/env bash
# End-to-end release bundle check through install.sh.
#
# Builds or accepts a dist directory, signs it with a throwaway key, installs
# every non-optional app into a temporary HOME, runs each app against fake
# runtime tools, then uninstalls everything.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-apps-bundle.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT INT TERM
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); printf 'ok   %s\n' "$1"; }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %s\n' "$1" >&2; }
assert_file() { if [[ -e "$1" ]]; then pass "$2"; else fail "$2 (missing $1)"; fi; }
assert_no_file() { if [[ ! -e "$1" ]]; then pass "$2"; else fail "$2 (unexpected $1)"; fi; }

command -v minisign >/dev/null 2>&1 || { printf 'minisign is required\n' >&2; exit 1; }

DIST="${1:-$WORK/dist}"
if [[ -z "${1:-}" ]]; then
    DIST="$DIST" "$ROOT/scripts/package-release.sh" >/dev/null
fi
[[ -f "$DIST/index.json" ]] || { printf 'test-bundle: missing %s/index.json\n' "$DIST" >&2; exit 1; }

minisign -G -W -p "$WORK/pub" -s "$WORK/sec" >/dev/null 2>&1
for file in "$DIST"/index.json "$DIST"/*.tar.gz; do
    minisign -Sm "$file" -s "$WORK/sec" -x "$file.minisig" >/dev/null
done

E="$WORK/env"
mkdir -p "$E/home" "$E/bin" "$E/state" "$E/config" "$E/data" "$E/run"
cat >"$E/bin/rofi" <<'EOF'
#!/bin/sh
printf '%s\n' "$@" >"${ROFI_LOG:-/dev/null}"
cat >/dev/null || true
exit "${ROFI_STATUS:-1}"
EOF
cat >"$E/bin/cliphist" <<'EOF'
#!/bin/sh
case "$1" in
    list) printf '1\tfirst entry\n2\tsecond entry\n' ;;
    decode) printf 'decoded value\n' ;;
esac
exit 0
EOF
cat >"$E/bin/wl-copy" <<'EOF'
#!/bin/sh
cat >/dev/null || true
exit 0
EOF
cat >"$E/bin/nmcli" <<'EOF'
#!/bin/sh
case "$*" in
    'radio wifi') echo enabled ;;
    *DEVICE,TYPE,STATE*) printf 'wlan0:wifi:connected\n' ;;
    *IN-USE,SSID,SIGNAL,SECURITY,FREQ*) printf '*:Home:80:WPA2:5180\n' ;;
    *GENERAL.CONNECTION*) printf 'GENERAL.CONNECTION:Home\n' ;;
esac
exit 0
EOF
cat >"$E/bin/systemctl" <<'EOF'
#!/bin/sh
printf '%s\n' "$*" >>"${SYSTEMCTL_LOG:-/dev/null}"
exit 0
EOF
mkdir -p "$E/config/hypr"
cat >"$E/config/hypr/hyprland.conf" <<'EOF'
$mainMod = SUPER
# --- Windows ---
bind = $mainMod, Q, killactive,
bind = $mainMod, Return, exec, rofi -show drun
EOF
chmod 755 "$E/bin/rofi" "$E/bin/cliphist" "$E/bin/wl-copy" "$E/bin/nmcli" "$E/bin/systemctl"

run_env() {
    env HOME="$E/home" \
        XDG_STATE_HOME="$E/state" \
        XDG_CONFIG_HOME="$E/config" \
        XDG_DATA_HOME="$E/data" \
        XDG_RUNTIME_DIR="$E/run" \
        PATH="$E/bin:$PATH" \
        ROFI_LOG="${ROFI_LOG:-}" \
        SYSTEMCTL_LOG="${SYSTEMCTL_LOG:-}" \
        TZ=UTC \
        "$@"
}

export ROFI_LOG="$E/rofi.args"
export SYSTEMCTL_LOG="$E/systemctl.log"

if run_env "$ROOT/install.sh" --index "$DIST/index.json" --pubkey "$WORK/pub" --all; then
    pass "install.sh --all installs the bundle"
else
    fail "install.sh --all installs the bundle"
fi

for bin in rofi-calendar rofi-clipboard rofi-keybinds rofi-network screen-time rofi-screen-time; do
    assert_file "$E/home/.local/bin/$bin" "bundle installs $bin"
done
assert_no_file "$E/home/.local/bin/rofi-hub" "bundle does not install the old hub"
assert_no_file "$E/home/.local/bin/rofi-hub-menu" "bundle does not install the optional menu"
assert_no_file "$E/config/rofi-hub" "bundle does not write hub theme state"
assert_file "$E/config/systemd/user/screen-time.service" "bundle installs the app service unit"

# Each app must run with its own runtime tools and no hub.
for app in rofi-calendar rofi-clipboard rofi-keybinds rofi-network; do
    rm -f "$E/rofi.args"
    if run_env "$E/home/.local/bin/$app" >/dev/null 2>&1; then
        pass "$app runs with fake rofi"
    else
        fail "$app runs with fake rofi"
    fi
    assert_file "$E/rofi.args" "$app invoked rofi"
    if grep -q -- "-theme" "$E/rofi.args"; then
        pass "$app passed an app theme"
    else
        fail "$app passed an app theme"
    fi
done

# Screen Time's wrapper needs the sibling daemon and a working renderer.
rm -f "$E/rofi.args"
status=0
run_env "$E/home/.local/bin/rofi-screen-time" >/dev/null 2>&1 || status=$?
if ((status == 0 || status == 1)); then
    pass "rofi-screen-time runs with fake rofi"
else
    fail "rofi-screen-time runs with fake rofi (status $status)"
fi
assert_file "$E/rofi.args" "rofi-screen-time invoked rofi"
if grep -q -- "-show" "$E/rofi.args" && grep -q -- "-theme" "$E/rofi.args"; then
    pass "rofi-screen-time passed its own theme and mode"
else
    fail "rofi-screen-time passed its own theme and mode"
fi

# Release payload must not contain distro artifacts.
if find "$DIST" -maxdepth 1 \( -name '*.deb' -o -name '*.rpm' -o -name '*.apk' \) | grep -q .; then
    fail "release contains no distro artifacts"
else
    pass "release contains no distro artifacts"
fi

if run_env "$ROOT/install.sh" --uninstall; then
    pass "install.sh --uninstall removes installed apps"
else
    fail "install.sh --uninstall removes installed apps"
fi
assert_no_file "$E/home/.local/bin/rofi-calendar" "uninstall removed calendar"
assert_no_file "$E/home/.local/bin/rofi-screen-time" "uninstall removed screen-time wrapper"
assert_no_file "$E/state/rofi-apps/installed/calendar" "uninstall removed calendar state"

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
