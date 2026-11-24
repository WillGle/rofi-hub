#!/usr/bin/env bash
# Installer integration and fault-injection tests in a temp HOME.
#
# Everything runs as the current user with stub systemctl/rofi/notify-send/
# hyprctl commands. No real service, timer, or user state directory is touched.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-hub-installer-test.XXXXXX")"
PASS=0
FAIL=0

cleanup() {
    if [ "${KEEP_TEST_WORK:-0}" = "1" ]; then
        printf 'test workdir kept at %s\n' "$WORK" >&2
        return 0
    fi
    chmod -R u+rwX "$WORK" 2>/dev/null || true
    rm -rf -- "$WORK"
}
trap cleanup EXIT INT TERM

pass() {
    PASS=$((PASS + 1))
    printf 'ok   %s\n' "$1"
}

fail() {
    FAIL=$((FAIL + 1))
    printf 'FAIL %s\n' "$1" >&2
    printf 'FAIL %s\n' "$1" >>"${FAIL_LOG:-/dev/null}"
}

die() {
    printf 'test-installer.sh: %s\n' "$*" >&2
    exit 1
}

assert_eq() {
    local expected="$1" actual="$2" label="$3"
    if [ "$expected" = "$actual" ]; then
        pass "$label"
    else
        fail "$label (expected '$expected', got '$actual')"
    fi
}

assert_file() {
    if [ -e "$1" ]; then
        pass "$2"
    else
        fail "$2 (missing $1)"
    fi
}

assert_not_file() {
    if [ ! -e "$1" ]; then
        pass "$2"
    else
        fail "$2 (unexpected $1)"
    fi
}

assert_success() {
    local label="$1"
    local output
    shift
    if output="$("$@" 2>&1)"; then
        pass "$label"
    else
        fail "$label (command failed)"
        printf '%s\n' "$output" | tail -n 12 >&2
    fi
}

assert_failure() {
    local label="$1"
    shift
    if "$@"; then
        fail "$label (command unexpectedly succeeded)"
    else
        pass "$label"
    fi
}

assert_mode() {
    local expected="$1" path="$2" label="$3" actual
    actual="$(stat -c '%a' "$path" 2>/dev/null || printf 'missing')"
    assert_eq "$expected" "$actual" "$label"
}

sha() {
    sha256sum "$1" | awk '{print $1}'
}

# ---------------------------------------------------------------------------
# Environment and stubs
# ---------------------------------------------------------------------------

command -v minisign >/dev/null 2>&1 || die "minisign is required for installer tests"
command -v flock >/dev/null 2>&1 || die "flock is required"
command -v jq >/dev/null 2>&1 || die "jq is required"

mkdir -p "$WORK/bin" "$WORK/home" "$WORK/state" "$WORK/config" "$WORK/data" \
    "$WORK/run" "$WORK/systemctl" "$WORK/tmp"
chmod 700 "$WORK/run" "$WORK/state"
export HOME="$WORK/home"
export XDG_STATE_HOME="$WORK/state"
export XDG_CONFIG_HOME="$WORK/config"
export XDG_DATA_HOME="$WORK/data"
export XDG_RUNTIME_DIR="$WORK/run"
export HYPRLAND_INSTANCE_SIGNATURE="installer-test"
export SYSTEMCTL_STATE="$WORK/systemctl"
export PATH="$WORK/bin:$PATH"
export TMPDIR="$WORK/tmp"
export SCREEN_TIME_DISABLE_IDLE=1
export SCREEN_TIME_DISABLE_SLEEP=1

cat >"$WORK/bin/systemctl" <<'EOF'
#!/usr/bin/env bash
state="${SYSTEMCTL_STATE:?}"
[ "${1:-}" = "--user" ] && shift
case "${1:-}" in
    is-active)
        if [ -f "$state/active" ]; then printf 'active\n'; else printf 'inactive\n'; exit 3; fi
        ;;
    start|enable) touch "$state/active" ;;
    stop|disable) rm -f "$state/active" ;;
    show-environment|daemon-reload) exit 0 ;;
    *) exit 0 ;;
