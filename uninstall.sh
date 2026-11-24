#!/usr/bin/env bash
# Remove rofi-hub, or selected components, by delegating to the same
# conservative installer used for installs and updates.
set -Eeuo pipefail

self="$(readlink -f -- "${BASH_SOURCE[0]}")"
dir="$(dirname -- "$self")"
installer=""
for candidate in "${dir}/install.sh" \
    "${XDG_STATE_HOME:-${HOME}/.local/state}/rofi-hub/install.sh"; do
    if [ -f "$candidate" ]; then
        installer="$candidate"
        break
    fi
done

if [ -z "$installer" ]; then
    printf 'uninstall.sh: cannot find install.sh next to this script or in the rofi-hub state directory\n' >&2
    printf 'Download install.sh from the release and run: bash install.sh --uninstall\n' >&2
    exit 1
fi

exec bash "$installer" --uninstall "$@"
