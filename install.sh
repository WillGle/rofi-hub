#!/usr/bin/env bash
# rofi-apps installer.
#
# Installs independent, signed app tarballs listed in a release index. The
# installer only copies, verifies, records, and removes files. It never
# installs runtime dependencies, never changes PATH, never writes themes, and
# never launches an app.
set -Eeuo pipefail

REPO="WillGle/rofi-hub"
INSTALLER_VERSION="0.3.0"
DEFAULT_RELEASES_BASE="https://github.com/${REPO}/releases/latest/download"

PREFIX="${ROFI_APPS_PREFIX:-$HOME/.local}"
INDEX_SOURCE="${ROFI_APPS_INDEX:-$DEFAULT_RELEASES_BASE/index.json}"
PUBKEY="${ROFI_APPS_PUBKEY:-}"
MODE=""
APPS=()
ALL=0
NO_SERVICE=0
ALLOW_UNSIGNED=0
DRY_RUN=0
VERBOSE=0

XDG_DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
XDG_STATE_HOME="${XDG_STATE_HOME:-$HOME/.local/state}"
XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}"

STATE_ROOT="$XDG_STATE_HOME/rofi-apps"
INSTALLED_DIR="$STATE_ROOT/installed"
DATA_ROOT="$XDG_DATA_HOME"
BIN_DIR="$PREFIX/bin"
PROVIDER_DIR="$DATA_ROOT/rofi-apps/providers"
SYSTEMD_USER_DIR="$XDG_CONFIG_HOME/systemd/user"

SELF_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ARCH="$(printf '%s' "${ROFI_APPS_ARCH:-$(uname -m 2>/dev/null || printf unknown)}" |
    tr '[:upper:]' '[:lower:]')"
case "$ARCH" in
    amd64) ARCH=x86_64 ;;
    arm64) ARCH=aarch64 ;;
esac
WORK=""
INDEX_FILE=""
INDEX_BASE=""
LOCK_DIR=""
CURRENT_APP=""
BACKUP_DIR=""
NEW_FILES=()
BACKUP_FILES=()
declare -A PREFLIGHT_DIR=()

usage() {
    cat <<'EOF'
Usage: install.sh <command> [options]

Install, update, list, or remove independent rofi-apps from a signed release
index. No root and no runtime dependency installation.

Commands:
  --app ID[,ID...]        install one or more apps from the index
  --all                   install every non-optional app in the index
  --list                  list apps in the index and installation state
  --update [ID[,ID...]]   update the named apps, or every installed app
  --uninstall [ID[,...]]  remove the named apps, or every installed app

Options:
  --index PATH|URL        release index (default: latest GitHub release)
  --prefix PATH           binary prefix (default: ~/.local)
  --no-service            install files without enabling an app service
  --allow-unsigned        skip signature checks (local testing only)
  --pubkey PATH           minisign public key for signature verification
  --dry-run               print actions without changing files
  --verbose               print each installed file
  -h, --help              show this help
  --version               print the installer version

Environment:
  ROFI_APPS_INDEX         default index source
  ROFI_APPS_PREFIX        default install prefix
  ROFI_APPS_PUBKEY        default minisign public key
  ROFI_APPS_ARCH          override the detected architecture (testing only)

The installer refuses to write into a Nix-managed path. Prerequisites are
checked with `command -v` from each app.toml `requires` list; missing tools
abort the install instead of being installed automatically.
EOF
}

die() {
    printf 'install.sh: %s\n' "$*" >&2
    exit 1
}

info() {
    printf '%s\n' "$*"
}

verbose() {
    if ((VERBOSE)); then
        printf '%s\n' "$*"
    fi
}

warn() {
    printf 'install.sh: warning: %s\n' "$*" >&2
}

cleanup() {
    if [[ -n "$WORK" && -d "$WORK" ]]; then
        rm -rf -- "$WORK"
    fi
    if [[ -n "$LOCK_DIR" && -d "$LOCK_DIR" ]]; then
        rmdir -- "$LOCK_DIR" 2>/dev/null || true
    fi
}