esac
EOF
chmod +x "$WORK/bin/systemctl"

cat >"$WORK/bin/rofi" <<'EOF'
#!/usr/bin/env bash
printf 'rofi 2.0.0\n'
EOF
chmod +x "$WORK/bin/rofi"

cat >"$WORK/bin/hyprctl" <<'EOF'
#!/usr/bin/env bash
printf 'Hyprland, 0.45.0\n'
EOF
chmod +x "$WORK/bin/hyprctl"

cat >"$WORK/bin/notify-send" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"${NOTIFY_LOG:-/dev/null}"
EOF
chmod +x "$WORK/bin/notify-send"
export NOTIFY_LOG="$WORK/notifications.log"
: >"$NOTIFY_LOG"

HUB_BIN="$HOME/.local/bin/rofi-hub"
SC_BIN="$HOME/.local/bin/screen-time"
SC_WRAPPER="$HOME/.local/bin/rofi-screen-time"
HUB_THEME="$XDG_DATA_HOME/rofi-hub/themes/hub.rasi"
PROVIDER="$XDG_DATA_HOME/rofi-hub/providers/screen-time.toml"
SC_THEME="$XDG_DATA_HOME/screen-time/rofi/screen-time.rasi"
SC_THEME_STATE="$XDG_CONFIG_HOME/rofi-hub/theme"
SC_CONFIG="$XDG_CONFIG_HOME/screen-time/config.toml"
UNIT="$XDG_CONFIG_HOME/systemd/user/screen-time.service"
STATE="$XDG_STATE_HOME/rofi-hub"
SC_STATE="$XDG_STATE_HOME/screen-time"
BACKUPS="$STATE/backups/updates"
VERSION_FILE="$STATE/installed-version"
COMPONENTS_FILE="$STATE/components"
DB="$SC_STATE/screen-time.db"

# Build once. The installer always installs release-shaped tarballs.
(cd "$ROOT" && cargo build --locked --package rofi-hub --package screen-time)

make_bundle() {
    local out="$1" version="$2"
    local stage dir
    stage="$(mktemp -d "$WORK/release.XXXXXX")"
    dir="$(dirname "$out")"
    mkdir -p "$stage/bin" "$stage/share/rofi-hub/providers" "$stage/share/rofi-hub/themes" \
        "$stage/share/rofi" "$stage/share/screen-time" "$stage/systemd" "$dir"
    install -m 755 "$ROOT/target/debug/rofi-hub" "$stage/bin/rofi-hub"
    install -m 755 "$ROOT/target/debug/screen-time" "$stage/bin/screen-time"
    install -m 755 "$ROOT/crates/screen-time/share/rofi-screen-time" "$stage/bin/rofi-screen-time"
    install -m 644 "$ROOT/themes/hub.rasi" "$stage/share/rofi-hub/themes/hub.rasi"
    install -m 644 "$ROOT/providers/screen-time.toml" "$stage/share/rofi-hub/providers/screen-time.toml"
    install -m 644 "$ROOT/crates/screen-time/share/rofi/screen-time.rasi" "$stage/share/rofi/screen-time.rasi"
    install -m 644 "$ROOT/crates/screen-time/share/screen-time/config.example.toml" "$stage/share/screen-time/config.example.toml"
    install -m 644 "$ROOT/crates/screen-time/systemd/screen-time.service" "$stage/systemd/screen-time.service"
    printf '%s\n' "$version" >"$stage/VERSION"
    install -m 644 "$ROOT/LICENSE" "$ROOT/README.md" "$ROOT/CHANGELOG.md" "$stage/"
    tar -czf "$out" -C "$stage" .
    (cd "$dir" && sha256sum "$(basename "$out")" >SHA256SUMS)
    rm -rf -- "$stage"
}

