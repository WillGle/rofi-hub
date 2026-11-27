#!/usr/bin/env bash
# Inline Atomic Tasks menu tests with a fake Rofi.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)"
APP="$ROOT/crates/atomic-note/share/atomic-note"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/atomic-menu.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT INT TERM
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); printf 'ok   %s\n' "$1"; }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %s\n' "$1" >&2; }
assert_contains() {
    if grep -qF -- "$1" "$2" 2>/dev/null; then pass "$3"; else fail "$3 (missing '$1')"; fi
}
assert_not_contains() {
    if grep -qF -- "$1" "$2" 2>/dev/null; then fail "$3 (unexpected '$1')"; else pass "$3"; fi
}

new_env() {
    ENV_DIR="$WORK/env$1"
    mkdir -p "$ENV_DIR/home" "$ENV_DIR/bin" "$ENV_DIR/config" "$ENV_DIR/data" "$ENV_DIR/run"
    cp "$ROOT/crates/atomic-note/share/rofi/atomic-note.rasi" "$ENV_DIR/atomic-note.rasi"
    cat >"$ENV_DIR/bin/rofi" <<'EOF'
#!/bin/sh
printf '%s\n' "$@" >>"$ARGS_LOG"
count=0
theme=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -theme) theme=$2; shift 2 ;;
        *) shift ;;
    esac
done
if [ -n "$theme" ] && [ -r "$theme" ]; then
    count=$(find "$THEME_DIR" -name 'theme.*' 2>/dev/null | wc -l)
    count=$((count + 1))
    cp "$theme" "$THEME_DIR/theme.$count"
fi
cat >/dev/null
if [ ! -s "$REPLY_FILE" ]; then exit 1; fi
reply=$(sed -n '1p' "$REPLY_FILE")
if [ "$(wc -l <"$REPLY_FILE")" -gt 1 ]; then
    sed '1d' "$REPLY_FILE" >"$REPLY_FILE.tmp"
    mv "$REPLY_FILE.tmp" "$REPLY_FILE"
else
    rm -f "$REPLY_FILE"
