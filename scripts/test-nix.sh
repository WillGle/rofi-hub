#!/usr/bin/env bash
# Nix package/module/closure checks.
#
# Requires a Nix installation with flakes enabled. Cross-architecture builds
# are eval-only unless the host has a remote builder or binfmt emulation.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-apps-nix.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT INT TERM
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); printf 'ok   %s\n' "$1"; }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %s\n' "$1" >&2; }

if ! command -v nix >/dev/null 2>&1; then
    printf 'test-nix: nix is not installed; skipping\n' >&2
    exit 0
fi

cd "$ROOT"
SYSTEM="$(nix eval --raw --impure --expr builtins.currentSystem)"
APPS=(rofi-calendar rofi-clipboard rofi-keybinds rofi-network screen-time rofi-hub-menu)

for app in "${APPS[@]}"; do
    if nix build --no-link ".#$app" >/dev/null 2>&1; then
        pass "NIX-01 nix build .#$app"
    else
        fail "NIX-01 nix build .#$app"
        continue
    fi
    closure="$(nix path-info -rSh ".#$app" 2>/dev/null | awk '{print $1}')"
    if [[ "$app" != "rofi-hub-menu" ]] && grep -qE -- '-rofi-hub(-menu)?-[0-9]' <<<"$closure"; then
        fail "NIX-01 $app closure contains hub/menu"
    else
        pass "NIX-01 $app closure excludes hub/menu"
    fi
    if grep -qE -- '-rofi-calendar-|-rofi-clipboard-|-rofi-keybinds-|-rofi-network-|-screen-time-' <<<"$closure" |
        grep -v "$app-" | grep -q .; then
        fail "NIX-01 $app closure contains another app"
    else
        pass "NIX-01 $app closure excludes other apps"
    fi
    if ! grep -qE -- '-rofi-[0-9]' <<<"$closure"; then
        fail "NIX-01 $app closure is missing rofi"
    else
        pass "NIX-01 $app closure carries rofi"
    fi
done

# Per-app module: only that app package is installed.
nix build --no-link ".#checks.${SYSTEM}.module-calendar-eval" >/dev/null 2>&1 || true
calendar_eval="$(nix build --no-link --print-out-paths ".#checks.${SYSTEM}.module-calendar-eval" 2>/dev/null || true)"
if [[ -n "$calendar_eval" ]] && grep -q -- '-rofi-calendar-' "$calendar_eval"; then
    pass "NIX-02 calendar module installs the calendar package"
else
    fail "NIX-02 calendar module installs the calendar package"
fi
if [[ -n "$calendar_eval" ]] && grep -qE -- '-rofi-clipboard-|-rofi-keybinds-|-rofi-network-|-screen-time-|-rofi-hub' "$calendar_eval"; then
    fail "NIX-02 calendar module pulls another app"
else
    pass "NIX-02 calendar module pulls no other app"
fi

screen_eval="$(nix build --no-link --print-out-paths ".#checks.${SYSTEM}.module-screen-time-eval" 2>/dev/null || true)"
if [[ -n "$screen_eval" ]] && grep -q -- '-screen-time-' "$screen_eval"; then
    pass "NIX-04 screen-time module installs the app"
else
    fail "NIX-04 screen-time module installs the app"
fi
if [[ -n "$screen_eval" ]] && grep -q 'screen-time daemon' "$screen_eval"; then
    pass "NIX-04 screen-time module owns the service"
else
    fail "NIX-04 screen-time module owns the service"
fi
if [[ -n "$screen_eval" ]] && grep -qE -- '-rofi-calendar-|-rofi-clipboard-|-rofi-keybinds-|-rofi-network-|-rofi-hub' "$screen_eval"; then
    fail "NIX-04 screen-time module pulls another app"
else
    pass "NIX-04 screen-time module pulls no other app"
fi

menu_eval="$(nix build --no-link --print-out-paths ".#checks.${SYSTEM}.module-menu-eval" 2>/dev/null || true)"
if [[ -n "$menu_eval" ]] && grep -q -- '-rofi-hub-menu-' "$menu_eval"; then
    pass "NIX-05 menu module is optional and installs only the menu"
else
    fail "NIX-05 menu module is optional and installs only the menu"
fi
if [[ -n "$menu_eval" ]] && grep -qE -- '-rofi-calendar-|-screen-time-|-rofi-clipboard-' "$menu_eval"; then
    fail "NIX-05 menu module pulls apps"
else
    pass "NIX-05 menu module pulls no apps"
fi

# App modules must not reintroduce a global runtime package list.
if grep -q "runtimePackagesFor" flake.nix modules -r; then
    fail "NIX-03 no runtimePackagesFor"
else
    pass "NIX-03 no runtimePackagesFor"
fi

# aarch64: evaluation is available even without a builder.
if nix eval --raw ".#packages.aarch64-linux.rofi-calendar.drvPath" >/dev/null 2>&1; then
    pass "NIX-07 aarch64 package evaluates"
else
    fail "NIX-07 aarch64 package evaluates"
fi
if [[ "$(uname -m)" = "aarch64" ]]; then
    if nix build --no-link .#rofi-calendar >/dev/null 2>&1; then
        pass "NIX-07 native aarch64 build"
    else
        fail "NIX-07 native aarch64 build"
    fi
else
    printf 'note: aarch64 native build requires a remote builder; eval checked\n'
fi
# The flake also exposes cross-built aarch64 packages on x86_64. These are
# evaluation-guarded; the release path still builds natively on arm64.
if [[ "$(uname -m)" = "x86_64" ]]; then
    if nix eval --raw ".#packages.x86_64-linux.rofi-calendar-aarch64.drvPath" >/dev/null 2>&1; then
        pass "NIX-08 cross aarch64 package evaluates"
    else
        fail "NIX-08 cross aarch64 package evaluates"
    fi
fi

# Full flake check (builds checks, including per-app tests).
if nix flake check >/dev/null 2>&1; then
    pass "NIX-06 nix flake check"
else
    fail "NIX-06 nix flake check"
fi

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
