#!/usr/bin/env bash
# Install rofi-hub into a PREFIX/DESTDIR tree.
#
# This is the single install source for distro packages. It never touches
# $HOME, never calls systemctl, and never runs install.sh. A package manager
# stages a complete tree with DESTDIR and owns the resulting files.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

PREFIX="${PREFIX:-/usr/local}"
DESTDIR="${DESTDIR:-}"
BINDIR="${BINDIR:-${PREFIX}/bin}"
DATADIR="${DATADIR:-${PREFIX}/share}"
SYSCONFDIR="${SYSCONFDIR:-${PREFIX}/etc}"
SYSTEMD_USERUNITDIR="${SYSTEMD_USERUNITDIR:-${PREFIX}/lib/systemd/user}"
MANDIR="${MANDIR:-${DATADIR}/man}"
LICENSEDIR="${LICENSEDIR:-${DATADIR}/licenses/rofi-hub}"
DOCDIR="${DOCDIR:-${DATADIR}/doc/rofi-hub}"
DESKTOPDIR="${DESKTOPDIR:-${DATADIR}/applications}"
ICONDIR="${ICONDIR:-${DATADIR}/icons/hicolor/scalable/apps}"
APPDATADIR="${APPDATADIR:-${DATADIR}/metainfo}"
BASHCOMPLETIONSDIR="${BASHCOMPLETIONSDIR:-${DATADIR}/bash-completion/completions}"
ZSHCOMPLETIONSDIR="${ZSHCOMPLETIONSDIR:-${DATADIR}/zsh/site-functions}"
FISHCOMPLETIONSDIR="${FISHCOMPLETIONSDIR:-${DATADIR}/fish/vendor_completions.d}"
CONTRIBDIR="${CONTRIBDIR:-${DATADIR}/rofi-hub/contrib}"

# A release tarball is self-contained: bin/, share/, packaging/, and VERSION
# at the top level. A source checkout builds into target/release and keeps
# providers/themes in the repository layout. Both layouts feed the same
# installer.
RELEASE_LAYOUT=0
if [ -x "${ROOT}/bin/rofi-hub" ] && [ -f "${ROOT}/VERSION" ] && [ ! -d "${ROOT}/crates" ]; then
    RELEASE_LAYOUT=1
fi
if [ "$RELEASE_LAYOUT" -eq 1 ]; then
    TARGETDIR="${TARGETDIR:-${ROOT}/bin}"
    PROVIDERS_SRC="${ROOT}/share/rofi-hub/providers"
    HUB_THEME_SRC="${ROOT}/share/rofi-hub/themes/hub.rasi"
    SC_THEME_SRC="${ROOT}/share/rofi/screen-time.rasi"
    SC_CONFIG_SRC="${ROOT}/share/screen-time/config.example.toml"
    WRAPPER_SRC="${ROOT}/bin/rofi-screen-time"
else
    TARGETDIR="${TARGETDIR:-${CARGO_TARGET_DIR:-${ROOT}/target}/release}"
    PROVIDERS_SRC="${ROOT}/providers"
    HUB_THEME_SRC="${ROOT}/themes/hub.rasi"
    SC_THEME_SRC="${ROOT}/crates/screen-time/share/rofi/screen-time.rasi"
    SC_CONFIG_SRC="${ROOT}/crates/screen-time/share/screen-time/config.example.toml"
    WRAPPER_SRC="${ROOT}/crates/screen-time/share/rofi-screen-time"
fi
UNIT_TEMPLATE="${ROOT}/packaging/systemd/screen-time.service.in"

WITH_APPS=1
WITH_SCREEN_TIME=1
DRY_RUN=0
VERBOSE=0
SELECTED=()

usage() {
    cat <<'EOF'
Usage: packaging/install-tree.sh [OPTIONS]

Install the rofi-hub tree into $DESTDIR$PREFIX. PREFIX defaults to
/usr/local; package recipes set PREFIX=/usr and DESTDIR to the package root.

Options:
  --all                    install the hub, apps, and screen-time (default)
  --without COMPONENT      skip `apps` (calendar/keybinds/clipboard) or
                           `screen-time`; may be repeated
  --dry-run                print `MODE PATH` lines without writing anything
  --verbose, -v            print every installed file
  -h, --help               show this help

Environment:
  PREFIX, DESTDIR, BINDIR, DATADIR, SYSCONFDIR, SYSTEMD_USERUNITDIR,
  MANDIR, LICENSEDIR, DOCDIR, DESKTOPDIR, ICONDIR, APPDATADIR,
  BASHCOMPLETIONSDIR, ZSHCOMPLETIONSDIR, FISHCOMPLETIONSDIR, CONTRIBDIR,
  TARGETDIR (default: target/release)

The script is safe to run as a normal user. It never writes to $HOME.
EOF
}