fi
case "$reply" in
    @row:*:*)
        rest=${reply#@row:}
        row=${rest%%:*}
        filter=${rest#*:}
        printf '%s\n%s\n' "$row" "$filter"
        exit 0
        ;;
    @custom:*:*:*)
        rest=${reply#@custom:}
        code=${rest%%:*}
        rest=${rest#*:}
        row=${rest%%:*}
        filter=${rest#*:}
        printf '%s\n%s\n' "$row" "$filter"
        exit $((10 + code))
        ;;
    @cancel) exit 1 ;;
    *) printf '%s\n' "$reply"; exit 0 ;;
esac
EOF
    cat >"$ENV_DIR/bin/pkill" <<'EOF'
#!/bin/sh
exit 0
EOF
    chmod +x "$ENV_DIR/bin/"*
    mkdir -p "$ENV_DIR/themes"
    export HOME="$ENV_DIR/home"
    export XDG_CONFIG_HOME="$ENV_DIR/config"
    export XDG_DATA_HOME="$ENV_DIR/data"
    export XDG_STATE_HOME="$ENV_DIR/state"
    export XDG_RUNTIME_DIR="$ENV_DIR/run"
    export PATH="$ENV_DIR/bin:$PATH"
    export REPLY_FILE="$ENV_DIR/replies"
    export ARGS_LOG="$ENV_DIR/args.log"
    export THEME_DIR="$ENV_DIR/themes"
    export ROFI_BIN="$ENV_DIR/bin/rofi"
    export ROFI_THEME="$ENV_DIR/atomic-note.rasi"
    : >"$ARGS_LOG"
    printf '[Critical] Alpha task\n[Moderate] Beta task\n' >"$HOME/.atomic_tasks"
}

run_menu() {
    reply_file="$1"
    shift
    printf '%s\n' "$@" >"$reply_file"
    timeout 10 bash "$APP" >"$ENV_DIR/out" 2>"$ENV_DIR/err" || true
}

new_env 1
run_menu "$REPLY_FILE" "@row:0:Alpha" "@custom:4:0:Alpha" "@cancel"
assert_contains "[Moderate] Beta task" "$HOME/.atomic_tasks" "done removes the selected task"
assert_contains "[Critical] Alpha task" "$HOME/.atomic_tasks.done" "done keeps an undo log"
assert_contains "-filter" "$ARGS_LOG" "the typed filter is passed on reopen"
assert_contains "Alpha" "$ARGS_LOG" "the typed filter survives the action"
assert_contains "children: [ actionbar, box-task-actions, box-priority-actions, inputbar, statusbar, listview ]" "$THEME_DIR/theme.2" "Enter expands the inline action area"
for button in button-done button-edit button-move button-critical button-high button-moderate button-low button-actions; do
    assert_contains "$button" "$THEME_DIR/theme.2" "expanded area has $button"
done
assert_contains 'action: "kb-custom-5"' "$THEME_DIR/theme.2" "Done button uses the Done key"
assert_contains 'action: "kb-custom-1"' "$THEME_DIR/theme.2" "Add button uses the Add key"
assert_not_contains "children: [ actionbar, inputbar, statusbar, listview ]" "$THEME_DIR/theme.2" "no second menu replaces the main list"

if bash "$APP" undo >/dev/null 2>&1; then
    assert_contains "[Critical] Alpha task" "$HOME/.atomic_tasks" "undo restores the done batch"
else
    fail "undo restores the done batch"
fi
if bash "$APP" render-waybar >"$ENV_DIR/waybar.json" 2>/dev/null; then
    assert_contains '"text"' "$ENV_DIR/waybar.json" "Waybar render returns JSON"
    assert_contains "Alpha task" "$ENV_DIR/waybar.json" "Waybar render includes the task"
else
    fail "Waybar render returns JSON"
fi

new_env 2
run_menu "$REPLY_FILE" "@row:0:" "@custom:7:0:" "@cancel"
assert_contains "[Critical] Alpha task" "$HOME/.atomic_tasks" "Alt+1 sets the selected task to critical"

new_env 3
run_menu "$REPLY_FILE" "@row:0:" "@custom:5:0:" "Edited task" "@cancel"
assert_contains "[Critical] Edited task" "$HOME/.atomic_tasks" "Alt+e edits the selected task inline"

new_env 4
run_menu "$REPLY_FILE" "@row:1:" "@custom:6:1:" "@cancel"
first=$(sed -n '1p' "$HOME/.atomic_tasks")
second=$(sed -n '2p' "$HOME/.atomic_tasks")
if [ "$first" = "[Moderate] Beta task" ] && [ "$second" = "[Critical] Alpha task" ]; then
    pass "Alt+Up moves the selected task to the top"
else
    fail "Alt+Up moves the selected task to the top"
fi

new_env 5
choice="$XDG_STATE_HOME/rofi-apps/atomic-note/theme-choice"
run_menu "$REPLY_FILE" "@custom:11:0:Alpha" "@cancel"
assert_contains "-kb-custom-12" "$ARGS_LOG" "Ctrl+1 is bound to the theme control"
assert_contains "Control+1" "$ARGS_LOG" "Ctrl+1 is shown as the theme shortcut"
if [ "$(cat "$choice" 2>/dev/null)" = "nord" ]; then
    pass "Ctrl+1 locks the Nord palette"
else
    fail "Ctrl+1 locks the Nord palette"
fi
assert_contains "#2e3440" "$THEME_DIR/theme.2" "the locked Nord palette is used on reopen"

run_menu "$REPLY_FILE" "@cancel"
assert_contains "#2e3440" "$THEME_DIR/theme.3" "the saved palette survives a new launch"

run_menu "$REPLY_FILE" "@custom:14:0:Alpha" "@cancel"
if [ ! -e "$choice" ]; then
    pass "Ctrl+4 removes the theme lock"
else
    fail "Ctrl+4 removes the theme lock"
fi
run_menu "$REPLY_FILE" "@cancel"
assert_not_contains "#2e3440" "$THEME_DIR/theme.5" "Ctrl+4 restores the Auto palette"

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
