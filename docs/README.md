# Docs

- [Architecture](ARCHITECTURE.md) — dependency graph, app package contract, theme precedence, installer and release model.
- [Packaging](PACKAGING.md) — per-app release layout, signing, install paths, and Nix outputs.
- [Migration](MIGRATING.md) — old `rofi-hub` install to per-app packages, theme ownership, and aliases.
- [UI style](UI_STYLE.md) — palette, component grammar, profiles, and theme ownership.
- [Screenshot matrix](screenshots/ui/README.md)
- [Screen Time user guide](../crates/screen-time/docs/USER_GUIDE.md)
- [Screen Time data safety and updates](../crates/screen-time/docs/AUTO_UPDATE.md)
- [Screen Time evaluation](../crates/screen-time/docs/EVALUATION.md)
- [Screen Time developer notes](../crates/screen-time/docs/README.md)

## Reproduce checks

From the workspace root:

```bash
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
python3 scripts/check-deps.py
python3 scripts/check-app-sync.py
shellcheck install.sh uninstall.sh scripts/*.sh \
  crates/screen-time/scripts/*.sh crates/screen-time/share/rofi-screen-time
scripts/test-apps.sh
scripts/test-theme.sh
scripts/test-installer.sh
scripts/test-release-index.sh
scripts/test-bundle.sh
scripts/test-migration.sh
scripts/test-performance.sh
scripts/test-nix.sh
nix flake check
```
