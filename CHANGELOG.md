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
- Add a Nix flake with all-in-one, launcher-only, and tracker-only packages plus
  Home Manager modules.
