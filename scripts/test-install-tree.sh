#!/usr/bin/env bash
# Validate packaging/install-tree.sh without touching the host system.
#
# The test stages a package-shaped tree into a temporary DESTDIR, compares it
# with packaging/MANIFEST, checks modes, validates the unit and desktop assets,
# and exercises the staged binaries with an isolated XDG environment.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-hub-install-tree.XXXXXX")"
trap 'chmod -R u+w "$WORK" 2>/dev/null || true; rm -rf -- "$WORK"' EXIT INT TERM

PREFIX="${PREFIX:-/usr}"
export HOME="$WORK/home"
export XDG_CONFIG_HOME="$WORK/config"
export XDG_DATA_HOME="$WORK/data"
export XDG_STATE_HOME="$WORK/state"
export XDG_RUNTIME_DIR="$WORK/run"
mkdir -p "$HOME" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_RUNTIME_DIR"

fail() {
    printf 'test-install-tree: FAIL: %s\n' "$*" >&2
    exit 1
}

pass() {
    printf 'test-install-tree: ok: %s\n' "$*"
}

skip() {
    printf 'test-install-tree: skip: %s\n' "$*" >&2
}

if [ "${SKIP_BUILD:-0}" -ne 1 ]; then
    if [ ! -x target/release/rofi-hub ] || [ ! -x target/release/screen-time ]; then
        printf 'test-install-tree: building workspace\n'
        cargo build --release --locked --workspace
    fi
fi
[ -x target/release/rofi-hub ] || fail "target/release/rofi-hub is missing (build first)"

# A fake systemctl catches accidental service management.
FAKEBIN="$WORK/fakebin"
mkdir -p "$FAKEBIN"
cat >"$FAKEBIN/systemctl" <<EOF
#!/usr/bin/env bash
printf 'systemctl %s\n' "\$*" >>"$WORK/systemctl.log"
exit 1
EOF
chmod 755 "$FAKEBIN/systemctl"
export PATH="$FAKEBIN:$PATH"

actual_files() {
    local root="$1"
    find "$root" -type f | sed "s|^${root}||" | LC_ALL=C sort
}

manifest_files() {
    awk 'NF && $1 !~ /^#/ {print $2}' "$1" | LC_ALL=C sort
}

check_tree() {
    local root="$1"
    local manifest="$2"
    local label="$3"
    actual_files "$root" >"$WORK/actual-list.$label"
    manifest_files "$manifest" >"$WORK/expected-list.$label"
    if ! diff -u "$WORK/expected-list.$label" "$WORK/actual-list.$label"; then
        fail "$label file list differs from manifest"
    fi
    while read -r mode path; do
        [ -n "$mode" ] || continue
        case "$mode" in \#*) continue ;; esac
        local actual_mode
        actual_mode="$(stat -c '%a' "${root}${path}")"
        [ "$actual_mode" = "$mode" ] || fail "$label mode for $path is $actual_mode, expected $mode"
    done <"$manifest"
    pass "$label tree matches $(wc -l <"$WORK/expected-list.$label") manifest entries"
}

stage() {
    local root="$1"
    shift
    DESTDIR="$root" PREFIX="$PREFIX" packaging/install-tree.sh "$@"
}

# --- Full install ---------------------------------------------------------
FULL="$WORK/full"
stage "$FULL" --all
check_tree "$FULL" packaging/MANIFEST full

PREFIX="$PREFIX" packaging/install-tree.sh --dry-run | LC_ALL=C sort >"$WORK/expected.dry"
LC_ALL=C sort packaging/MANIFEST >"$WORK/expected.static"
diff -u "$WORK/expected.static" "$WORK/expected.dry" ||
    fail "packaging/MANIFEST is out of date; regenerate from --dry-run"
pass "MANIFEST matches --dry-run"

[ ! -s "$WORK/systemctl.log" ] || fail "install-tree invoked systemctl: $(cat "$WORK/systemctl.log")"
[ -z "$(find "$HOME" -mindepth 1 -print -quit)" ] || fail "install-tree wrote to HOME"
pass "no systemctl call and no \$HOME writes"