die() {
    printf 'install-tree.sh: %s\n' "$*" >&2
    exit 1
}

note() {
    if [ "$VERBOSE" -eq 1 ]; then
        printf 'install-tree.sh: %s\n' "$*" >&2
    fi
}

add_without() {
    case "$1" in
        apps|app)
            WITH_APPS=0
            ;;
        screen-time|screentime)
            WITH_SCREEN_TIME=0
            ;;
        *)
            die "unknown component '$1'; expected 'apps' or 'screen-time'"
            ;;
    esac
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --all)
            WITH_APPS=1
            WITH_SCREEN_TIME=1
            ;;
        --without)
            [ "$#" -ge 2 ] || die "--without requires a component"
            shift
            add_without "$1"
            ;;
        --without=*)
            add_without "${1#--without=}"
            ;;
        --dry-run)
            DRY_RUN=1
            ;;
        --verbose|-v)
            VERBOSE=1
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            die "unknown argument '$1' (try --help)"
            ;;
    esac
    shift
done

if [ "$WITH_APPS" -eq 1 ]; then
    SELECTED+=(calendar keybinds clipboard)
fi
if [ "$WITH_SCREEN_TIME" -eq 1 ]; then
    SELECTED+=(screen-time)
fi

# Everything this invocation intends to install. Keeping the list in one
# place lets --dry-run double as a manifest generator.
FILES=()

add_file() {
    local mode="$1"
    local src="$2"
    local dest="$3"
    FILES+=("${mode}|${src}|${dest}")
}

add_asset() {
    local mode="$1"
    local src="$2"
    local dest="$3"
    [ -f "$src" ] || die "missing source file: $src"
    add_file "$mode" "$src" "$dest"
}

add_binary() {
    local name="$1"
    local src="${TARGETDIR}/${name}"
    if [ "$DRY_RUN" -eq 0 ]; then
        [ -x "$src" ] || die "missing executable: $src (build with: cargo build --release --locked --workspace)"
    fi
    add_file 755 "$src" "${BINDIR}/${name}"
}

# Binaries.
add_binary rofi-hub
for app in "${SELECTED[@]}"; do
    if [ "$app" != "screen-time" ]; then
        add_binary "rofi-${app}"
    fi
done
if [ "$WITH_SCREEN_TIME" -eq 1 ]; then
    add_binary screen-time
    add_asset 755 "$WRAPPER_SRC" "${BINDIR}/rofi-screen-time"
fi

# Provider manifests and themes.
for app in "${SELECTED[@]}"; do
    add_asset 644 "${PROVIDERS_SRC}/${app}.toml" "${DATADIR}/rofi-hub/providers/${app}.toml"
done
add_asset 644 "$HUB_THEME_SRC" "${DATADIR}/rofi-hub/themes/hub.rasi"
if [ "$WITH_SCREEN_TIME" -eq 1 ]; then
    add_asset 644 "$SC_THEME_SRC" "${DATADIR}/rofi/screen-time.rasi"
    add_asset 644 "$SC_CONFIG_SRC" "${DATADIR}/screen-time/config.example.toml"
    add_asset 644 "$UNIT_TEMPLATE" "${SYSTEMD_USERUNITDIR}/screen-time.service"
    add_asset 755 "${ROOT}/packaging/openrc/screen-time" "${CONTRIBDIR}/openrc/screen-time"
    add_asset 755 "${ROOT}/packaging/runit/screen-time/run" "${CONTRIBDIR}/runit/screen-time/run"
fi

# Desktop integration.
add_asset 644 "${ROOT}/packaging/rofi-hub.desktop" "${DESKTOPDIR}/rofi-hub.desktop"
add_asset 644 "${ROOT}/packaging/rofi-hub.svg" "${ICONDIR}/rofi-hub.svg"
add_asset 644 "${ROOT}/packaging/rofi-hub.metainfo.xml" "${APPDATADIR}/rofi-hub.metainfo.xml"

# Man pages and completions follow the installed commands.
MAN_PAGES=(rofi-hub)
if [ "$WITH_APPS" -eq 1 ]; then
    MAN_PAGES+=(rofi-calendar rofi-keybinds rofi-clipboard)
