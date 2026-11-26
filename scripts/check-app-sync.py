#!/usr/bin/env python3
"""Verify that every app package is wired through the workspace and release.

The canonical app list is the set of ``crates/*/app.toml`` files. This script
keeps the Cargo workspace, flake packages/modules, and release tooling in sync.
"""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
errors: list[str] = []
checks = 0


def check(condition: bool, message: str) -> None:
    global checks
    checks += 1
    if not condition:
        errors.append(message)


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


workspace = load_toml(ROOT / "Cargo.toml")
members = {
    member.removeprefix("crates/"): member
    for member in workspace["workspace"]["members"]
}
flake = (ROOT / "flake.nix").read_text()
release = (ROOT / "scripts/package-release.sh").read_text()
workflow = (ROOT / ".github/workflows/release.yml").read_text()

apps: dict[str, dict] = {}
for crate_dir in sorted((ROOT / "crates").iterdir()):
    metadata_path = crate_dir / "app.toml"
    if not metadata_path.is_file():
        continue
    metadata = load_toml(metadata_path)
    app_id = metadata["id"]
    apps[app_id] = metadata
    crate = crate_dir.name
    check(crate in members, f"{crate}: missing from workspace members")
    manifest = load_toml(crate_dir / "Cargo.toml")
    check(
        manifest["package"]["version"] == metadata["version"],
        f"{app_id}: Cargo.toml and app.toml versions differ",
    )
    provider = crate_dir / metadata["provider"] if metadata.get("provider") else None
    if provider is not None:
        check(provider.is_file(), f"{app_id}: missing provider {provider}")
        if provider.is_file():
            provider_data = load_toml(provider)
            check(provider_data["id"] == app_id, f"{app_id}: provider id mismatch")
            check(
                provider_data["version"] == metadata["version"],
                f"{app_id}: provider version mismatch",
            )
    layout = crate_dir / "share/rofi" / f"{app_id}.rasi"
    check(layout.is_file(), f"{app_id}: missing layout fallback {layout}")

    check(
        f'"{app_id}:{crate}:' in release or f'("{app_id}", "{crate}",' in release,
        f"{app_id}: missing from package-release app list",
    )
    package_attr = "rofiCalendar" if crate == "rofi-calendar" else None
    if package_attr:
        check(package_attr in flake, f"{app_id}: missing flake package")
    if app_id == "hub-menu":
        check("rofi-hub-menu" in flake, "hub-menu: missing flake module")
    else:
        check(f"rofi-apps.{app_id}" in flake or f'app = "{app_id}"' in flake,
              f"{app_id}: missing flake module")

for required in (
    "install.sh",
    "uninstall.sh",
    "installer/index.schema.json",
    "scripts/package-release.sh",
    "scripts/sign-release.sh",
):
    check((ROOT / required).is_file(), f"missing {required}")

for forbidden in (
    "packaging",
    "providers",
    "scripts/install-tree.sh",
    "scripts/test-install-tree.sh",
    "scripts/check-packaging-sync.py",
    "nfpm.yaml",
):
    check(not (ROOT / forbidden).exists(), f"legacy path still present: {forbidden}")

for script in (
    "scripts/check-deps.py",
    "scripts/test-apps.sh",
    "scripts/test-theme.sh",
    "scripts/test-installer.sh",
    "scripts/test-bundle.sh",
    "scripts/test-migration.sh",
    "scripts/test-release-index.sh",
    "scripts/test-nix.sh",
):
    check(script in workflow or script == "scripts/test-nix.sh",
          f"CI workflow does not run {script}")

check(len(apps) == 6, f"expected 6 app packages, found {len(apps)}")
check("rofi-hub-menu" in flake, "missing optional rofi-hub-menu package")
check("rofi-hub-installer" in flake, "missing rofi-hub-installer package")

if errors:
    print(f"check-app-sync: FAILED ({len(errors)} errors)", file=sys.stderr)
    for error in errors:
        print(f"  - {error}", file=sys.stderr)
    sys.exit(1)

print(f"check-app-sync: ok ({checks} checks, {len(apps)} apps)")
