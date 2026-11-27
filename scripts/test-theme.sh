#!/usr/bin/env bash
# Theme precedence, isolation, and cache tests.
#
# Proves that app overrides, ROFI_THEME, the user base theme, and the packaged
# fallback win in the documented order, and that no hub state participates.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-apps-theme.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT INT TERM
PASS=0
FAIL=0
BIN_DIR="${ROFI_APPS_BIN_DIR:-$ROOT/target/debug}"

pass() { PASS=$((PASS + 1)); printf 'ok   %s\n' "$1"; }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %s\n' "$1" >&2; }
assert_contains() {
    if grep -qF -- "$1" "$2" 2>/dev/null; then pass "$3"; else fail "$3 (missing '$1' in $2)"; fi
}
assert_not_contains() {
    if grep -qF -- "$1" "$2" 2>/dev/null; then fail "$3 (unexpected '$1' in $2)"; else pass "$3"; fi
}
assert_mode() {
    local expected="$1" path="$2" label="$3" actual
    actual="$(stat -c '%a' "$path" 2>/dev/null || printf missing)"
    if [[ "$expected" = "$actual" ]]; then pass "$label"; else fail "$label (expected $expected, got $actual)"; fi
}

if [[ ! -x "$BIN_DIR/rofi-calendar" ]]; then
    (cd "$ROOT" && cargo build --workspace) >/dev/null
    BIN_DIR="$ROOT/target/debug"
fi

E="$WORK/env"
# Isolate XDG_DATA_DIRS so host packages cannot leak a fallback theme into
# tests after the packaged-fallback case. THM-04 overrides it explicitly.
mkdir -p "$E/home" "$E/bin" "$E/state" "$E/config" "$E/data" "$E/run" \
    "$E/data-dirs" "$E/packaged/rofi"
cat >"$E/bin/rofi" <<'EOF'
#!/bin/sh
printf '%s\n' "$@" >"${ROFI_LOG:-/dev/null}"
cat >/dev/null || true
exit 1
EOF
chmod 755 "$E/bin/rofi"
export ROFI_LOG="$E/rofi.args"

app_env() {
    env HOME="$E/home" \
        XDG_STATE_HOME="$E/state" \
        XDG_CONFIG_HOME="$E/config" \
        XDG_DATA_HOME="$E/data" \
        XDG_DATA_DIRS="$E/data-dirs" \
        XDG_RUNTIME_DIR="$E/run" \
        PATH="$E/bin:$PATH" \
        ROFI_LOG="$ROFI_LOG" \
        TZ=UTC \
        "$@"
}

run_calendar() {
    rm -f "$E/rofi.args"
    app_env "$BIN_DIR/rofi-calendar" >/dev/null 2>&1 || true
    if [[ ! -f "$E/rofi.args" ]]; then
        fail "calendar did not invoke rofi"
    fi
    return 0
}

theme_arg() {
    awk 'previous == "-theme" { print; exit } { previous = $0 }' "$E/rofi.args"
}

mkdir -p "$E/config/rofi"

# THM-01: full app override wins.
printf '* { background-color: #101010; }\n' >"$E/config/rofi/calendar.rasi"
printf '* { background-color: #202020; }\n' >"$WORK/global.rasi"
printf '* { background-color: #303030; }\n' >"$E/config/rofi/theme.rasi"
run_calendar
assert_contains "$E/config/rofi/calendar.rasi" "$E/rofi.args" "THM-01 app override wins"
rm -f "$E/config/rofi/calendar.rasi"

# THM-02: ROFI_THEME is a full global override.
app_env env ROFI_THEME="$WORK/global.rasi" "$BIN_DIR/rofi-calendar" >/dev/null 2>&1 || true
assert_contains "$WORK/global.rasi" "$E/rofi.args" "THM-02 ROFI_THEME wins over base"

# THM-03: base theme + app layout.
rm -f "$E/config/rofi/calendar.rasi"
run_calendar
theme="$(theme_arg)"
if [[ "$theme" = "$E/run/rofi/theme-calendar.rasi" ]]; then
    pass "THM-03 generated runtime theme selected"
else
    fail "THM-03 generated runtime theme selected (got $theme)"
fi
assert_contains "@import" "$theme" "THM-03 base theme is imported"
assert_contains "$E/config/rofi/theme.rasi" "$theme" "THM-03 imported path is the user base"
if grep -q "background-color" "$theme"; then
    fail "THM-03 layout layer does not own colors"
else
    pass "THM-03 layout layer does not own colors"
fi
assert_mode 600 "$theme" "SEC-04 generated theme is 0600"
assert_mode 700 "$E/run/rofi" "SEC-04 runtime theme dir is 0700"

# THM-07: a cache hit does not rewrite the generated theme.
before="$(stat -c '%i' "$theme")"
run_calendar
after="$(stat -c '%i' "$(theme_arg)")"
if [[ "$before" = "$after" ]]; then pass "THM-07 theme cache does not rewrite"; else fail "THM-07 theme cache does not rewrite"; fi

# THM-08: hub state is irrelevant.
mkdir -p "$E/config/rofi-hub"
printf 'nord\n' >"$E/config/rofi-hub/theme"
contents_before="$(sha256sum "$theme" | awk '{print $1}')"
run_calendar
contents_after="$(sha256sum "$(theme_arg)" | awk '{print $1}')"
if [[ "$contents_before" = "$contents_after" ]]; then
    pass "THM-08 hub theme state does not affect apps"
else
    fail "THM-08 hub theme state does not affect apps"
fi

# THM-04: packaged fallback when the user has no base theme.
rm -f "$E/config/rofi/theme.rasi"
printf '* { background-color: #404040; }\n' >"$E/packaged/rofi/calendar.rasi"
app_env env XDG_DATA_DIRS="$E/packaged" "$BIN_DIR/rofi-calendar" >/dev/null 2>&1 || true
assert_contains "$E/packaged/rofi/calendar.rasi" "$E/rofi.args" "THM-04 packaged fallback wins without user theme"

# THM-05: the optional menu theme is independent.
mkdir -p "$E/data/rofi-apps/providers"
cat >"$E/data/rofi-apps/providers/probe.toml" <<'EOF'
id = "probe"
name = "Probe"
description = "Theme isolation"
category = "Utilities"
exec = ["/bin/true"]
EOF
app_env "$ROOT/target/debug/rofi-hub-menu" theme set nord >/dev/null 2>&1 || true
run_calendar
if [[ "$(theme_arg)" = "$E/run/rofi/theme-calendar.rasi" ]] && ! grep -q "rofi-hub-menu" "$E/rofi.args"; then
    pass "THM-05 menu theme does not affect apps"
else
    fail "THM-05 menu theme does not affect apps"
fi

# THM-06: screen-time's app override does not affect calendar.
mkdir -p "$E/config/rofi"
printf '* { background-color: #505050; }\n' >"$E/config/rofi/screen-time.rasi"
run_calendar
if [[ "$(theme_arg)" = "$E/run/rofi/theme-calendar.rasi" ]] && ! grep -q "screen-time" "$E/rofi.args"; then
    pass "THM-06 screen-time theme is isolated"
else
    fail "THM-06 screen-time theme is isolated"
fi

# Built-in fallback still works with no user or packaged theme.
rm -rf "$E/packaged" "$E/config/rofi" "$E/config/rofi-hub"
run_calendar
assert_contains "background-color: #1d2021" "$(theme_arg)" "THM-04 built-in palette fallback"

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
