#!/usr/bin/env python3
"""Dependency and ownership guard for the rofi-apps architecture.

Fails when:
  * an app crate depends on the menu, installer, or a sibling app;
  * the menu depends on an app crate;
  * an app package is missing app.toml or its provider manifest;
  * provider/app/Cargo versions disagree;
  * app runtime sources read hub-owned environment variables;
  * the flake reintroduces a global runtime package list.
"""

from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
APPS = {
    "rofi-calendar": "calendar",
    "rofi-clipboard": "clipboard",
    "rofi-keybinds": "keybinds",
    "rofi-network": "network",
    "screen-time": "screen-time",
}
FORBIDDEN_APP_DEPS = {"rofi-hub", "rofi-hub-menu", "rofi-hub-installer", *APPS}

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


def dependencies(manifest: dict) -> set[str]:
    names: set[str] = set()
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        names.update(manifest.get(section, {}).keys())
    return names


def normalized(name: str) -> str:
    return name.replace("_", "-")


workspace = load_toml(ROOT / "Cargo.toml")
members = [ROOT / member for member in workspace["workspace"]["members"]]
manifests = {}
for directory in members:
    manifest = load_toml(directory / "Cargo.toml")
    manifests[manifest["package"]["name"]] = (directory, manifest)

check("rofi-dmenu" in manifests, "rofi-dmenu library is missing")
check("rofi-hub-menu" in manifests, "rofi-hub-menu package is missing")
check("rofi-hub" not in manifests, "legacy rofi-hub crate must be gone")
check("rofi-core" not in manifests, "legacy rofi-core crate must be renamed")

for app, app_id in APPS.items():
    check(app in manifests, f"missing app crate {app}")
    if app not in manifests:
        continue
    directory, manifest = manifests[app]
    deps = {normalized(dep) for dep in dependencies(manifest)}
    forbidden = deps & {normalized(dep) for dep in FORBIDDEN_APP_DEPS - {app}}
    check(
        not forbidden,
        f"{app} must not depend on {sorted(forbidden)}",
    )
    check(
        any(normalized(dep) == "rofi-dmenu" for dep in deps),
        f"{app} must depend on the rofi-dmenu library",
    )

    app_toml = directory / "app.toml"
    check(app_toml.is_file(), f"{app} is missing app.toml")
    if app_toml.is_file():
        metadata = load_toml(app_toml)
        check(metadata.get("id") == app_id, f"{app_toml}: id must be {app_id}")
        check(
            metadata.get("version") == manifest["package"]["version"],
            f"{app_toml}: version does not match Cargo.toml",
        )
        provider_name = metadata.get("provider")
        check(isinstance(provider_name, str), f"{app_toml}: provider is required")
        if isinstance(provider_name, str):
            provider = directory / provider_name
            check(provider.is_file(), f"{app_toml}: missing provider {provider}")
            if provider.is_file():
                provider_data = load_toml(provider)
                check(
                    provider_data.get("id") == app_id,
                    f"{provider}: provider id must be {app_id}",
                )
                check(
                    provider_data.get("version") == metadata.get("version"),
                    f"{provider}: provider version must match app.toml",
                )
                check(
                    provider_data.get("exec", [])[:1] == [metadata.get("bin")],
                    f"{provider}: exec must start with the app binary",
                )
        layout = metadata.get("layout")
        check(
            layout is None or (directory / layout).is_file(),
            f"{app_toml}: layout {layout} is missing",
        )

    for source in (directory / "src").rglob("*.rs"):
        text = source.read_text(errors="replace")
        check(
            "ROFI_HUB_" not in text,
            f"{source}: app runtime must not read ROFI_HUB_* variables",
        )

if "rofi-hub-menu" in manifests:
    directory, manifest = manifests["rofi-hub-menu"]
    deps = {normalized(dep) for dep in dependencies(manifest)}
    app_deps = deps & {normalized(app) for app in APPS}
    check(not app_deps, f"rofi-hub-menu must not depend on apps: {sorted(app_deps)}")
    check("rofi-dmenu" in deps, "rofi-hub-menu must depend on rofi-dmenu")
    menu_sources = "\n".join(
        path.read_text(errors="replace") for path in (directory / "src").rglob("*.rs")
    )
    for variable in (
        "ROFI_HUB_PROVIDER",
        "ROFI_HUB_THEME",
        "ROFI_HUB_THEME_FILE",
        "ROFI_HUB_PROVIDER_DIR",
    ):
        check(
            f'env("{variable}"' not in menu_sources
            and f'env_remove("{variable}"' not in menu_sources,
            f"rofi-hub-menu must not set {variable}",
        )

installer = (ROOT / "install.sh").read_text(errors="replace")
check("KNOWN_APPS" not in installer, "install.sh must not hardcode KNOWN_APPS")
check("--nixos" not in installer, "install.sh must not expose --nixos")
for app in APPS.values():
    # Apps may be named in comments/help; the check targets hardcoded arrays.
    check(
        f'"{app}"' not in installer,
        f"install.sh must not hardcode app id {app}",
    )
for forbidden in ("cliphist", "wl-clipboard", "networkmanager"):
    check(
        f"install {forbidden}" not in installer.lower(),
        f"install.sh must not install {forbidden}",
    )

flake = (ROOT / "flake.nix").read_text(errors="replace")
check(
    "runtimePackagesFor" not in flake,
    "flake.nix must not define a global runtimePackagesFor",
)
check(
    "rofiWithPlugins" not in flake,
    "flake.nix must not inject a plugin-enabled Rofi into apps",
)

release_script = (ROOT / "scripts/package-release.sh").read_text(errors="replace")
check(
    "index.json" in release_script,
    "package-release.sh must emit index.json",
)
check(
    "app.toml" in release_script,
    "package-release.sh must package app.toml",
)

# No distro packaging artifacts may remain.
for pattern in ("*.deb", "*.rpm", "*.apk"):
    found = [path for path in ROOT.rglob(pattern) if ".git" not in path.parts]
    check(not found, f"distro artifact(s) present: {found[:3]}")
for directory in (
    "debian",
    "fedora",
    "arch",
    "alpine",
    "gentoo",
    "opensuse",
    "void",
    "slackbuilds",
    "alt",
):
    check(
        not (ROOT / "packaging" / directory).exists(),
        f"packaging/{directory} must be removed",
    )

if errors:
    print("check-deps: FAILED", file=sys.stderr)
    for error in errors:
        print(f"  - {error}", file=sys.stderr)
    sys.exit(1)

print(f"check-deps: ok ({checks} checks)")
print(json.dumps({"apps": sorted(APPS), "checks": checks}))