# Unit substitution and verification.
unit="$FULL${PREFIX}/lib/systemd/user/screen-time.service"
grep -Fx "ExecStart=${PREFIX}/bin/screen-time daemon" "$unit" >/dev/null ||
    fail "unit ExecStart is not ${PREFIX}/bin/screen-time"
if command -v systemd-analyze >/dev/null 2>&1; then
    # Verify a tree whose ExecStart really exists on this host. The /usr unit
    # above is still checked byte-for-byte for its installed path.
    VERIFY_PREFIX="$WORK/verify-prefix"
    PREFIX="$VERIFY_PREFIX" packaging/install-tree.sh --all >/dev/null
    systemd-analyze verify "$VERIFY_PREFIX/lib/systemd/user/screen-time.service"
    pass "systemd-analyze verify"
else
    skip "systemd-analyze not installed"
fi

# Desktop, AppStream, man, and completion syntax.
if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "$FULL${PREFIX}/share/applications/rofi-hub.desktop"
    pass "desktop-file-validate"
elif [ "${REQUIRE_LINT:-0}" -eq 1 ]; then
    fail "desktop-file-validate is required but missing"
else
    skip "desktop-file-validate not installed"
fi

if command -v rofi >/dev/null 2>&1; then
    rofi_ok=1
    for theme in \
        "$FULL${PREFIX}/share/rofi-hub/themes/hub.rasi" \
        "$FULL${PREFIX}/share/rofi/screen-time.rasi"; do
        if ! output="$(rofi -no-config -theme "$theme" -dump-theme 2>&1)"; then
            # Some distro builds refuse to dump a theme without X/Wayland.
            if printf '%s' "$output" | grep -q "Failed to open display\|Connection has error"; then
                rofi_ok=0
                continue
            fi
            printf '%s\n' "$output" >&2
            fail "Rofi rejected the installed theme: $theme"
        fi
    done
    if [ "$rofi_ok" -eq 1 ]; then
        pass "rofi parses the installed hub and screen-time themes"
    else
        skip "rofi needs a display to dump themes"
    fi
else
    skip "rofi not installed"
fi

if command -v appstreamcli >/dev/null 2>&1; then
    appstreamcli validate --no-net "$FULL${PREFIX}/share/metainfo/rofi-hub.metainfo.xml"
    pass "appstreamcli validate"
elif [ "${REQUIRE_LINT:-0}" -eq 1 ]; then
    fail "appstreamcli is required but missing"
else
    skip "appstreamcli not installed"
fi

