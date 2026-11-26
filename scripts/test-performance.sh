#!/usr/bin/env bash
# Startup budget check for the standalone apps.
#
# Each app is launched with a fake Rofi that exits immediately, so the
# measurement covers process start, theme resolution, config reads, and row
# construction. The budget is deliberately loose (0.4s) so slow CI runners do
# not flake; regressions should compare the printed medians, not just pass/fail.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-apps-perf.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT INT TERM
PASS=0
FAIL=0
BUDGET_MS=400

pass() { PASS=$((PASS + 1)); printf 'ok   %s\n' "$1"; }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %s\n' "$1" >&2; }

BIN_DIR="${ROFI_APPS_BIN_DIR:-$ROOT/target/release}"
if [[ ! -x "$BIN_DIR/rofi-calendar" ]]; then
    (cd "$ROOT" && cargo build --release --workspace) >/dev/null
    BIN_DIR="$ROOT/target/release"
fi

mkdir -p "$WORK/home" "$WORK/bin" "$WORK/state" "$WORK/config/hypr" \
    "$WORK/data" "$WORK/run"
cat >"$WORK/bin/rofi" <<'EOF'
#!/bin/sh
cat >/dev/null || true
exit 1
EOF
cat >"$WORK/bin/cliphist" <<'EOF'
#!/bin/sh
case "$1" in
    list) printf '1\tentry\n' ;;
esac
exit 0
EOF
cat >"$WORK/bin/wl-copy" <<'EOF'
#!/bin/sh
exit 0
EOF
cat >"$WORK/bin/nmcli" <<'EOF'
#!/bin/sh
case "$*" in
    'radio wifi') echo enabled ;;
    *DEVICE,TYPE,STATE*) printf 'wlan0:wifi:connected\n' ;;
    *IN-USE,SSID,SIGNAL,SECURITY,FREQ*) printf '*:Home:80:WPA2:5180\n' ;;
esac
exit 0
EOF
chmod 755 "$WORK/bin"/*
cat >"$WORK/config/hypr/hyprland.conf" <<'EOF'
$mainMod = SUPER
# --- Windows ---
bind = $mainMod, Q, killactive,
EOF

printf '%-24s %8s\n' "APP" "MEDIAN_MS"
perf_bin="$WORK/perf-bin"
mkdir -p "$perf_bin"
cp -- "$BIN_DIR/screen-time" "$perf_bin/screen-time"
cp -- "$ROOT/crates/screen-time/share/rofi-screen-time" "$perf_bin/rofi-screen-time"
chmod 755 "$perf_bin"/*

for app in rofi-calendar rofi-clipboard rofi-keybinds rofi-network rofi-screen-time; do
    bin="$BIN_DIR/$app"
    if [[ "$app" = "rofi-screen-time" ]]; then
        bin="$perf_bin/rofi-screen-time"
    fi
    [[ -x "$bin" ]] || { fail "missing $bin"; continue; }
    times=()
    for _ in 1 2 3 4 5; do
        start=$(date +%s%N)
        HOME="$WORK/home" XDG_STATE_HOME="$WORK/state" XDG_CONFIG_HOME="$WORK/config" \
            XDG_DATA_HOME="$WORK/data" XDG_RUNTIME_DIR="$WORK/run" \
            PATH="$WORK/bin:$PATH" TZ=UTC "$bin" >/dev/null 2>&1 || true
        end=$(date +%s%N)
        times+=( $(( (end - start) / 1000000 )) )
    done
    median="$(printf '%s\n' "${times[@]}" | sort -n | sed -n '3p')"
    printf '%-24s %8s\n' "$app" "$median"
    if (( median <= BUDGET_MS )); then
        pass "$app startup under ${BUDGET_MS}ms (median ${median}ms)"
    else
        fail "$app startup budget exceeded (median ${median}ms > ${BUDGET_MS}ms)"
    fi
done

# The theme cache must not rewrite on a second identical launch.
theme="$WORK/run/rofi/theme-calendar.rasi"
first_inode="$(stat -c '%i' "$theme")"
start=$(date +%s%N)
HOME="$WORK/home" XDG_STATE_HOME="$WORK/state" XDG_CONFIG_HOME="$WORK/config" \
    XDG_DATA_HOME="$WORK/data" XDG_RUNTIME_DIR="$WORK/run" \
    PATH="$WORK/bin:$PATH" TZ=UTC "$BIN_DIR/rofi-calendar" >/dev/null 2>&1 || true
end=$(date +%s%N)
second_inode="$(stat -c '%i' "$theme")"
printf '%-24s %8s\n' "cache-hit" "$(( (end - start) / 1000000 ))"
if [[ "$first_inode" = "$second_inode" ]]; then
    pass "theme cache hit does not rewrite the generated file"
else
    fail "theme cache hit rewrote the generated file"
fi

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