restore_backup() {
    local target rel
    for target in "${NEW_FILES[@]}"; do
        [[ -n "$target" ]] && rm -f -- "$target"
    done
    for rel in "${BACKUP_FILES[@]}"; do
        [[ -n "$rel" ]] || continue
        target="/${rel//__//}"
        mkdir -p -- "$(dirname -- "$target")"
        cp -a -- "$BACKUP_DIR/$rel" "$target"
    done
}

on_exit() {
    local status=$?
    if [[ -n "$CURRENT_APP" && -d "${BACKUP_DIR:-}" ]]; then
        warn "install failed for $CURRENT_APP; restoring previous files"
        restore_backup || true
        rm -f -- "${INSTALLED_DIR:-}/.in-progress-$CURRENT_APP" 2>/dev/null || true
    fi
    cleanup
    exit "$status"
}

trap on_exit EXIT

add_apps() {
    local raw="$1"
    [[ -n "$raw" ]] || die "--app requires at least one id"
    local item
    local parts=()
    IFS=',' read -r -a parts <<<"$raw"
    for item in "${parts[@]}"; do
        item="${item//[[:space:]]/}"
        [[ -n "$item" ]] || continue
        [[ "$item" =~ ^[a-z0-9][a-z0-9_-]*$ ]] || die "invalid app id: $item"
        APPS+=("$item")
    done
}

while (($#)); do
    case "$1" in
        --app)
            shift
            (($#)) || die "--app requires a value"
            add_apps "$1"
            ;;
        --app=*) add_apps "${1#*=}" ;;
        --all) ALL=1 ;;
        --list) MODE=list ;;
        --update) MODE=update ;;
        --uninstall) MODE=uninstall ;;
        --index)
            shift
            (($#)) || die "--index requires a value"
            INDEX_SOURCE="$1"
            ;;
        --index=*) INDEX_SOURCE="${1#*=}" ;;
        --prefix)
            shift
            (($#)) || die "--prefix requires a value"
            PREFIX="$1"
            ;;
        --prefix=*) PREFIX="${1#*=}" ;;
        --no-service) NO_SERVICE=1 ;;
        --allow-unsigned) ALLOW_UNSIGNED=1 ;;
        --pubkey)
            shift
            (($#)) || die "--pubkey requires a value"
            PUBKEY="$1"
            ;;
        --pubkey=*) PUBKEY="${1#*=}" ;;
        --dry-run) DRY_RUN=1 ;;
        --verbose) VERBOSE=1 ;;
        --version)
            printf '%s\n' "$INSTALLER_VERSION"
            exit 0
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        --theme)
            die "--theme was removed; themes belong to the user/theme module" ;;
        --)
            shift
            while (($#)); do
                add_apps "$1"
                shift
            done
            break
            ;;
        -*)
            die "unknown option: $1 (use --help)"
            ;;
        *)
            add_apps "$1"
            ;;
    esac
    shift
done

MODE="${MODE:-install}"
BIN_DIR="$PREFIX/bin"

if [[ -z "$PUBKEY" ]]; then
    for candidate in \
        "$SELF_DIR/minisign.pub" \
        "$SELF_DIR/../share/rofi-hub-installer/minisign.pub" \
        "$SELF_DIR/../minisign.pub"; do
        if [[ -f "$candidate" ]]; then
            PUBKEY="$candidate"
            break
        fi
    done
fi