fi
if [ "$WITH_SCREEN_TIME" -eq 1 ]; then
    MAN_PAGES+=(screen-time rofi-screen-time)
fi
for page in "${MAN_PAGES[@]}"; do
    add_asset 644 "${ROOT}/packaging/man/${page}.1" "${MANDIR}/man1/${page}.1"
done

BASH_COMPLETIONS=(rofi-hub)
ZSH_COMPLETIONS=(_rofi-hub)
FISH_COMPLETIONS=(rofi-hub.fish)
if [ "$WITH_SCREEN_TIME" -eq 1 ]; then
    BASH_COMPLETIONS+=(screen-time)
    ZSH_COMPLETIONS+=(_screen-time)
    FISH_COMPLETIONS+=(screen-time.fish)
fi
for completion in "${BASH_COMPLETIONS[@]}"; do
    add_asset 644 "${ROOT}/packaging/completions/${completion}.bash" \
        "${BASHCOMPLETIONSDIR}/${completion}"
done
for completion in "${ZSH_COMPLETIONS[@]}"; do
    add_asset 644 "${ROOT}/packaging/completions/${completion}" \
        "${ZSHCOMPLETIONSDIR}/${completion}"
done
for completion in "${FISH_COMPLETIONS[@]}"; do
    add_asset 644 "${ROOT}/packaging/completions/${completion}" \
        "${FISHCOMPLETIONSDIR}/${completion}"
done

# License and documentation.
add_asset 644 "${ROOT}/LICENSE" "${LICENSEDIR}/LICENSE"
add_asset 644 "${ROOT}/README.md" "${DOCDIR}/README.md"
add_asset 644 "${ROOT}/CHANGELOG.md" "${DOCDIR}/CHANGELOG.md"
for doc in ARCHITECTURE.md MIGRATING.md PACKAGING.md; do
    add_asset 644 "${ROOT}/docs/${doc}" "${DOCDIR}/${doc}"
done

stage_path() {
    printf '%s%s' "$DESTDIR" "$1"
}

install_one() {
    local mode="$1"
    local src="$2"
    local dest="$3"
    local target
    target="$(stage_path "$dest")"
    mkdir -p -- "$(dirname -- "$target")"
    if [[ "$dest" == "${MANDIR}/man1/"* ]]; then
        gzip -9n -c -- "$src" >"${target}.gz"
        chmod 644 "${target}.gz"
        target="${target}.gz"
    else
        install -m "$mode" -- "$src" "$target"
    fi
    note "installed ${target}"
}

if [ "$DRY_RUN" -eq 1 ]; then
    for record in "${FILES[@]}"; do
        IFS='|' read -r mode src dest <<<"$record"
        if [[ "$dest" == "${MANDIR}/man1/"* ]]; then
            dest="${dest}.gz"
        fi
        printf '%s %s\n' "$mode" "${dest}"
    done
    exit 0
fi

# Preflight the destination before writing a single file. A relative DESTDIR
# is accepted for compatibility, but it is resolved from the current working
# directory exactly like `install -D` would resolve it.
if [ -n "$DESTDIR" ]; then
    [ ! -e "$DESTDIR" ] || [ -d "$DESTDIR" ] || die "DESTDIR is not a directory: $DESTDIR"
    mkdir -p -- "$DESTDIR" || die "cannot create DESTDIR: $DESTDIR"
    probe="${DESTDIR}/.rofi-hub-install-probe.$$"
    if ! ( : >"$probe" ) 2>/dev/null; then
        die "DESTDIR is not writable: $DESTDIR"
    fi
    rm -f -- "$probe"
fi

installed=0
for record in "${FILES[@]}"; do
    IFS='|' read -r mode src dest <<<"$record"
    install_one "$mode" "$src" "$dest"
    installed=$((installed + 1))
done

# The installed unit must point at the real binary, not the build tree.
if [ "$WITH_SCREEN_TIME" -eq 1 ]; then
    unit="$(stage_path "${SYSTEMD_USERUNITDIR}/screen-time.service")"
    tmp="${unit}.tmp.$$"
    sed -e "s|@BINDIR@|${BINDIR}|g" -e "s|@SYSCONFDIR@|${SYSCONFDIR}|g" \
        "$UNIT_TEMPLATE" >"$tmp"
    chmod 644 "$tmp"
    mv -f -- "$tmp" "$unit"
fi

printf 'install-tree.sh: installed %d files into %s\n' "$installed" "${DESTDIR:-/}" >&2
