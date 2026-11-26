#!/usr/bin/env bash
# Cross-distro `install.sh` smoke test in containers.
#
# Uses a shell-script app payload so the test exercises the installer itself,
# not the host glibc ABI. Skips cleanly when no container runtime is available.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/rofi-apps-matrix.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT INT TERM
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); printf 'ok   %s\n' "$1"; }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %s\n' "$1" >&2; }

if [[ "${ROFI_APPS_SKIP_MATRIX:-0}" = 1 ]]; then
    printf 'test-install-matrix: skipped by ROFI_APPS_SKIP_MATRIX=1\n'
    exit 0
fi

DOCKER=""
for candidate in docker podman; do
    if command -v "$candidate" >/dev/null 2>&1 && "$candidate" info >/dev/null 2>&1; then
        DOCKER="$candidate"
        break
    fi
done
if [[ -z "$DOCKER" ]]; then
    printf 'test-install-matrix: no container runtime; skipping\n' >&2
    exit 0
fi

DIST="$WORK/dist"
mkdir -p "$DIST/stage/bin" "$DIST/stage/share/rofi" \
    "$DIST/stage/share/rofi-apps/providers" "$DIST/stage/share/rofi-apps/apps/calendar"
cat >"$DIST/stage/app.toml" <<'EOF'
id = "calendar"
version = "1.2.0"
bin = "rofi-calendar"
provider = "providers/calendar.toml"
requires = ["sh"]
optional_requires = []
service = false
EOF
cat >"$DIST/stage/bin/rofi-calendar" <<'EOF'
#!/bin/sh
printf 'matrix-ok\n'
EOF
chmod 755 "$DIST/stage/bin/rofi-calendar"
cp "$DIST/stage/app.toml" "$DIST/stage/share/rofi-apps/apps/calendar/app.toml"
printf '* { }\n' >"$DIST/stage/share/rofi/calendar.rasi"
cat >"$DIST/stage/share/rofi-apps/providers/calendar.toml" <<'EOF'
id = "calendar"
name = "Calendar"
description = "Matrix probe"
exec = ["sh", "-c", "true"]
version = "1.2.0"
EOF
tar -czf "$DIST/rofi-calendar-1.2.0-x86_64.tar.gz" -C "$DIST/stage" .
sha="$(sha256sum "$DIST/rofi-calendar-1.2.0-x86_64.tar.gz" | awk '{print $1}')"
cat >"$DIST/index.json" <<EOF
{
  "schema": 1,
  "installer": { "version": "0.3.0" },
  "apps": [
    {
      "id": "calendar",
      "version": "1.2.0",
      "url": "rofi-calendar-1.2.0-x86_64.tar.gz",
      "sha256": "$sha",
      "requires": ["sh"]
    }
  ]
}
EOF

IMAGES=(debian:12 ubuntu:24.04 fedora:latest archlinux:base-devel)
for image in "${IMAGES[@]}"; do
    run_image="$image"
    if ! "$DOCKER" image inspect "$run_image" >/dev/null 2>&1; then
        if ! "$DOCKER" pull "$run_image" >/dev/null 2>&1; then
            # Docker Hub tokens expire; fall back to the public GCR mirror.
            case "$image" in
                */*) mirror="$image" ;;
                *) mirror="mirror.gcr.io/library/$image" ;;
            esac
            if "$DOCKER" image inspect "$mirror" >/dev/null 2>&1 ||
                "$DOCKER" pull "$mirror" >/dev/null 2>&1; then
                run_image="$mirror"
            else
                printf 'test-install-matrix: cannot pull %s; skipping\n' "$image" >&2
                continue
            fi
        fi
    fi
    # shellcheck disable=SC2016
    container_script='
set -eu
export HOME=/tmp/home
export XDG_STATE_HOME=/tmp/state
export XDG_CONFIG_HOME=/tmp/config
export XDG_DATA_HOME=/tmp/data
mkdir -p "$HOME" "$XDG_STATE_HOME" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME"
bash /install.sh --index /dist/index.json --allow-unsigned --app calendar --no-service
test -x "$HOME/.local/bin/rofi-calendar"
test -f "$XDG_DATA_HOME/rofi-apps/providers/calendar.toml"
test -f "$XDG_DATA_HOME/rofi/calendar.rasi"
test -f "$XDG_STATE_HOME/rofi-apps/installed/calendar"
test "$("$HOME/.local/bin/rofi-calendar")" = "matrix-ok"
test "$(stat -c %a "$XDG_STATE_HOME/rofi-apps/installed/calendar")" = "600"
bash /install.sh --index /dist/index.json --allow-unsigned --uninstall calendar
test ! -e "$HOME/.local/bin/rofi-calendar"
test ! -e "$XDG_STATE_HOME/rofi-apps/installed/calendar"
printf "container-ok\n"
'
    if "$DOCKER" run --rm --pull=never \
        -v "$ROOT/install.sh:/install.sh:ro" \
        -v "$DIST:/dist:ro" \
        "$run_image" sh -c "$container_script" >"$WORK/$image.log" 2>&1; then
        pass "DIST cross-distro install.sh on $image"
    else
        fail "DIST cross-distro install.sh on $image"
        tail -n 15 "$WORK/$image.log" >&2
    fi
done

if ((PASS == 0)); then
    printf 'test-install-matrix: no local images available; skipping\n' >&2
    exit 0
fi

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
