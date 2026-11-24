# Changelog

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