require_tools() {
    local tool
    for tool in tar sha256sum awk sed grep cut find uname; do
        command -v "$tool" >/dev/null 2>&1 || die "required tool not found: $tool"
    done
    if [[ "$INDEX_SOURCE" == http://* || "$INDEX_SOURCE" == https://* ]]; then
        command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 ||
            die "curl or wget is required to download releases"
    fi
    if ((!ALLOW_UNSIGNED)); then
        command -v minisign >/dev/null 2>&1 || die "minisign is required to verify releases"
        [[ -n "$PUBKEY" && -f "$PUBKEY" ]] || die "no trusted minisign public key found"
    fi
}

fetch() {
    local url="$1"
    local dest="$2"
    case "$url" in
        http://* | https://*)
            if command -v curl >/dev/null 2>&1; then
                curl -fsSL --retry 2 --connect-timeout 20 -o "$dest" "$url"
            elif command -v wget >/dev/null 2>&1; then
                wget -q -O "$dest" "$url"
            else
                die "cannot download $url: curl or wget is missing"
            fi
            ;;
        *)
            [[ -f "$url" ]] || die "release file not found: $url"
            cp -- "$url" "$dest"
            ;;
    esac
}

resolve_url() {
    local url="$1"
    case "$url" in
        http://* | https://*) printf '%s\n' "$url" ;;
        *) printf '%s/%s\n' "${INDEX_BASE%/}" "$url" ;;
    esac
}

verify_signature() {
    local file="$1"
    local sig="$2"
    if ((ALLOW_UNSIGNED)); then
        return 0
    fi
    [[ -f "$sig" ]] || die "missing signature: $sig"
    minisign -Vm "$file" -p "$PUBKEY" -x "$sig" >/dev/null 2>&1 ||
        die "signature verification failed: $file"
}

verify_checksum() {
    local file="$1"
    local expected="$2"
    local actual
    actual="$(sha256sum "$file" | awk '{print $1}')"
    [[ "$actual" = "$expected" ]] ||
        die "checksum mismatch for $file (expected $expected, got $actual)"
}

# Print flat JSON objects from the index. App entries are flat by schema.
json_objects() {
    tr '\n\r' '  ' <"$INDEX_FILE" | grep -o '{[^{}]*}' | grep '"id"' || true
}

json_string() {
    local object="$1"
    local key="$2"
    printf '%s\n' "$object" |
        sed -n "s/.*\"$key\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" |
        head -n1
}

json_array() {
    local object="$1"
    local key="$2"
    local inner
    inner="$(printf '%s\n' "$object" |
        sed -n "s/.*\"$key\"[[:space:]]*:[[:space:]]*\[\([^]]*\)\].*/\1/p" |
        head -n1)"
    printf '%s' "$inner" | grep -o '"[^"]*"' | tr -d '"' || true
}

json_is_true() {
    local object="$1"
    local key="$2"
    printf '%s' "$object" | grep -q "\"$key\"[[:space:]]*:[[:space:]]*true"
}

