# Changelog

## Unreleased

- Show 12 nearby Wi-Fi candidates in the Network main view.
- Use a 3-column × 10-row Keybinds list (10 visible rows maximum).
- Default the optional app launcher to 3 columns × 10 rows (config clamps
  `columns` 1–6 and `lines` 1–10). The emoji picker uses a fixed 4 × 15 grid
  and the power menu a 3 × 2 grid.
- Close UI-01..08 across the seven apps:
  - Keybinds rows put the shortcut and description in the searchable dmenu id
    while keeping a stable binding index; header rows never run a command.
  - Shared row markup inherits the row foreground for titles and metadata, so
    selected text stays readable on light desktop themes.
  - Clipboard, Keybinds, and Hub headers use the fixed active style and ignore
    header selections.
  - Clipboard replaces textual chips with RASI Copy/Delete/Clear/Refresh
    buttons that share the Enter/`Alt+d`/`Alt+x`/`Alt+r` paths.
  - Calendar, Clipboard, Keybinds, Network, Atomic Tasks, and Screen Time
    persist `Ctrl+1–4` as an app-owned theme lock
    (`~/.local/state/rofi-apps/<app>/theme-choice`); `Ctrl+4` clears it and
    restores the configured theme.
  - Hub `Ctrl+1–4` preserves filter, selected provider id, and window height;
    the selected palette is layered over the resolved menu theme.
  - Screen Time keeps the cursor separate from the expanded detail and restores
    the app on the same page after a theme change.
  - Atomic Tasks moves Done/Edit/Move/priority controls into an inline
    expandable action area opened by Enter or Actions; the second task menu is
    gone.
  - Calendar moves Prev/Today/Next to RASI buttons outside the filtered list,
    so they remain visible while typing a date. The button row reserves space
    for the full rounded outline at scaled/HiDPI font sizes. The window is
    about 5% narrower and 10% taller; Prev and Next are compact icon buttons
    while Today keeps the remaining row width. The month title and sheet text
    use 18pt/12pt sizes for a clearer hierarchy. The theme hint moved out of
    the month card to a centered line below the action row, so it is never
    clipped by the card at scaled font sizes.
- Link `/share/rofi` and `/share/rofi-apps` into the NixOS system profile so
  the optional menu discovers every enabled app and apps keep their packaged
  fallback layouts.
- Add the inline Atomic Tasks menu test and the §18 interaction screenshot set.
- Revisit the README, UI guide, architecture, packaging, migration, screenshot,
  Screen Time, and goal documents for the current seven-app behavior.
- Redesign `rofi-calendar`: centered month sheet and seven-column grid in the
  message, `Go to date` input, and Prev/Today/Next in one RASI button row.
  Focused dates use bold underline plus a full date line; invalid input keeps
  the month and shows one error line.
- Redesign `rofi-network`: one connection line, `Find network` filter, and a
  vertical SSID list; Manage and Connection details are separate screens.
  Long SSIDs truncate while full-name filtering still matches, and empty/radio
  states are static nonselectable rows.
- Release indexes keep a tarball URL and checksum per architecture. Native
  x86_64 and aarch64 bundles merge without losing either mapping, and
  `install.sh` rejects a host with no artifact.
- Add a native aarch64 Nix CI job for all seven packages.
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