make_fixture() {
    local dir="$1" tag="$2" bundle="$3" key="$4"
    mkdir -p "$dir/$tag"
    cp "$bundle" "$dir/$tag/rofi-hub-x86_64-linux.tar.gz"
    install -m 755 "$ROOT/install.sh" "$dir/$tag/install.sh"
    install -m 755 "$ROOT/uninstall.sh" "$dir/$tag/uninstall.sh"
    install -m 644 "$ROOT/minisign.pub" "$dir/$tag/minisign.pub"
    (
        cd "$dir/$tag"
        sha256sum rofi-hub-x86_64-linux.tar.gz install.sh uninstall.sh minisign.pub >SHA256SUMS
        rm -f -- SHA256SUMS.minisig install.sh.minisig
        minisign -Sm SHA256SUMS -s "$key" -x SHA256SUMS.minisig >/dev/null
        minisign -Sm install.sh -s "$key" -x install.sh.minisig >/dev/null
    )
}

run_install() {
    bash "$ROOT/install.sh" --tarball "$BUNDLE" --allow-unsigned "$@"
}

seed_database() {
    timeout 3 "$SC_BIN" daemon >/dev/null 2>&1 || true
    python3 - "$DB" <<'PY'
import sqlite3, sys
path = sys.argv[1]
db = sqlite3.connect(path)
db.execute("INSERT INTO apps(key, first_seen_ts, last_seen_ts) VALUES('Code', 1000, 2000) ON CONFLICT(key) DO NOTHING")
db.execute("INSERT INTO sessions(date, app, start_ts, end_ts, seconds, state, confidence) VALUES('2026-11-20', 'Code', 1000, 1300, 300, 'closed', 'high')")
db.execute("INSERT INTO daily_app(date, app, seconds, sessions, first_ts, last_ts) VALUES('2026-11-20', 'Code', 300, 1, 1000, 1300)")
db.execute("INSERT INTO daily_slots(date, app, slot, seconds) VALUES('2026-11-20', 'Code', 1, 300)")
db.commit()
db.close()
PY
}

doctor_json() {
    "$SC_BIN" doctor --json --no-runtime
}

# ---------------------------------------------------------------------------
# Core install, theme, data safety, and backups
# ---------------------------------------------------------------------------

TEST_VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/p' "$ROOT/Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -n1)"
[ -n "$TEST_VERSION" ] || die "cannot read workspace version"
TEST_TAG="v${TEST_VERSION}"
BUNDLE="$WORK/release/rofi-hub-x86_64-linux.tar.gz"
make_bundle "$BUNDLE" "$TEST_VERSION"

STALE_INSTALL="$TMPDIR/rofi-hub-install.stale-test"
STALE_CHECK="$TMPDIR/rofi-hub-check.stale-test"
RECENT_STAGE="$TMPDIR/rofi-hub-install.recent-test"
mkdir -p "$STALE_INSTALL" "$STALE_CHECK" "$RECENT_STAGE"
touch -d '2 hours ago' "$STALE_INSTALL" "$STALE_CHECK"

assert_success "first install with theme" run_install --theme nord
assert_not_file "$STALE_INSTALL" "install removes stale staging"
assert_not_file "$STALE_CHECK" "install removes stale check staging"
assert_file "$RECENT_STAGE" "recent staging is kept"
rm -rf -- "$RECENT_STAGE"

assert_file "$HUB_BIN" "hub binary installed"
assert_file "$SC_BIN" "screen-time binary installed"
assert_file "$SC_WRAPPER" "screen-time wrapper installed"
assert_file "$HUB_THEME" "hub fallback theme installed"
assert_file "$PROVIDER" "provider manifest installed"
assert_file "$SC_THEME" "screen-time theme installed"
assert_file "$UNIT" "screen-time unit installed"
assert_eq "nord" "$(cat "$SC_THEME_STATE")" "hub theme choice is recorded"
assert_eq "nord" "$(sed -n '/^\[theme\]/,/^\[/p' "$SC_CONFIG" | sed -n 's/^mode = "\(.*\)"/\1/p' | head -n1)" "screen-time config is seeded from --theme"
assert_eq "$TEST_VERSION" "$(cat "$VERSION_FILE")" "installed version is recorded"
assert_eq "hub screen-time" "$(tr '\n' ' ' <"$COMPONENTS_FILE" | sed 's/ $//')" "component selection is recorded"
assert_file "$SYSTEMCTL_STATE/active" "service starts on first install"
assert_success "installed hub lists the provider" env ROFI_HUB_PROVIDER_DIR="$XDG_DATA_HOME/rofi-hub/providers" "$HUB_BIN" list