# Validate one artifact URL and checksum pair.
validate_artifact() {
    local id="$1"
    local url="$2"
    local sha="$3"
    [[ -n "$url" && -n "$sha" ]] ||
        die "invalid index: app entry for $id is missing a url/sha256 pair"
    [[ "$sha" =~ ^[0-9a-f]{64}$ ]] || die "invalid index: bad sha256 for $id"
    case "$url" in
        /*) die "invalid index: unsafe url for $id" ;;
    esac
    [[ "/$url/" == *"/../"* ]] && die "invalid index: unsafe url for $id"
    if [[ "$url" == *://* && "$url" != https://* ]]; then
        die "invalid index: only https URLs are allowed for $id"
    fi
}

# Schema 1 indexes carry one flat url/sha256 pair. Schema 2 indexes carry one
# pair per architecture (`url_x86_64`, `sha256_aarch64`, ...), so the release
# job can merge the native x86_64 and aarch64 bundles without losing a mapping.
validate_index() {
    local text installer_block object id version url sha requires optional
    local arch arch_url arch_sha seen_arches seen_ids=" "
    text="$(tr '\n\r' '  ' <"$INDEX_FILE")"
    printf '%s' "$text" | grep -q '"schema"[[:space:]]*:[[:space:]]*[12]\([^0-9]\|$\)' ||
        die "invalid index: unsupported or missing schema"
    installer_block="$(printf '%s' "$text" |
        grep -o '"installer"[[:space:]]*:[[:space:]]*{[^{}]*}' | head -n1)"
    [[ -n "$installer_block" ]] || die "invalid index: installer.version is required"
    [[ -n "$(json_string "$installer_block" version)" ]] ||
        die "invalid index: installer.version is required"
    printf '%s' "$text" | grep -q '"apps"[[:space:]]*:' || die "invalid index: apps missing"

    while IFS= read -r object; do
        [[ -n "$object" ]] || continue
        id="$(json_string "$object" id)"
        version="$(json_string "$object" version)"
        url="$(json_string "$object" url)"
        sha="$(json_string "$object" sha256)"
        [[ -n "$id" && -n "$version" ]] ||
            die "invalid index: app entry is missing id/version"
        [[ "$id" =~ ^[a-z0-9][a-z0-9_-]*$ ]] || die "invalid index: bad app id '$id'"
        [[ "$seen_ids" == *" $id "* ]] && die "invalid index: duplicate app id $id"
        seen_ids+="$id "
        [[ "$version" =~ ^[0-9A-Za-z.+-]+$ ]] || die "invalid index: bad version for $id"

        seen_arches=""
        if [[ -n "$url" || -n "$sha" ]]; then
            validate_artifact "$id" "$url" "$sha"
            seen_arches="default"
        fi
        for arch in x86_64 aarch64; do
            arch_url="$(json_string "$object" "url_$arch")"
            arch_sha="$(json_string "$object" "sha256_$arch")"
            [[ -n "$arch_url" || -n "$arch_sha" ]] || continue
            validate_artifact "$id" "$arch_url" "$arch_sha"
            seen_arches+=" $arch"
        done
        [[ -n "$seen_arches" ]] ||
            die "invalid index: app entry for $id has no artifact"

        requires="$(json_array "$object" requires)"
        while IFS= read -r tool_name; do
            [[ -n "$tool_name" ]] || continue
            [[ "$tool_name" =~ ^[A-Za-z0-9._+/-]+$ ]] ||
                die "invalid index: bad prerequisite for $id"
        done <<<"$requires"
        optional="$(json_is_true "$object" optional && printf true || printf false)"
        [[ "$optional" == "true" || "$optional" == "false" ]] ||
            die "invalid index: optional must be a boolean for $id"
    done < <(json_objects)

    [[ "$seen_ids" != " " ]] || die "invalid index: apps must be a non-empty array"
}

load_index() {
    [[ -n "$INDEX_FILE" ]] && return 0
    require_tools
    WORK="${WORK:-$(mktemp -d "${TMPDIR:-/tmp}/rofi-apps-install.XXXXXX")}"

    if [[ "$INDEX_SOURCE" == http://* || "$INDEX_SOURCE" == https://* ]]; then
        INDEX_BASE="${INDEX_SOURCE%/*}"
        INDEX_FILE="$WORK/index.json"
        fetch "$INDEX_SOURCE" "$INDEX_FILE"
        if ((!ALLOW_UNSIGNED)); then
            fetch "${INDEX_SOURCE}.minisig" "$WORK/index.json.minisig"
            verify_signature "$INDEX_FILE" "$WORK/index.json.minisig"
        fi
    else
        [[ -f "$INDEX_SOURCE" ]] || die "index not found: $INDEX_SOURCE"
        INDEX_FILE="$INDEX_SOURCE"
        INDEX_BASE="$(cd -- "$(dirname -- "$INDEX_SOURCE")" && pwd)"
        if ((!ALLOW_UNSIGNED)); then
            verify_signature "$INDEX_FILE" "${INDEX_SOURCE}.minisig"
        fi
    fi

    validate_index
}

index_records() {
    local filter="$*"
    local object id url sha arch available
    while IFS= read -r object; do
        [[ -n "$object" ]] || continue
        id="$(json_string "$object" id)"
        if [[ -n "$filter" && " $filter " != *" $id "* ]]; then
            continue
        fi
        url="$(json_string "$object" url)"
        sha="$(json_string "$object" sha256)"
        available=""
        if [[ -n "$url" ]]; then
            available="legacy"
        fi
        for arch in x86_64 aarch64; do
            if [[ -n "$(json_string "$object" "url_$arch")" ]]; then
                available="${available:+$available,}$arch"
            fi
        done
        if [[ -z "$url" ]]; then
            url="$(json_string "$object" "url_$ARCH")"
            sha="$(json_string "$object" "sha256_$ARCH")"
        fi
        printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
            "$id" \
            "$(json_string "$object" version)" \
            "$url" \
            "$sha" \
            "$(json_array "$object" requires | tr '\n' ',' | sed 's/,$//')" \
            "$(json_is_true "$object" optional && printf true || printf false)" \
            "$available"
    done < <(json_objects)
}

record_for() {
    local id="$1"
    local record
    record="$(index_records "$id")"
    [[ -n "$record" ]] || die "unknown app: $id"
    if [[ -z "$(record_field "$record" 3)" && -z "$(record_field "$record" 4)" ]]; then
        local available
        available="$(record_field "$record" 7)"
        die "$id: release index has no artifact for $ARCH (available: ${available:-none})"
    fi
    printf '%s\n' "$record"
}

record_field() {
    local record="$1"
    local field="$2"
    cut -f"$field" <<<"$record"
}

state_path() {
    printf '%s/%s\n' "$INSTALLED_DIR" "$1"
}

app_installed() {
    [[ -f "$(state_path "$1")" ]]
}

installed_ids() {
    [[ -d "$INSTALLED_DIR" ]] || return 0
    find "$INSTALLED_DIR" -maxdepth 1 -type f ! -name '.in-progress-*' -printf '%f\n' | sort
}

state_value() {
    local state="$1"
    local key="$2"
    sed -n "s/^$key=//p" "$state" | head -n1
}

state_files() {
    sed -n 's/^file=//p' "$1"
}

toml_value() {
    local file="$1"
    local key="$2"
    sed -n "s/^[[:space:]]*${key}[[:space:]]*=[[:space:]]*\"\(.*\)\"[[:space:]]*$/\1/p" "$file" |
        head -n1
}

toml_bool() {
    local file="$1"
    local key="$2"
    sed -n "s/^[[:space:]]*${key}[[:space:]]*=[[:space:]]*\(true\|false\).*/\1/p" "$file" |
        head -n1
}

