#!/usr/bin/env bash
# Migration fixture: old rofi-hub install -> per-app install.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-apps-migration.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT INT TERM
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); printf 'ok   %s\n' "$1"; }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %s\n' "$1" >&2; }
assert_file() { if [[ -e "$1" ]]; then pass "$2"; else fail "$2 (missing $1)"; fi; }
assert_no_file() { if [[ ! -e "$1" ]]; then pass "$2"; else fail "$2 (unexpected $1)"; fi; }
assert_contains() {
    if grep -qF -- "$1" "$2" 2>/dev/null; then pass "$3"; else fail "$3 (missing '$1')"; fi
}
assert_same() {
    if [[ "$1" = "$2" ]]; then pass "$3"; else fail "$3"; fi
}

command -v minisign >/dev/null 2>&1 || { printf 'minisign is required\n' >&2; exit 1; }

DIST="$WORK/dist"
DIST="$DIST" "$ROOT/scripts/package-release.sh" >/dev/null
minisign -G -W -p "$WORK/pub" -s "$WORK/sec" >/dev/null 2>&1
for file in "$DIST"/index.json "$DIST"/*.tar.gz; do
    minisign -Sm "$file" -s "$WORK/sec" -x "$file.minisig" >/dev/null
done

E="$WORK/home-root"
H="$E/home"
mkdir -p "$H/.local/bin" "$H/.local/share/rofi-hub/providers" \
    "$H/.local/share/rofi-hub/themes" "$H/.config/rofi-hub" "$H/.config/rofi" \
    "$H/.local/state" "$E/bin" "$E/run"
cat >"$H/.local/bin/rofi-hub" <<'EOF'
#!/bin/sh
exit 0
EOF
chmod 755 "$H/.local/bin/rofi-hub"
cat >"$H/.local/share/rofi-hub/providers/legacy.toml" <<'EOF'
id = "legacy"
name = "Legacy App"
description = "Installed by the old hub"
category = "Utilities"
exec = ["/bin/true"]
EOF
printf '* { background-color: #0a0a0a; }\n' >"$H/.local/share/rofi-hub/themes/hub.rasi"
printf 'theme.mode = "nord"\n' >"$H/.config/rofi-hub/config.toml"
printf 'nord\n' >"$H/.config/rofi-hub/theme"
printf '* { background-color: #abcdef; }\n' >"$H/.config/rofi/theme.rasi"

# Old install snapshot.
OLD_PROVIDER="$H/.local/share/rofi-hub/providers/legacy.toml"
OLD_HUB="$H/.local/bin/rofi-hub"
OLD_CONFIG="$H/.config/rofi-hub/config.toml"
OLD_THEME_STATE="$H/.config/rofi-hub/theme"
OLD_PROVIDER_SUM="$(sha256sum "$OLD_PROVIDER" | awk '{print $1}')"
OLD_CONFIG_SUM="$(sha256sum "$OLD_CONFIG" | awk '{print $1}')"
OLD_STATE_SUM="$(sha256sum "$OLD_THEME_STATE" | awk '{print $1}')"

cat >"$E/bin/rofi" <<'EOF'
#!/bin/sh
printf '%s\n' "$@" >"${ROFI_LOG:-/dev/null}"
env | grep -E '^ROFI_HUB_' >"${ROFI_ENV_LOG:-/dev/null}" || true
cat >/dev/null || true
exit 1
EOF
chmod 755 "$E/bin/rofi"
export ROFI_LOG="$E/rofi.args"
export ROFI_ENV_LOG="$E/rofi.env"

run_env() {
    env HOME="$H" \
        XDG_STATE_HOME="$H/.local/state" \
        XDG_CONFIG_HOME="$H/.config" \
        XDG_DATA_HOME="$H/.local/share" \
        XDG_RUNTIME_DIR="$E/run" \
        PATH="$E/bin:$PATH" \
        ROFI_LOG="$ROFI_LOG" \
        ROFI_ENV_LOG="$ROFI_ENV_LOG" \
        TZ=UTC \
        "$@"
}

# MIG-01: install a per-app release next to the old hub.
if run_env "$ROOT/install.sh" --index "$DIST/index.json" --pubkey "$WORK/pub" --app calendar >/dev/null 2>&1; then
    pass "MIG-01 new installer installs calendar over an old hub"
else
    fail "MIG-01 new installer installs calendar over an old hub"
fi
assert_file "$E/home/.local/bin/rofi-calendar" "MIG-01 calendar installed"
assert_file "$OLD_HUB" "MIG-01 old hub binary preserved"
assert_same "$OLD_PROVIDER_SUM" "$(sha256sum "$OLD_PROVIDER" | awk '{print $1}')" "MIG-01 old provider unchanged"
assert_same "$OLD_CONFIG_SUM" "$(sha256sum "$OLD_CONFIG" | awk '{print $1}')" "MIG-01 old config unchanged"
assert_same "$OLD_STATE_SUM" "$(sha256sum "$OLD_THEME_STATE" | awk '{print $1}')" "MIG-01 old theme state unchanged"

# MIG-03: the menu still discovers the legacy provider path.
if run_env "$ROOT/target/release/rofi-hub-menu" list --json >"$WORK/menu.json" 2>/dev/null; then
    pass "MIG-03 menu lists providers after migration"
else
    fail "MIG-03 menu lists providers after migration"
fi
assert_contains '"legacy"' "$WORK/menu.json" "MIG-03 legacy provider path is read"
assert_contains '"calendar"' "$WORK/menu.json" "MIG-03 new provider is read"

# The migrated app runs with the user base theme, not the old hub theme.
mkdir -p "$H/.config/rofi"
printf '* { background-color: #abcdef; }\n' >"$H/.config/rofi/theme.rasi"
run_env "$E/home/.local/bin/rofi-calendar" >/dev/null 2>&1 || true
assert_contains "$E/run/rofi/theme-calendar.rasi" "$E/rofi.args" "MIG-01 app uses the user base theme"
if [[ -s "$E/rofi.env" ]]; then
    fail "MIG-02 migrated app does not read hub variables"
else
    pass "MIG-02 migrated app does not read hub variables"
fi

# MIG-02: remove the old hub and its provider dir; the app still runs.
rm -f "$OLD_HUB"
rm -rf "$H/.local/share/rofi-hub"
run_env "$E/home/.local/bin/rofi-calendar" >/dev/null 2>&1 || true
assert_file "$E/rofi.args" "MIG-02 calendar runs after removing the old hub"
assert_no_file "$H/.local/bin/rofi-hub" "MIG-02 old hub removed"
assert_file "$H/.config/rofi-hub/theme" "MIG-02 old user config is left for the user"

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