seed_database
before="$(doctor_json)"
before_sessions="$(printf '%s' "$before" | jq -r '.fingerprint.session_seconds')"
before_events="$(printf '%s' "$before" | jq -r '.fingerprint.counts.events')"
assert_eq "300" "$before_sessions" "seeded session seconds are visible"
config_sha="$(sha "$SC_CONFIG")"

assert_success "reinstall with data" run_install
after="$(doctor_json)"
assert_eq "$config_sha" "$(sha "$SC_CONFIG")" "config.toml is unchanged"
assert_eq "$before_sessions" "$(printf '%s' "$after" | jq -r '.fingerprint.session_seconds')" "session seconds survive an update"
assert_eq "$before_events" "$(printf '%s' "$after" | jq -r '.fingerprint.counts.events')" "event count survives an update"
assert_eq "ok" "$(printf '%s' "$after" | jq -r '.quick_check')" "database quick_check is ok"

backup="$(find "$BACKUPS" -mindepth 1 -maxdepth 1 -type d | sort | tail -n1)"
[ -n "$backup" ] || die "no update backup was created"
assert_file "$backup/manifest.json" "backup manifest exists"
assert_eq "verified" "$(jq -r '.status' "$backup/manifest.json")" "successful backup is verified"
assert_eq "300" "$(jq -r '.snapshot.fingerprint.session_seconds' "$backup/manifest.json")" "backup records the fingerprint"
assert_eq "hub screen-time" "$(jq -r '.components | join(" ")' "$backup/manifest.json")" "backup records components"
assert_mode 700 "$backup" "backup directory is private"
assert_mode 600 "$backup/manifest.json" "backup manifest is private"
assert_mode 600 "$backup/db/screen-time.db" "backup database is private"

REFERENCE="$WORK/reference-snapshot"
cp -a "$backup/db" "$REFERENCE"

# ---------------------------------------------------------------------------
# Fault injection and interrupted updates
# ---------------------------------------------------------------------------

for stage in hub hub-theme binary wrapper theme unit version data-check doctor; do
    assert_failure "fault after $stage rolls back" \
        env ROFI_HUB_INSTALLER_FAIL_AFTER="$stage" bash "$ROOT/install.sh" --tarball "$BUNDLE" --allow-unsigned >/dev/null 2>&1
    assert_eq "$TEST_VERSION" "$(cat "$VERSION_FILE")" "version restored after fault at $stage"
    assert_eq "300" "$("$SC_BIN" check-data --against "$REFERENCE" --json | jq -r '.actual_fingerprint.session_seconds')" "data restored after fault at $stage"
    assert_file "$SYSTEMCTL_STATE/active" "service restored after fault at $stage"
done

for stage in snapshot hub binary; do
    set +e
    env ROFI_HUB_INSTALLER_KILL_AFTER="$stage" bash "$ROOT/install.sh" --tarball "$BUNDLE" --allow-unsigned >/dev/null 2>&1
    kill_status=$?
    set -e
    assert_eq "137" "$kill_status" "simulated kill after $stage"
    assert_file "$STATE/updates/update-in-progress.json" "interrupted update ($stage) leaves in-progress state"
    assert_success "next run recovers interrupted update ($stage)" run_install
    assert_not_file "$STATE/updates/update-in-progress.json" "recovery ($stage) clears in-progress state"
    assert_eq "300" "$("$SC_BIN" check-data --against "$REFERENCE" --json | jq -r '.actual_fingerprint.session_seconds')" "data intact after interrupted recovery ($stage)"
done

# Protected user files.
printf '\n/* user edit */\n' >>"$SC_THEME"
assert_failure "modified theme refuses update" run_install >/dev/null 2>&1
assert_success "modified theme accepts --force" run_install --force
sed -i 's|^ExecStart=.*|ExecStart=/bin/true|' "$UNIT"
assert_failure "custom unit ExecStart refuses update" run_install >/dev/null 2>&1
if find "$STATE/backups/units" -type f -print -quit 2>/dev/null | grep -q .; then
    pass "custom unit edit is backed up"