toml_list() {
    local file="$1"
    local key="$2"
    local inner
    inner="$(sed -n "s/^[[:space:]]*${key}[[:space:]]*=[[:space:]]*\[\(.*\)\].*/\1/p" "$file" |
        head -n1)"
    printf '%s' "$inner" | grep -o '"[^"]*"' | tr -d '"' || true
}

refuse_nix_managed_path() {
    local path resolved
    for path in "$PREFIX" "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_CONFIG_HOME"; do
        resolved="$(readlink -f -- "$path" 2>/dev/null || printf '%s' "$path")"
        case "$resolved" in
            /nix/store/*)
                die "refusing to write into a Nix-managed path: $resolved"
                ;;
        esac
    done
}

acquire_lock() {
    mkdir -p -- "$STATE_ROOT"
    chmod 700 -- "$STATE_ROOT"
    if command -v flock >/dev/null 2>&1; then
        exec 9>"$STATE_ROOT/lock"
        flock -n 9 || die "another install.sh run holds the lock"
    else
        LOCK_DIR="$STATE_ROOT/lock.d"
        mkdir -- "$LOCK_DIR" 2>/dev/null || die "another install.sh run holds the lock"
    fi
}

check_prerequisites() {
    local metadata="$1"
    local missing=()
    local tool
    while IFS= read -r tool; do
        [[ -n "$tool" ]] || continue
        if ! command -v "$tool" >/dev/null 2>&1; then
            missing+=("$tool")
        fi
    done < <(toml_list "$metadata" requires)
    if ((${#missing[@]})); then
        die "missing prerequisite(s): ${missing[*]}; install them with your system package manager"
    fi
}

validate_archive() {
    local archive="$1"
    local entry
    while IFS= read -r entry; do
        [[ -n "$entry" ]] || continue
        case "$entry" in
            /* | ../* | */../* | .. | */.. | *$'\n'* | *$'\r'*)
                die "unsafe path in archive: $entry"
                ;;
        esac
        case "$entry" in
            app.toml | ./app.toml | bin/* | ./bin/* | share/* | ./share/* | systemd/* | ./systemd/* | LICENSE | ./LICENSE | README.md | ./README.md | . | ./) ;;
            *) die "unexpected archive member: $entry" ;;
        esac
    done < <(tar -tzf "$archive")
    if tar -tvzf "$archive" | awk 'substr($1,1,1) ~ /[lhcbp]/ { found=1 } END { exit !found }'; then
        die "unsafe archive member type (link or device)"
    fi
}

preflight() {
    local id="$1"
    local version url sha record archive sig extract metadata resolved
    record="$(record_for "$id")"
    version="$(record_field "$record" 2)"
    url="$(record_field "$record" 3)"
    sha="$(record_field "$record" 4)"
    archive="$WORK/$id.tar.gz"
    sig="$WORK/$id.tar.gz.minisig"
    resolved="$(resolve_url "$url")"
    verbose "fetching $resolved"
    fetch "$resolved" "$archive"
    verify_checksum "$archive" "$sha"
    if ((!ALLOW_UNSIGNED)); then
        fetch "$(resolve_url "$url").minisig" "$sig"
        verify_signature "$archive" "$sig"
    fi

    extract="$WORK/$id"
    mkdir -p -- "$extract"
    validate_archive "$archive"
    tar -xzf "$archive" --no-same-owner --no-same-permissions -C "$extract"

    metadata="$extract/app.toml"
    [[ -f "$metadata" ]] || die "$id: tarball is missing app.toml"
    [[ "$(toml_value "$metadata" id)" = "$id" ]] || die "$id: app.toml id mismatch"
    [[ "$(toml_value "$metadata" version)" = "$version" ]] ||
        die "$id: app.toml version does not match index ($version)"
    check_prerequisites "$metadata"
    PREFLIGHT_DIR["$id"]="$extract"
}

backup_target() {
    local target="$1"
    local rel="$2"
    if [[ -e "$target" || -L "$target" ]]; then
        mkdir -p -- "$(dirname -- "$BACKUP_DIR/$rel")"
        cp -a -- "$target" "$BACKUP_DIR/$rel"
        BACKUP_FILES+=("$rel")
    fi
}

install_one_file() {
    local source="$1"
    local target="$2"
    local mode="$3"
    case "$target" in
        *$'\n'* | *$'\r'*) die "refusing a path with control characters: $target" ;;
    esac
    local rel="${target#/}"
    rel="${rel//\//__}"
    if ((DRY_RUN)); then
        info "would install $target"
        return 0
    fi
    mkdir -p -- "$(dirname -- "$target")"
    if [[ -L "$target" ]]; then
        case "$(readlink -f -- "$target" 2>/dev/null || true)" in
            /nix/store/*) die "refusing to replace Nix-managed symlink: $target" ;;
        esac
    fi
    backup_target "$target" "$rel"
    local temp="$target.tmp.$$"
    cp -- "$source" "$temp"
    chmod "$mode" -- "$temp"
    mv -f -- "$temp" "$target"
    NEW_FILES+=("$target")
    verbose "installed $target"
}

write_state() {
    local id="$1"
    local version="$2"
    local bin="$3"
    local provider="$4"
    local unit="$5"
    local state temp file
    state="$(state_path "$id")"
    mkdir -p -- "$INSTALLED_DIR"
    chmod 700 -- "$INSTALLED_DIR" "$STATE_ROOT"
    temp="$state.tmp.$$"
    {
        printf 'schema=1\n'
        printf 'id=%s\n' "$id"
        printf 'version=%s\n' "$version"
        printf 'installer=%s\n' "$INSTALLER_VERSION"
        printf 'bin=%s\n' "$bin"
        printf 'provider=%s\n' "$provider"
        printf 'service_unit=%s\n' "$unit"
        for file in "${NEW_FILES[@]}"; do
            printf 'file=%s\n' "$file"
        done
    } >"$temp"
    chmod 600 -- "$temp"
    mv -f -- "$temp" "$state"
    printf '%s\n' "$INSTALLER_VERSION" >"$STATE_ROOT/version"
    chmod 600 -- "$STATE_ROOT/version"
}

enable_service() {
    local unit_name="$1"
    ((NO_SERVICE)) && return 0
    if ! command -v systemctl >/dev/null 2>&1; then
        warn "systemctl not found; $unit_name installed but not enabled"
        return 0
    fi
    if ((DRY_RUN)); then
        info "would enable $unit_name"
        return 0
    fi
    if systemctl --user daemon-reload >/dev/null 2>&1 &&
        systemctl --user enable --now "$unit_name" >/dev/null 2>&1; then
        verbose "enabled $unit_name"
    else
        warn "could not enable $unit_name; run: systemctl --user enable --now $unit_name"
    fi
}

install_app() {
    local id="$1"
    local extract="${PREFLIGHT_DIR[$id]:-}"
    [[ -n "$extract" ]] || die "$id: internal error: missing preflight"
    local metadata="$extract/app.toml"
    local version bin provider service unit_rel unit="" unit_name="" unit_source
    local provider_path="" source target

    version="$(toml_value "$metadata" version)"
    bin="$(toml_value "$metadata" bin)"
    provider="$(toml_value "$metadata" provider)"
    service="$(toml_bool "$metadata" service)"
    unit_rel="$(toml_value "$metadata" service_unit)"

    CURRENT_APP="$id"
    BACKUP_DIR="$WORK/backup-$id"
    NEW_FILES=()
    BACKUP_FILES=()
    mkdir -p -- "$BACKUP_DIR"
    if ((!DRY_RUN)); then
        mkdir -p -- "$INSTALLED_DIR"
        chmod 700 -- "$STATE_ROOT" "$INSTALLED_DIR"
        : >"$INSTALLED_DIR/.in-progress-$id"
        chmod 600 -- "$INSTALLED_DIR/.in-progress-$id"
    fi

    while IFS= read -r -d '' source; do
        target="$BIN_DIR/$(basename -- "$source")"
        install_one_file "$source" "$target" 755
    done < <(find "$extract/bin" -type f -print0)

    if [[ "${ROFI_APPS_INJECT_FAILURE:-}" = "after-bin" ]]; then
        die "injected failure after installing binaries (test hook)"
    fi

    if [[ -d "$extract/share" ]]; then
        while IFS= read -r -d '' source; do
            target="$DATA_ROOT/${source#"$extract/share/"}"
            install_one_file "$source" "$target" 644
        done < <(find "$extract/share" -type f -print0)
    fi

    if [[ "$service" = "true" && "$NO_SERVICE" = 0 && -n "$unit_rel" && -f "$extract/$unit_rel" ]]; then
        unit="$SYSTEMD_USER_DIR/$(basename -- "$unit_rel")"
        unit_name="$(basename -- "$unit_rel")"
        unit_source="$extract/$unit_rel"
        if [[ "$PREFIX/bin" != "$HOME/.local/bin" ]]; then
            unit_source="$WORK/unit-$id.service"
            sed "s|%h/.local/bin|$PREFIX/bin|g" "$extract/$unit_rel" >"$unit_source"
        fi
        install_one_file "$unit_source" "$unit" 644
    fi

    if ((DRY_RUN)); then
        CURRENT_APP=""
        BACKUP_DIR=""
        return 0
    fi

    if [[ -n "$provider" ]]; then
        provider_path="$PROVIDER_DIR/$(basename -- "$provider")"
    fi
    write_state "$id" "$version" "$BIN_DIR/$bin" "$provider_path" "$unit_name"
    rm -f -- "$INSTALLED_DIR/.in-progress-$id"
    if [[ -n "$unit_name" ]]; then
        enable_service "$unit_name"
    fi

    CURRENT_APP=""
    BACKUP_DIR=""
    info "installed $id $version"
}

uninstall_app() {
    local id="$1"
    local state
    state="$(state_path "$id")"
    app_installed "$id" || die "$id is not installed (no state at $state)"
    if ((DRY_RUN)); then
        info "would uninstall $id"
        return 0
    fi

    local bin unit file
    bin="$(state_value "$state" bin)"
    unit="$(state_value "$state" service_unit)"
    if [[ -n "$unit" ]] && command -v systemctl >/dev/null 2>&1; then
        systemctl --user disable --now "$unit" >/dev/null 2>&1 || true
    fi

    while IFS= read -r file; do
        [[ -n "$file" ]] || continue
        case "$file" in
            "$BIN_DIR"/* | "$DATA_ROOT"/* | "$SYSTEMD_USER_DIR"/*) rm -f -- "$file" ;;
            *) warn "refusing to remove unexpected path: $file" ;;
        esac
    done < <(state_files "$state")
    rm -f -- "$state"
    [[ -n "$bin" ]] && rm -f -- "$bin"
    for dir in "$PROVIDER_DIR" "$DATA_ROOT/rofi-apps/apps/$id"; do
        rmdir -- "$dir" 2>/dev/null || true
    done
    info "uninstalled $id"
}

list_apps() {
    load_index
    local record id version requires state_version marker
    printf '%-16s %-10s %-24s %s\n' "ID" "VERSION" "REQUIRES" "STATE"
    while IFS= read -r record; do
        [[ -n "$record" ]] || continue
        id="$(record_field "$record" 1)"
        version="$(record_field "$record" 2)"
        requires="$(record_field "$record" 5)"
        [[ -n "$requires" ]] || requires="-"
        marker="available"
        if app_installed "$id"; then
            state_version="$(state_value "$(state_path "$id")" version)"
            if [[ "$state_version" = "$version" ]]; then
                marker="installed"
            else
                marker="installed ${state_version:-?}"
            fi
        fi
        printf '%-16s %-10s %-24s %s\n' "$id" "$version" "$requires" "$marker"
    done < <(index_records)
}

select_install_apps() {
    if ((ALL)); then
        mapfile -t SELECTED < <(index_records | awk -F '\t' '$6 != "true" {print $1}')
    else
        SELECTED=("${APPS[@]}")
    fi
    ((${#SELECTED[@]})) || die "select --app ID or --all"
}

main() {
    case "$MODE" in
        list)
            load_index
            list_apps
            return 0
            ;;
        install | update | uninstall) ;;
        *) die "unknown command mode: $MODE" ;;
    esac

    if [[ "$MODE" = uninstall ]]; then
        acquire_lock
        local ids=()
        local id
        if ((${#APPS[@]})); then
            ids=("${APPS[@]}")
        else
            mapfile -t ids < <(installed_ids)
        fi
        ((${#ids[@]})) || die "no installed apps to uninstall"
        for id in "${ids[@]}"; do
            uninstall_app "$id"
        done
        return 0
    fi

    load_index
    refuse_nix_managed_path
    acquire_lock

    local ids=()
    local id
    if [[ "$MODE" = update ]]; then
        if ((${#APPS[@]})); then
            ids=("${APPS[@]}")
        else
            mapfile -t ids < <(installed_ids)
        fi
        ((${#ids[@]})) || die "no installed apps to update"
        for id in "${ids[@]}"; do
            app_installed "$id" || die "$id is not installed; use --app $id"
        done
    else
        select_install_apps
        ids=("${SELECTED[@]}")
        for id in "${ids[@]}"; do
            if app_installed "$id"; then
                die "$id is already installed; use --update $id"
            fi
        done
    fi

    # Preflight every selected app before touching any file. This guarantees a
    # missing prerequisite or bad checksum aborts without a partial install.
    for id in "${ids[@]}"; do
        preflight "$id"
    done
    for id in "${ids[@]}"; do
        install_app "$id"
    done
}

main "$@"