if command -v man >/dev/null 2>&1; then
    for page in packaging/man/*.1; do
        man --warnings -l "$page" >/dev/null || fail "man page has warnings: $page"
    done
    pass "man pages render without warnings"
else
    skip "man not installed"
fi

if command -v bash >/dev/null 2>&1; then
    for completion in packaging/completions/*.bash; do
        bash -n "$completion"
    done
    pass "bash completion syntax"
fi
if command -v zsh >/dev/null 2>&1; then
    for completion in packaging/completions/_*; do
        zsh -n "$completion"
    done
    pass "zsh completion syntax"
else
    skip "zsh not installed"
fi
if command -v fish >/dev/null 2>&1; then
    for completion in packaging/completions/*.fish; do
        fish -n "$completion"
    done
    pass "fish completion syntax"
else
    skip "fish not installed"
fi

for page in "$FULL${PREFIX}"/share/man/man1/*.1.gz; do
    gzip -t "$page" || fail "corrupt man page: $page"
done
pass "installed man pages are valid gzip"

# --- Staged runtime -------------------------------------------------------
export PATH="$FULL${PREFIX}/bin:$PATH"
export ROFI_HUB_PROVIDER_DIR="$FULL${PREFIX}/share/rofi-hub/providers"
export ROFI_HUB_THEME_FILE="$FULL${PREFIX}/share/rofi-hub/themes/hub.rasi"

version="$("$FULL${PREFIX}/bin/rofi-hub" --version)"
[ "$version" = "rofi-hub 0.2.1" ] || fail "unexpected hub version: $version"
pass "staged rofi-hub --version: $version"

"$FULL${PREFIX}/bin/rofi-hub" list --json >"$WORK/list.json"
python3 - "$WORK/list.json" "$FULL${PREFIX}/share/rofi-hub/providers" <<'PY'
import json
import pathlib
import sys
import tomllib

listed = {item["id"]: item for item in json.loads(pathlib.Path(sys.argv[1]).read_text())}
expected = {
    path.stem: tomllib.loads(path.read_text())
    for path in pathlib.Path(sys.argv[2]).glob("*.toml")
}
assert set(listed) == set(expected), (sorted(listed), sorted(expected))
assert all(item["available"] for item in listed.values()), listed
for provider_id, manifest in expected.items():
    assert listed[provider_id]["exec"] == manifest["exec"]
print(f"providers ok: {', '.join(sorted(listed))}")
PY
pass "staged hub discovers every installed provider"

tracker_version="$("$FULL${PREFIX}/bin/screen-time" --version)"
[ "$tracker_version" = "screen-time 0.5.0" ] || fail "unexpected tracker version: $tracker_version"
pass "staged screen-time --version: $tracker_version"

if "$FULL${PREFIX}/bin/screen-time" doctor --no-runtime --json >"$WORK/doctor.json" 2>"$WORK/doctor.err"; then
    pass "staged screen-time doctor --no-runtime"
else
    cat "$WORK/doctor.err" >&2
    fail "screen-time doctor --no-runtime failed"
fi

# The wrapper must resolve the packaged theme without SCREEN_TIME_THEME.
# A regular file at XDG_RUNTIME_DIR makes the generated-theme path fail, so
# the wrapper has to fall back to the installed data-directory theme.
mkdir -p "$WORK/wrapper-bin"
cat >"$WORK/wrapper-bin/rofi" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$@" >"$WORK/rofi-args"
exit 1
EOF
chmod 755 "$WORK/wrapper-bin/rofi"
: >"$WORK/runtime-is-a-file"
env -u SCREEN_TIME_THEME -u ROFI_THEME \
    PATH="$WORK/wrapper-bin:$PATH" \
    XDG_RUNTIME_DIR="$WORK/runtime-is-a-file" \
    XDG_DATA_HOME="$WORK/data" \
    XDG_DATA_DIRS="$FULL${PREFIX}/share" \
    "$FULL${PREFIX}/bin/rofi-screen-time" >/dev/null 2>&1 || true
grep -Fx -- "-theme" "$WORK/rofi-args" >/dev/null ||
    fail "rofi-screen-time did not pass a theme to rofi"
grep -Fx "${FULL}${PREFIX}/share/rofi/screen-time.rasi" "$WORK/rofi-args" >/dev/null ||
    fail "rofi-screen-time did not find the packaged data-directory theme"
pass "rofi-screen-time falls back to the installed theme"

# Every dmenu app must use the same installed chrome as Screen Time, including
# a direct launch with no ROFI_HUB_THEME_FILE.
mkdir -p "$WORK/app-bin" "$WORK/config/hypr"
cat >"$WORK/app-bin/rofi" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$@" >"\$ROFI_ARGS"
exit 1
EOF
cat >"$WORK/app-bin/cliphist" <<'EOF'
#!/usr/bin/env bash
printf '0\thello world\n'
EOF
cat >"$WORK/app-bin/wl-copy" <<'EOF'
#!/usr/bin/env bash
cat >/dev/null
EOF
chmod 755 "$WORK/app-bin"/*
printf '# --- Test ---\nbind = SUPER, E, exec, kitty\n' >"$WORK/config/hypr/hyprland.conf"
unset ROFI_THEME ROFI_HUB_THEME_FILE SCREEN_TIME_THEME
export XDG_DATA_DIRS="$FULL${PREFIX}/share"
export PATH="$WORK/app-bin:$PATH"
for app in hub calendar keybinds clipboard; do
    args="$WORK/theme-$app.args"
    : >"$args"
    ROFI_ARGS="$args" ROFI_BIN="$WORK/app-bin/rofi" \
        "$FULL${PREFIX}/bin/rofi-$app" >/dev/null 2>&1 || true
    theme="$(awk 'prev == "-theme" {print; exit} {prev=$0}' "$args")"
    [ "$theme" = "$FULL${PREFIX}/share/rofi-hub/themes/hub.rasi" ] ||
        fail "$app did not use the installed shared theme (got '${theme}')"
    grep -q 'padding: 20px;' "$theme" || fail "$app theme lacks Screen Time padding"
    grep -q 'border: 0px 0px 0px 2px;' "$theme" ||
        fail "$app theme lacks the Screen Time accent selection bar"
done
pass "hub, calendar, keybinds, and clipboard share the installed Screen Time-style chrome"

# A saved non-auto mode must generate the matching palette, even when the
# installed auto theme exists.
mkdir -p "$WORK/config/rofi-hub"
printf 'gruvbox\n' >"$WORK/config/rofi-hub/theme"
: >"$WORK/theme-saved.args"
ROFI_ARGS="$WORK/theme-saved.args" ROFI_BIN="$WORK/app-bin/rofi" \
    "$FULL${PREFIX}/bin/rofi-calendar" >/dev/null 2>&1 || true
theme="$(awk 'prev == "-theme" {print; exit} {prev=$0}' "$WORK/theme-saved.args")"
grep -q 'border-color: #fabd2f;' "$theme" ||
    fail "saved gruvbox mode did not generate the Gruvbox accent"
rm -f "$WORK/config/rofi-hub/theme"
pass "saved hub mode generates the matching Screen Time palette for direct launches"

# --- Variant installs -----------------------------------------------------
VARIANT="$WORK/without-screen-time"
PREFIX="$PREFIX" packaging/install-tree.sh --dry-run --without screen-time | LC_ALL=C sort >"$WORK/expected.without-screen-time"
stage "$VARIANT" --without screen-time
check_tree "$VARIANT" "$WORK/expected.without-screen-time" without-screen-time
[ ! -e "$VARIANT${PREFIX}/bin/screen-time" ] || fail "screen-time installed despite --without screen-time"
[ ! -e "$VARIANT${PREFIX}/share/rofi-hub/providers/screen-time.toml" ] ||
    fail "screen-time provider installed despite --without screen-time"
pass "--without screen-time skips the tracker"

VARIANT="$WORK/without-apps"
PREFIX="$PREFIX" packaging/install-tree.sh --dry-run --without apps | LC_ALL=C sort >"$WORK/expected.without-apps"
stage "$VARIANT" --without apps
check_tree "$VARIANT" "$WORK/expected.without-apps" without-apps
[ ! -e "$VARIANT${PREFIX}/bin/rofi-calendar" ] || fail "apps installed despite --without apps"
[ -e "$VARIANT${PREFIX}/share/rofi-hub/providers/screen-time.toml" ] ||
    fail "screen-time provider missing with --without apps"
pass "--without apps keeps screen-time only"

HUB_ONLY="$WORK/hub-only"
PREFIX="$PREFIX" packaging/install-tree.sh --dry-run --without apps --without screen-time | LC_ALL=C sort >"$WORK/expected.hub-only"
stage "$HUB_ONLY" --without apps --without screen-time
check_tree "$HUB_ONLY" "$WORK/expected.hub-only" hub-only
[ ! -d "$HUB_ONLY${PREFIX}/share/rofi-hub/providers" ] || fail "hub-only install created a provider directory"
pass "hub-only install has no providers"

# --- Negative tests -------------------------------------------------------
DRY_ROOT="$WORK/dry-run-root"
DESTDIR="$DRY_ROOT" PREFIX="$PREFIX" packaging/install-tree.sh --dry-run >/dev/null
[ ! -e "$DRY_ROOT" ] || fail "--dry-run created $DRY_ROOT"
pass "--dry-run writes nothing"

NOT_A_DIR="$WORK/not-a-directory"
: >"$NOT_A_DIR"
if DESTDIR="$NOT_A_DIR" PREFIX="$PREFIX" packaging/install-tree.sh --all >/dev/null 2>&1; then
    fail "install-tree accepted a DESTDIR that is a regular file"
fi
pass "invalid DESTDIR fails before installing"

# Reinstalling over an existing tree is idempotent and does not duplicate.
stage "$FULL" --all >/dev/null
check_tree "$FULL" packaging/MANIFEST full-reinstall

printf '\ntest-install-tree: all checks passed\n'
