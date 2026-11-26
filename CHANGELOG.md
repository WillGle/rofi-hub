# Changelog

## Unreleased

- Redesign `rofi-calendar`: centered month sheet and seven-column grid in the
  message, `Go to date` input, and Prev/Today/Next in one compact row. Focused
  dates use bold underline plus a full date line; invalid input keeps the month
  and shows one error line.
- Redesign `rofi-network`: one connection line, `Find network` filter, and a
  vertical SSID list; Manage and Connection details are separate screens.
  Long SSIDs truncate while full-name filtering still matches, and empty/radio
  states are static nonselectable rows.
- Release indexes keep a tarball URL and checksum per architecture. Native
  x86_64 and aarch64 bundles merge without losing either mapping, and
  `install.sh` rejects a host with no artifact.
- Add a native aarch64 Nix CI job for all six packages.
- Refresh Calendar/Network screenshots and the Calendar/Network theme profiles;
  update Screen Time installation docs for the per-app installer.

## 0.3.0

- Split the runtime menu from the apps. `crates/rofi-hub-menu` is optional and
  never injects theme, provider, config, or `PATH` variables into apps.
- Rename `rofi-core` to `rofi-dmenu`, add feature gates, and remove all
  `ROFI_HUB_*` handling and hub theme state from the app path.
- Extract Network into `crates/rofi-network` with its own provider, layout,
  release, and runtime dependency wrapper.
- Publish independent per-app packages and versions:
  calendar 1.2.0, clipboard 0.8.0, keybinds 1.0.0, network 0.3.0,
  screen-time 0.5.0, rofi-hub-menu 0.3.0, rofi-dmenu 0.3.0.
- Replace the bundle installer with a generic `index.json` + `app.toml`
  installer: per-app state, update/uninstall one app, checksum/signature
  verification, path-traversal rejection, rollback, and app-owned services.
- Theme ownership moves to the user: app override, `ROFI_THEME`, base
  `~/.config/rofi/theme.rasi`, packaged fallback, built-in fallback.
- Replace Nix bundles with `packages.rofi-<app>` and per-app NixOS/Home
  Manager modules under `services.rofi-apps`. Remove `runtimePackagesFor`.
  `services.rofi-hub.enable` remains as a deprecated alias.
- Remove `.deb`, `.rpm`, `.apk`, AUR, Debian official, Gentoo, openSUSE, Void,
  Slackware, ALT, and all distro packaging scripts from delivery.
- Add dependency, app, theme, installer, migration, release-index, and Nix
  test suites plus cross-distro `install.sh` container tests.

## 0.2.1

- Add multi-distro packaging: a `PREFIX`/`DESTDIR` install tree shared by every
  recipe, systemd/OpenRC/runit services, desktop/AppStream/man/completions
  assets, native `.deb`/`.rpm`/`.apk` artifacts, Arch/AUR, Debian, Fedora,
  Alpine, Gentoo, openSUSE, Void, Slackware, and ALT recipes, a five-distro CI
  matrix, and `docs/PACKAGING.md`.
- Add the NetworkManager Wi-Fi and Ethernet menu (`rofi-hub network`).
- Treat Rofi closing its stdin early as a cancellation instead of a fatal I/O
  error.
- Publish the tarball, `.deb`, `.rpm`, `.apk`, source asset, and signed
  `SHA256SUMS` from the tag release.

## 0.2.0

- Add three built-in provider apps: `calendar` (month sheet with date search),
  `keybinds` (Hyprland shortcut reference with runnable `exec` rows), and
  `clipboard` (`cliphist` history copied back with `wl-copy`).
- Extend the release bundle, installer component selection, Nix packages, and
  Home Manager module to cover the new apps.
- Share app theme/config/PID resolution in `rofi-core`, including
  `ROFI_HUB_THEME_FILE` so providers opened from the hub keep the chosen look.
- Add app integration tests with a fake Rofi, an app-sync checker that validates
  provider manifests against the workspace, installer, flake, packaging, and CI,
  a release-bundle smoke test, and a CI pipeline with lint, workspace tests,
  MSRV, installer fault injection, bundle builds, and Nix checks.

## 0.1.0

- Introduce the `rofi-hub` launcher, `rofi-core` provider protocol, provider
  manifests, shared theme modes, and a generated Rofi manager theme.
- Ship one signed bundle with selective component installation:
  `install.sh` installs the all-in-one hub, selected apps, or only the
  launcher, and preserves the selection on later updates.
- Import Screen Time 0.5.0 as the first app, including suspend/hibernate gap
  tracking, the static day timeline, SQLite history, and signed updater.
- Keep the conservative update path: release verification, database snapshot,
  verified backups, rollback, periodic checks, and opt-in auto-apply timers.
- Honor `ROFI_BIN`, `ROFI_CONFIG`, and `ROFI_THEME` consistently in the hub and
  the Screen Time wrapper; providers resolve from `PATH` or next to the hub
  binary. The hub uses a private PID file and never kills other Rofi prompts.
- Keep CLI failures on stderr instead of desktop notifications, and cover
  dispatch, provider environment, themes, and `doctor` with integration tests.
- Add a Nix flake with all-in-one, launcher-only, and tracker-only packages plus
  Home Manager modules.
