#!/usr/bin/env bash
# Remove rofi-apps installed by install.sh. Keeps state semantics identical:
# only files recorded for the named app are removed.
set -Eeuo pipefail

SELF_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

if [[ -x "$SELF_DIR/install.sh" ]]; then
    exec "$SELF_DIR/install.sh" --uninstall "$@"
fi
if [[ -x "$SELF_DIR/../install.sh" ]]; then
    exec "$SELF_DIR/../install.sh" --uninstall "$@"
fi
if [[ -x "$SELF_DIR/rofi-hub-install" ]]; then
    exec "$SELF_DIR/rofi-hub-install" --uninstall "$@"
fi

printf 'uninstall.sh: cannot find install.sh next to %s\n' "$SELF_DIR" >&2
exit 1