else
    fail "custom unit edit is backed up"
fi
sed -i "s|^ExecStart=.*|ExecStart=${SC_BIN} daemon|" "$UNIT"
assert_success "reinstall after restoring the unit" run_install --force

# Legacy timer takeover.
touch "$XDG_CONFIG_HOME/systemd/user/screen-time-update-check.timer"
touch "$XDG_CONFIG_HOME/systemd/user/screen-time-update-check.service"
assert_success "install removes legacy update timers" run_install --no-service
assert_not_file "$XDG_CONFIG_HOME/systemd/user/screen-time-update-check.timer" "legacy check timer removed"
assert_not_file "$XDG_CONFIG_HOME/systemd/user/screen-time-update-check.service" "legacy check service removed"

# ---------------------------------------------------------------------------
# Hub-only, app-only, and selection persistence
# ---------------------------------------------------------------------------

assert_success "uninstall all components" bash "$ROOT/install.sh" --uninstall --yes
assert_not_file "$HUB_BIN" "uninstall removes the hub"
assert_not_file "$SC_BIN" "uninstall removes screen-time"
assert_not_file "$COMPONENTS_FILE" "uninstall clears the component list"
assert_file "$DB" "uninstall keeps the database"

assert_success "hub-only install" run_install --hub-only --no-service
assert_file "$HUB_BIN" "hub-only installs the hub"
assert_not_file "$SC_BIN" "hub-only does not install screen-time"
assert_not_file "$UNIT" "hub-only does not install the tracker unit"
assert_not_file "$PROVIDER" "hub-only does not install a provider"
assert_eq "hub" "$(cat "$COMPONENTS_FILE")" "hub-only records the selection"
assert_eq "$TEST_VERSION" "$(cat "$VERSION_FILE")" "hub-only records the version"

assert_success "bare reinstall preserves hub-only" run_install --no-service
assert_not_file "$SC_BIN" "bare reinstall stays hub-only"
assert_eq "hub" "$(cat "$COMPONENTS_FILE")" "bare reinstall keeps the recorded selection"

# The uninstall.sh wrapper delegates to install.sh.
WRAPPER_DIR="$WORK/uninstall-wrapper"
mkdir -p "$WRAPPER_DIR"
install -m 755 "$ROOT/install.sh" "$WRAPPER_DIR/install.sh"
install -m 755 "$ROOT/uninstall.sh" "$WRAPPER_DIR/uninstall.sh"
assert_success "uninstall.sh dry-run delegates" bash "$WRAPPER_DIR/uninstall.sh" --dry-run
assert_file "$HUB_BIN" "uninstall.sh dry-run leaves files in place"

# ---------------------------------------------------------------------------
# Signed releases, updates, and checks
# ---------------------------------------------------------------------------

KEY1="$WORK/key1.key"
KEY2="$WORK/key2.key"
minisign -G -W -p "$WORK/key1.pub" -s "$KEY1" >/dev/null
minisign -G -W -p "$WORK/key2.pub" -s "$KEY2" >/dev/null
PUB1="$(tail -n1 "$WORK/key1.pub")"

FIXTURE="$WORK/fixture"
NEW_BUNDLE="$WORK/release-0.2/rofi-hub-x86_64-linux.tar.gz"
LATEST_BUNDLE="$WORK/release-0.3/rofi-hub-x86_64-linux.tar.gz"
make_bundle "$NEW_BUNDLE" "0.2.0"
make_bundle "$LATEST_BUNDLE" "0.3.0"
make_fixture "$FIXTURE" "$TEST_TAG" "$BUNDLE" "$KEY1"
make_fixture "$FIXTURE" "v0.2.0" "$NEW_BUNDLE" "$KEY1"
make_fixture "$FIXTURE" "v0.3.0" "$LATEST_BUNDLE" "$KEY1"

export ROFI_HUB_TRUSTED_KEYS="test-key-1:$PUB1"
export ROFI_HUB_RELEASES_BASE="file://$FIXTURE"

