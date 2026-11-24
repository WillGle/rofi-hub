# Docs

- [Architecture](ARCHITECTURE.md) — workspace layout, provider contract, paths, theming, and the update model.
- [Migration from screen-time-rofi](MIGRATING.md) — what moved, what stayed compatible, and how to switch an existing install.
- [Screen Time user guide](../crates/screen-time/docs/USER_GUIDE.md)
- [Screen Time data safety and updates](../crates/screen-time/docs/AUTO_UPDATE.md)
- [Screen Time evaluation](../crates/screen-time/docs/EVALUATION.md)
- [Screen Time developer notes](../crates/screen-time/docs/README.md)

## Reproduce checks

From the workspace root:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
shellcheck install.sh uninstall.sh scripts/*.sh crates/screen-time/share/rofi-screen-time
scripts/test-installer.sh
nix flake check
```
