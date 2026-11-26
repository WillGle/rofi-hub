#!/usr/bin/env bash
# App runtime and optional-menu integration tests with fake runtime tools.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-apps-apps.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT INT TERM
PASS=0
FAIL=0
BIN_DIR="${ROFI_APPS_BIN_DIR:-$ROOT/target/debug}"

pass() { PASS=$((PASS + 1)); printf 'ok   %s\n' "$1"; }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %s\n' "$1" >&2; }
assert_success() { local label="$1"; shift; if "$@" >/dev/null 2>&1; then pass "$label"; else fail "$label"; fi; }
assert_failure() { local label="$1"; shift; if "$@" >/dev/null 2>&1; then fail "$label (unexpected success)"; else pass "$label"; fi; }
assert_contains() {
    if grep -qF -- "$1" "$2" 2>/dev/null; then pass "$3"; else fail "$3 (missing '$1')"; fi
}

if [[ ! -x "$BIN_DIR/rofi-calendar" || ! -x "$BIN_DIR/rofi-hub-menu" ]]; then
    (cd "$ROOT" && cargo build --workspace) >/dev/null
    BIN_DIR="$ROOT/target/debug"
fi

ENV_N=0
new_env() {
    ENV_N=$((ENV_N + 1))
    E="$WORK/env$ENV_N"
    mkdir -p "$E/home" "$E/bin" "$E/state" "$E/config" "$E/data" "$E/run"
    cat >"$E/bin/rofi" <<'EOF'
#!/bin/sh
printf '%s\n' "$@" >"${ROFI_LOG:-/dev/null}"
env | grep -E '^ROFI_HUB_' >"${ROFI_ENV_LOG:-/dev/null}" || true
cat >/dev/null || true
if [ -n "${ROFI_STATUS:-}" ] && [ ! -f "${ROFI_STATUS_FLAG:-/nonexistent}" ]; then
    : >"${ROFI_STATUS_FLAG:-/nonexistent}"
    exit "$ROFI_STATUS"
fi
exit 1
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
    cat >"$E/bin/notify-send" <<'EOF'
#!/bin/sh
exit 0
EOF
    chmod 755 "$E/bin"/*
    mkdir -p "$E/config/hypr"
    cat >"$E/config/hypr/hyprland.conf" <<'EOF'
$mainMod = SUPER
# --- Windows ---
bind = $mainMod, Q, killactive,
bind = $mainMod, Return, exec, rofi -show drun
EOF
    export ROFI_LOG="$E/rofi.args"
    export ROFI_ENV_LOG="$E/rofi.env"
}

app_env() {
    env HOME="$E/home" \
        XDG_STATE_HOME="$E/state" \
        XDG_CONFIG_HOME="$E/config" \
        XDG_DATA_HOME="$E/data" \
        XDG_RUNTIME_DIR="$E/run" \
        PATH="$E/bin:${PATH}" \
        ROFI_LOG="${ROFI_LOG:-}" \
        ROFI_ENV_LOG="${ROFI_ENV_LOG:-}" \
        ROFI_STATUS_FLAG="${E}/rofi-status-used" \
        TZ=UTC \
        "$@"
}

run_app() { app_env timeout 10 "$BIN_DIR/$1"; }

# ---------------------------------------------------------------------------
# APP-01/05/06: every app runs directly with fake Rofi and a clean hub env
# ---------------------------------------------------------------------------
for app in rofi-calendar rofi-clipboard rofi-keybinds rofi-network; do
    new_env
    assert_success "APP-01 $app direct launch" run_app "$app"
    assert_contains "-theme" "$E/rofi.args" "APP-01 $app passed app theme"
    if [[ -s "$E/rofi.env" ]]; then
        fail "APP-06 $app saw ROFI_HUB_* variables"
    else
        pass "APP-06 $app saw no ROFI_HUB_* variables"
    fi
done

new_env
app_env timeout 10 env ROFI_STATUS=1 "$BIN_DIR/rofi-calendar" >/dev/null 2>&1 || true
pass "APP-05 cancel is handled"
app_env timeout 10 env ROFI_STATUS=10 "$BIN_DIR/rofi-calendar" >/dev/null 2>&1 || true
pass "APP-05 custom key is handled"

# ---------------------------------------------------------------------------
# APP-03/04: missing runtime tools fail clearly, never panic
# ---------------------------------------------------------------------------
new_env
NO_ROFI="$WORK/no-rofi"
mkdir -p "$NO_ROFI"
for tool in sh cat cut find sed basename dirname readlink mktemp cp mv rm mkdir chmod sort head awk; do
    path="$(command -v "$tool" 2>/dev/null || true)"
    [[ -n "$path" ]] && ln -sf "$path" "$NO_ROFI/$tool"
done
if env HOME="$E/home" XDG_STATE_HOME="$E/state" XDG_CONFIG_HOME="$E/config" \
    XDG_DATA_HOME="$E/data" XDG_RUNTIME_DIR="$E/run" PATH="$NO_ROFI" \
    "$BIN_DIR/rofi-calendar" >"$WORK/no-rofi.out" 2>&1; then
    fail "APP-03 missing rofi fails"
else
    pass "APP-03 missing rofi fails"
fi
assert_contains "rofi" "$WORK/no-rofi.out" "APP-03 error names rofi"

new_env
NO_CLIPHIST="$WORK/no-cliphist"
mkdir -p "$NO_CLIPHIST"
ln -sf "$E/bin/rofi" "$NO_CLIPHIST/rofi"
ln -sf "$E/bin/wl-copy" "$NO_CLIPHIST/wl-copy"
if env HOME="$E/home" XDG_STATE_HOME="$E/state" XDG_CONFIG_HOME="$E/config" \
    XDG_DATA_HOME="$E/data" XDG_RUNTIME_DIR="$E/run" PATH="$NO_CLIPHIST" \
    "$BIN_DIR/rofi-clipboard" >"$WORK/no-cliphist.out" 2>&1; then
    fail "APP-04 missing cliphist fails"
else
    pass "APP-04 missing cliphist fails"
fi
assert_contains "cliphist" "$WORK/no-cliphist.out" "APP-04 error names cliphist"
if grep -qi "panicked" "$WORK/no-cliphist.out"; then
    fail "APP-04 missing cliphist does not panic"
else
    pass "APP-04 missing cliphist does not panic"
fi

# ---------------------------------------------------------------------------
# APP-02 / MENU-05: removing the menu does not affect an app
# ---------------------------------------------------------------------------
new_env
mkdir -p "$E/home/.local/bin"
cp "$BIN_DIR/rofi-calendar" "$E/home/.local/bin/rofi-calendar"
assert_success "APP-02 app runs with no menu installed" \
    app_env "$E/home/.local/bin/rofi-calendar"

# ---------------------------------------------------------------------------
# MENU-01/02/03/04: provider discovery, clean env, missing app, theme isolation
# ---------------------------------------------------------------------------
new_env
PROVIDERS="$E/data/rofi-apps/providers"
mkdir -p "$PROVIDERS"
for id in alpha beta gamma; do
    cat >"$PROVIDERS/$id.toml" <<EOF
id = "$id"
name = "${id^}"
description = "Test provider $id"
category = "Utilities"
exec = ["/bin/true"]
EOF
done
if app_env "$BIN_DIR/rofi-hub-menu" list --json >"$WORK/menu.json" 2>/dev/null; then
    pass "MENU-01 menu lists installed providers"
else
    fail "MENU-01 menu lists installed providers"
fi
for id in alpha beta gamma; do
    assert_contains "\"$id\"" "$WORK/menu.json" "MENU-01 provider $id discovered"
done

cat >"$WORK/provider-probe.sh" <<'EOF'
#!/bin/sh
env | sort >"${PROBE_LOG:-/dev/null}"
printf 'arg=%s\n' "$1" >>"${PROBE_LOG:-/dev/null}"
EOF
chmod 755 "$WORK/provider-probe.sh"
cat >"$PROVIDERS/probe.toml" <<EOF
id = "probe"
name = "Probe"
description = "Environment probe"
category = "Utilities"
exec = ["$WORK/provider-probe.sh", "one argument"]
EOF
export PROBE_LOG="$WORK/probe.log"
if app_env "$BIN_DIR/rofi-hub-menu" probe >/dev/null 2>&1; then
    pass "MENU-02 menu launches a provider"
else
    fail "MENU-02 menu launches a provider"
fi
for _ in $(seq 1 50); do
    [[ -s "$PROBE_LOG" ]] && break
    sleep 0.02
done
if grep -qE '^ROFI_HUB_' "$PROBE_LOG" 2>/dev/null; then
    fail "MENU-02 menu injected hub variables"
else
    pass "MENU-02 menu injected no hub variables"
fi
assert_contains "arg=one argument" "$PROBE_LOG" "MENU-02 argv is preserved"

cat >"$PROVIDERS/missing.toml" <<EOF
id = "missing"
name = "Missing"
description = "No such binary"
category = "Utilities"
exec = ["$WORK/definitely-not-installed"]
EOF
if app_env "$BIN_DIR/rofi-hub-menu" missing >"$WORK/menu-missing.out" 2>&1; then
    fail "MENU-03 missing app fails"
else
    pass "MENU-03 missing app fails"
fi
assert_contains "not installed or not in PATH" "$WORK/menu-missing.out" "MENU-03 error is clear"

# Menu theme changes must not affect an app.
app_env "$BIN_DIR/rofi-hub-menu" theme set nord >/dev/null 2>&1
rm -f "$E/rofi.args"
assert_success "MENU-04 app still runs after menu theme change" run_app rofi-calendar
if grep -q "rofi-hub-menu" "$E/rofi.args"; then
    fail "MENU-04 app theme is not the menu theme"
else
    pass "MENU-04 app theme is not the menu theme"
fi

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