assert_success "signed hub-only update" \
    env ROFI_HUB_LATEST_TAG=v0.2.0 bash "$ROOT/install.sh" --update --no-service
assert_eq "0.2.0" "$(cat "$VERSION_FILE")" "update records the new version"
assert_not_file "$SC_BIN" "hub-only update does not add screen-time"
assert_eq "hub" "$(cat "$COMPONENTS_FILE")" "hub-only update keeps the selection"

set +e
ROFI_HUB_LATEST_TAG=v0.2.0 bash "$ROOT/install.sh" --check --json >"$WORK/check-current.json"
current_status=$?
set -e
assert_eq "0" "$current_status" "check exits 0 when current"
assert_eq "test-key-1" "$(jq -r '.signature_key_id' "$WORK/check-current.json")" "check verifies the signed release"
assert_eq "0.2.0" "$(jq -r '.installed_version' "$WORK/check-current.json")" "check reports the installed version"

set +e
ROFI_HUB_LATEST_TAG=v0.3.0 bash "$ROOT/install.sh" --check --json >"$WORK/check-new.json"
new_status=$?
set -e
assert_eq "10" "$new_status" "check reports an available update"
assert_eq "0.3.0" "$(jq -r '.latest_version' "$WORK/check-new.json")" "check reports the latest version"

# Up to date must exit before any download.
assert_success "up-to-date update makes no download" \
    env ROFI_HUB_LATEST_TAG=v0.2.0 ROFI_HUB_RELEASES_BASE="file://$WORK/missing" \
    bash "$ROOT/install.sh" --update --no-service

# Tampered asset and wrong key fail closed.
TAMPERED="$WORK/tampered"
cp -a "$FIXTURE" "$TAMPERED"
printf 'tampered' >>"$TAMPERED/v0.3.0/rofi-hub-x86_64-linux.tar.gz"
assert_failure "tampered asset is refused" \
    env ROFI_HUB_LATEST_TAG=v0.3.0 ROFI_HUB_RELEASES_BASE="file://$TAMPERED" \
    bash "$ROOT/install.sh" --version v0.3.0 --no-service >/dev/null 2>&1

WRONGKEY="$WORK/wrongkey"
make_fixture "$WRONGKEY" "v0.3.0" "$LATEST_BUNDLE" "$KEY2"
assert_failure "wrong signing key is refused" \
    env ROFI_HUB_LATEST_TAG=v0.3.0 ROFI_HUB_RELEASES_BASE="file://$WRONGKEY" \
    bash "$ROOT/install.sh" --version v0.3.0 --no-service >/dev/null 2>&1

# ---------------------------------------------------------------------------
# App-only install and uninstall/purge
# ---------------------------------------------------------------------------

assert_success "uninstall hub only" bash "$ROOT/install.sh" --uninstall --hub-only --yes
assert_not_file "$HUB_BIN" "hub-only uninstall removes the hub"

assert_success "app-only install" run_install --app screen-time --no-hub --no-service
assert_file "$SC_BIN" "app-only installs screen-time"
assert_file "$PROVIDER" "app-only installs the provider manifest"
assert_not_file "$HUB_BIN" "app-only does not install the hub"
assert_eq "screen-time" "$(cat "$COMPONENTS_FILE")" "app-only records the selection"

assert_success "bare reinstall preserves app-only" run_install --no-service
assert_not_file "$HUB_BIN" "bare reinstall stays app-only"

assert_success "app-only uninstall keeps the database" bash "$ROOT/install.sh" --uninstall --no-hub --app screen-time --yes
assert_not_file "$SC_BIN" "app-only uninstall removes screen-time"
assert_file "$DB" "app-only uninstall keeps the database"

assert_success "reinstall app-only for purge" run_install --app screen-time --no-hub --no-service
assert_success "purge removes state and history" bash "$ROOT/install.sh" --uninstall --no-hub --app screen-time --purge --yes
assert_not_file "$DB" "purge removes the database"
assert_not_file "$STATE" "purge removes hub update state"

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
