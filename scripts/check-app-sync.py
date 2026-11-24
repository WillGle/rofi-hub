#!/usr/bin/env python3
"""Verify that every bundled app is wired through the workspace and pipeline.

The provider manifests in ``providers/*.toml`` are the canonical app list.
This script keeps the installer, Nix outputs, release packaging, and installer
test fixture in sync with those manifests.
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
INFRA_CRATES = {"rofi-core", "rofi-hub"}


class Checker:
    def __init__(self) -> None:
        self.errors: list[str] = []
        self.warnings: list[str] = []

    def fail(self, message: str) -> None:
        self.errors.append(message)

    def require(self, condition: bool, message: str) -> None:
        if not condition:
            self.fail(message)

    def warn(self, message: str) -> None:
        self.warnings.append(message)

    def finish(self) -> int:
        for warning in self.warnings:
            print(f"warning: {warning}", file=sys.stderr)
        if self.errors:
            for error in self.errors:
                print(f"error: {error}", file=sys.stderr)
            return 1
        return 0


def read_text(path: Path, checker: Checker) -> str:
    try:
        return path.read_text()
    except OSError as error:
        checker.fail(f"cannot read {path.relative_to(ROOT)}: {error}")
        return ""


def load_toml(path: Path, checker: Checker) -> dict:
    try:
        return tomllib.loads(path.read_text())
    except (OSError, tomllib.TOMLDecodeError) as error:
        checker.fail(f"cannot parse {path.relative_to(ROOT)}: {error}")
        return {}


def main() -> int:
    checker = Checker()

    workspace = load_toml(ROOT / "Cargo.toml", checker)
    workspace_package = workspace.get("workspace", {}).get("package", {})
    workspace_version = workspace_package.get("version")
    members = set(workspace.get("workspace", {}).get("members", []))
    default_members = set(workspace.get("workspace", {}).get("default-members", []))

    crates: dict[str, dict] = {}
    for directory in sorted((ROOT / "crates").iterdir()):
        if not directory.is_dir():
            continue
        manifest_path = directory / "Cargo.toml"
        if not manifest_path.is_file():
            checker.fail(f"crate directory without Cargo.toml: {directory.relative_to(ROOT)}")
            continue
        manifest = load_toml(manifest_path, checker)
        package = manifest.get("package", {})
        name = package.get("name")
        if not name:
            checker.fail(f"{manifest_path.relative_to(ROOT)} has no package.name")
            continue
        version = package.get("version")
        if isinstance(version, dict) and version.get("workspace"):
            version = workspace_version
        bins = [entry.get("name") for entry in manifest.get("bin", [])] or [name]
        crates[name] = {
            "directory": directory,
            "relative": str(directory.relative_to(ROOT)),
            "version": version,
            "bins": [bin_name for bin_name in bins if bin_name],
        }

    for crate, data in crates.items():
        relative = data["relative"]
        checker.require(
            relative in members,
            f"crate `{crate}` is not in workspace.members ({relative})",
        )
        checker.require(
            relative in default_members,
            f"crate `{crate}` is not in workspace.default-members ({relative})",
        )

    providers: dict[str, dict] = {}
    for path in sorted((ROOT / "providers").glob("*.toml")):
        manifest = load_toml(path, checker)
        provider_id = manifest.get("id")
        if not provider_id:
            checker.fail(f"{path.relative_to(ROOT)} has no id")
            continue
        if path.stem != provider_id:
            checker.fail(f"{path.relative_to(ROOT)} must be named after its id `{provider_id}`")
        if provider_id in providers:
            checker.fail(f"duplicate provider id `{provider_id}`")
            continue
        for field in ("name", "description", "category", "exec", "version"):
            checker.require(
                bool(manifest.get(field)),
                f"provider `{provider_id}` is missing `{field}`",
            )
        if not re.fullmatch(r"[a-z0-9][a-z0-9_-]*", provider_id):
            checker.fail(f"provider id `{provider_id}` is not a safe manifest id")
        exec_argv = manifest.get("exec") or []
        if not exec_argv:
            checker.fail(f"provider `{provider_id}` has an empty exec")
        else:
            for argument in exec_argv:
                checker.require(
                    "\0" not in argument and "\n" not in argument,
                    f"provider `{provider_id}` exec contains a control character",
                )
        providers[provider_id] = {"manifest": manifest, "path": path}

    app_crates: dict[str, str] = {}
    for provider_id, provider in providers.items():
        crate_name = provider_id if provider_id in crates else f"rofi-{provider_id}"
        if crate_name not in crates:
            checker.fail(
                f"provider `{provider_id}` has no matching crate "
                f"(looked for `{crate_name}`)"
            )
            continue
        crate = crates[crate_name]
        app_crates[provider_id] = crate_name

        provider_version = provider["manifest"].get("version")
        checker.require(
            provider_version == crate["version"],
            f"provider `{provider_id}` version {provider_version!r} "
            f"does not match crate `{crate_name}` version {crate['version']!r}",
        )

        exec_argv = provider["manifest"].get("exec") or []
        if not exec_argv:
            continue
        program = exec_argv[0]
        shipped_script = crate["directory"] / "share" / program
        checker.require(
            program in crate["bins"] or shipped_script.is_file(),
            f"provider `{provider_id}` exec `{program}` is not a binary of "
            f"crate `{crate_name}` or a shipped script ({shipped_script.relative_to(ROOT)})",
        )

    for crate in crates:
        if crate not in INFRA_CRATES and crate not in app_crates.values():
            checker.fail(
                f"crate `{crate}` is neither infrastructure nor mapped to a provider manifest"
            )

    install_text = read_text(ROOT / "install.sh", checker)
    known_match = re.search(r"KNOWN_APPS=\(([^)]*)\)", install_text)
    known_apps = re.findall(r'"([^"]+)"', known_match.group(1)) if known_match else []
    if not known_match:
        checker.fail("install.sh has no KNOWN_APPS array")
    checker.require(
        len(known_apps) == len(set(known_apps)),
        "install.sh KNOWN_APPS contains duplicates",
    )
    checker.require(
        set(known_apps) == set(providers),
        f"install.sh KNOWN_APPS ({sorted(known_apps)}) does not match "
        f"providers ({sorted(providers)})",
    )
    list_block = re.search(
        r'if \[ "\$LIST_APPS" -eq 1 \]; then(.*?)exit 0', install_text, re.S
    )
    for provider_id in providers:
        checker.require(
            list_block is not None and f"  {provider_id}" in list_block.group(1),
            f"install.sh --list-apps does not mention `{provider_id}`",
        )

    flake = read_text(ROOT / "flake.nix", checker)
    for provider_id, crate_name in app_crates.items():
        checker.require(
            re.search(rf'"--package"\s+"{re.escape(crate_name)}"', flake) is not None,
            f"flake.nix does not build crate `{crate_name}`",
        )
    for provider_id in providers:
        checker.require(
            f"providers/{provider_id}.toml" in flake,
            f"flake.nix does not install providers/{provider_id}.toml",
        )
    apps_enum = re.search(
        r"type = lib\.types\.listOf \(lib\.types\.enum \[(.*?)\]\)",
        flake,
        re.S,
    )
    if apps_enum is None:
        checker.fail("flake.nix Home Manager apps enum was not found")
    else:
        enum_apps = re.findall(r'"([^"]+)"', apps_enum.group(1))
        checker.require(
            set(enum_apps) == set(providers),
            f"flake.nix Home Manager apps ({sorted(enum_apps)}) do not match "
            f"providers ({sorted(providers)})",
        )

    package_script = read_text(ROOT / "scripts/package-release.sh", checker)
    test_installer = read_text(ROOT / "scripts/test-installer.sh", checker)
    workflow = read_text(ROOT / ".github/workflows/release.yml", checker)
    package_flag = r'(?:"--package"\s+"|--package\s+){crate}'
    for provider_id, crate_name in app_crates.items():
        program = providers[provider_id]["manifest"]["exec"][0]
        crate = crates[crate_name]
        built_target = (
            f"target/release/{program}"
            if program in crate["bins"]
            else f"crates/{crate_name}/share/{program}"
        )
        debug_target = (
            f"target/debug/{program}"
            if program in crate["bins"]
            else f"crates/{crate_name}/share/{program}"
        )
        checker.require(
            re.search(
                package_flag.format(crate=re.escape(crate_name)), package_script
            )
            is not None,
            f"scripts/package-release.sh does not build crate `{crate_name}`",
        )
        checker.require(
            built_target in package_script,
            f"scripts/package-release.sh does not install `{program}`",
        )
        checker.require(
            debug_target in test_installer,
            f"scripts/test-installer.sh does not package `{program}`",
        )
        checker.require(
            f"providers/{provider_id}.toml" in package_script,
            f"scripts/package-release.sh does not include providers/{provider_id}.toml",
        )
        checker.require(
            f"providers/{provider_id}.toml" in test_installer,
            f"scripts/test-installer.sh does not include providers/{provider_id}.toml",
        )

    for script in (
        "scripts/package-release.sh",
        "scripts/test-bundle.sh",
        "scripts/test-installer.sh",
        "scripts/check-app-sync.py",
    ):
        checker.require(script in workflow, f"release workflow does not run {script}")
    checker.require(
        "cargo test --workspace --locked" in workflow,
        "release workflow does not run workspace tests",
    )
    checker.require(
        "nix flake check" in workflow,
        "release workflow does not run nix flake check",
    )

    status = checker.finish()
    if status == 0:
        print(
            f"app sync ok: {len(providers)} apps "
            f"({', '.join(sorted(providers))})"
        )
    return status


if __name__ == "__main__":
    raise SystemExit(main())
