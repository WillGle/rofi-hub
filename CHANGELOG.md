# Changelog

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
