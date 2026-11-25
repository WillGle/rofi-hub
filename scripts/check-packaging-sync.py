#!/usr/bin/env python3
"""Check that packaging metadata, recipes, and release assets stay in sync.

This is the guard against version drift between Cargo, providers, the install
manifest, distro recipes, and the release workflow.
"""

from __future__ import annotations

import hashlib
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
errors: list[str] = []
warnings: list[str] = []


def fail(message: str) -> None:
    errors.append(message)


def warn(message: str) -> None:
    warnings.append(message)


def read(path: str) -> str:
    file = ROOT / path
    if not file.is_file():
        fail(f"missing file: {path}")
        return ""
    return file.read_text(encoding="utf-8")


def workspace_version() -> str:
    data = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    return data["workspace"]["package"]["version"]


def screen_time_version() -> str:
    data = tomllib.loads((ROOT / "crates/screen-time/Cargo.toml").read_text(encoding="utf-8"))
    return data["package"]["version"]


def require(pattern: str, text: str, label: str, flags: int = 0) -> re.Match[str] | None:
    match = re.search(pattern, text, flags)
    if not match:
        fail(f"{label}: pattern not found: {pattern}")
    return match


def check_recipe_version(
    path: str, version: str, pattern: str = r"pkgver=([^\s]+)", label: str | None = None
) -> None:
    text = read(path)
    if not text:
        return
    match = re.search(pattern, text, re.MULTILINE)
    if not match:
        fail(f"{path}: cannot find version ({pattern})")
        return
    if match.group(1) != version:
        fail(f"{path}: version {match.group(1)!r} != workspace {version!r}")
    if label and label not in text:
        fail(f"{path}: missing expected marker {label!r}")


def main() -> int:
    version = workspace_version()
    st_version = screen_time_version()

    # Provider manifests and crates agree.
    for path in sorted((ROOT / "providers").glob("*.toml")):
        data = tomllib.loads(path.read_text(encoding="utf-8"))
        expected = st_version if data["id"] == "screen-time" else version
        if data.get("version") != expected:
            fail(f"providers/{path.name}: version {data.get('version')!r} != {expected!r}")
        if path.stem != data["id"]:
            fail(f"providers/{path.name}: filename does not match id {data['id']!r}")

    # Distro recipes track the same tag.
    check_recipe_version("packaging/arch/PKGBUILD", version)
    check_recipe_version("packaging/arch/rofi-hub-bin/PKGBUILD", version)
    check_recipe_version("packaging/alpine/APKBUILD", version)
    check_recipe_version("packaging/void/template", version, r"^version=([^\s]+)")
    check_recipe_version("packaging/slackbuilds/rofi-hub.info", version, r'^VERSION="([^"]+)"')
    check_recipe_version("packaging/debian/changelog", f"{version}-1", r"^rofi-hub \(([^)]+)\)")

    for path in ("packaging/fedora/rofi-hub.spec", "packaging/opensuse/rofi-hub.spec", "packaging/alt/rofi-hub.spec"):
        text = read(path)
        match = re.search(r"^Version:\s*(\S+)", text, re.MULTILINE)
        if not match:
            fail(f"{path}: missing Version")
        elif match.group(1) != version:
            fail(f"{path}: version {match.group(1)!r} != workspace {version!r}")

    ebuild = f"packaging/gentoo/rofi-hub-{version}.ebuild"
    if not (ROOT / ebuild).is_file():
        fail(f"missing ebuild for {version}: {ebuild}")
    else:
        text = read(ebuild)
        if 'SRC_URI="' not in text or "rofi-hub-${PV}-source.tar.gz" not in text:
            fail(f"{ebuild}: source URI does not use the packaged source asset")
        if "packaging/install-tree.sh" not in text:
            fail(f"{ebuild}: does not call packaging/install-tree.sh")

    # Install-tree recipes must use DESTDIR/PREFIX and the shared installer.
    for path in (
        "packaging/arch/PKGBUILD",
        "packaging/arch/rofi-hub-bin/PKGBUILD",
        "packaging/arch/rofi-hub-git/PKGBUILD",
        "packaging/alpine/APKBUILD",
        "packaging/fedora/rofi-hub.spec",
        "packaging/opensuse/rofi-hub.spec",
        "packaging/alt/rofi-hub.spec",
    ):
        text = read(path)
        if "install-tree.sh" not in text:
            fail(f"{path}: does not call packaging/install-tree.sh")
        if "DESTDIR" not in text or "PREFIX=/usr" not in text:
            fail(f"{path}: missing DESTDIR/PREFIX=/usr install")

    # Source/checksum placeholders are called out, not silently accepted.
    arch_bin = read("packaging/arch/rofi-hub-bin/PKGBUILD")
    if "sha256sums_x86_64=('SKIP')" in arch_bin:
        warn("packaging/arch/rofi-hub-bin/PKGBUILD: replace SKIP checksums after publishing")

    # MANIFEST is exactly the --dry-run output.
    env = os.environ.copy()
    env.update({"PREFIX": "/usr", "HOME": "/nonexistent", "XDG_DATA_HOME": "/nonexistent"})
    run = subprocess.run(
        ["packaging/install-tree.sh", "--dry-run"],
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
    )
    if run.returncode != 0:
        fail(f"install-tree --dry-run failed: {run.stderr.strip()}")
    else:
        expected = [line.strip() for line in run.stdout.splitlines() if line.strip()]
        expected.sort(key=lambda line: line.split(maxsplit=1)[1])
        manifest_text = read("packaging/MANIFEST")
        manifest = [line.strip() for line in manifest_text.splitlines() if line.strip()]
        paths = [line.split(maxsplit=1)[1] for line in manifest if " " in line]
        if paths != sorted(paths):
            fail("packaging/MANIFEST is not sorted by path")
        if len(paths) != len(set(paths)):
            fail("packaging/MANIFEST has duplicate paths")
        for line in manifest:
            mode = line.split(maxsplit=1)[0] if " " in line else ""
            if mode not in {"644", "755"}:
                fail(f"packaging/MANIFEST: invalid mode in {line!r}")
        if not manifest:
            fail("packaging/MANIFEST is empty")
        elif expected != manifest:
            missing = sorted(set(expected) - set(manifest))
            extra = sorted(set(manifest) - set(expected))
            fail(f"packaging/MANIFEST is out of date (missing: {missing[:5]}, extra: {extra[:5]})")
        else:
            print(f"check-packaging-sync: manifest matches {len(manifest)} files")

    # If the release source archive has been built locally, every recipe must
    # carry its checksum. CI builds it before this check.
    archive = ROOT / "dist" / f"rofi-hub-{version}-source.tar.gz"
    if archive.is_file():
        data = archive.read_bytes()
        sha256 = hashlib.sha256(data).hexdigest()
        sha512 = hashlib.sha512(data).hexdigest()
        md5 = hashlib.md5(data).hexdigest()
        blake = hashlib.blake2b(data, digest_size=64).hexdigest()
        size = len(data)
        checks = [
            ("packaging/arch/PKGBUILD", sha256),
            ("packaging/arch/.SRCINFO", sha256),
            ("packaging/alpine/APKBUILD", sha512),
            ("packaging/void/template", sha256),
            ("packaging/slackbuilds/rofi-hub.info", md5),
            ("packaging/gentoo/Manifest", blake),
        ]
        for path, digest in checks:
            if digest not in read(path):
                fail(f"{path}: checksum does not match {archive.name}")
        if f"DIST rofi-hub-{version}.tar.gz {size} " not in read("packaging/gentoo/Manifest"):
            fail("packaging/gentoo/Manifest: source size does not match the archive")
        print(f"check-packaging-sync: source checksums match {archive.name}")

    # Desktop and AppStream assets agree.
    desktop = read("packaging/rofi-hub.desktop")
    if "Exec=rofi-hub" not in desktop or "Icon=rofi-hub" not in desktop:
        fail("packaging/rofi-hub.desktop: Exec/Icon mismatch")
    if not (ROOT / "packaging/rofi-hub.svg").is_file():
        fail("packaging/rofi-hub.svg is missing")
    metainfo = read("packaging/rofi-hub.metainfo.xml")
    if "<launchable type=\"desktop-id\">rofi-hub.desktop</launchable>" not in metainfo:
        fail("packaging/rofi-hub.metainfo.xml: launchable does not reference rofi-hub.desktop")
    if "<project_license>MIT</project_license>" not in metainfo:
        fail("packaging/rofi-hub.metainfo.xml: license mismatch")

    # AUR variants declare replaces/conflicts.
    for variant in ("packaging/arch/rofi-hub-bin/PKGBUILD", "packaging/arch/rofi-hub-git/PKGBUILD"):
        text = read(variant)
        if "provides=('rofi-hub')" not in text or "conflicts=" not in text:
            fail(f"{variant}: missing provides/conflicts for the AUR variant")

    # Docs and workflow must describe the same gates.
    docs = read("docs/PACKAGING.md")
    for needle in (
        "Dependency matrix",
        "Install layout",
        "Split packages",
        "Service managers",
        "Provider and theme resolution",
        "Release binary baseline",
        "Release artifacts",
        "Per-distro commands",
        "Packaging status",
        "Release checklist",
        "repology.org",
    ):
        if needle not in docs:
            fail(f"docs/PACKAGING.md: missing section {needle!r}")
    if "rofi-hub" not in read("README.md"):
        fail("README.md: missing project name")

    workflow = read(".github/workflows/release.yml")
    for job in ("install-tree:", "distro-build:", "artifacts:", "release-smoke:"):
        if job not in workflow:
            fail(f".github/workflows/release.yml: missing job {job.rstrip(':')}")
    for path in (
        "scripts/test-install-tree.sh",
        "scripts/build-package-artifacts.sh",
        "scripts/test-package-artifacts.sh",
    ):
        if path not in workflow:
            fail(f".github/workflows/release.yml: does not run {path}")
    for asset in ("*.deb", "*.rpm", "*.apk"):
        if asset not in workflow:
            fail(f".github/workflows/release.yml: release assets miss {asset}")
    if "rofi-hub-*-source.tar.gz" not in workflow:
        fail(".github/workflows/release.yml: source asset is not uploaded")
    if "ubuntu-24.04-arm" not in workflow:
        fail(".github/workflows/release.yml: install tree misses the aarch64 runner")
    # The tag release job must gate on package builds and attach every artifact
    # type; uploading them in another job alone is not enough.
    marker = "\n  release:\n"
    if marker not in workflow:
        fail(".github/workflows/release.yml: missing release job")
    else:
        release_job = workflow.split(marker, 1)[1]
        for job in ("distro-build", "artifacts", "apk-package", "release-smoke"):
            if job not in release_job:
                fail(f".github/workflows/release.yml: release does not gate on {job}")
        for asset in ("*.deb", "*.rpm", "*.apk"):
            if asset not in release_job:
                fail(f".github/workflows/release.yml: release job does not attach {asset}")

    # Executable bits on scripts that package managers invoke.
    for path in (
        "packaging/install-tree.sh",
        "scripts/build-package-artifacts.sh",
        "scripts/test-package-artifacts.sh",
        "scripts/test-install-tree.sh",
        "packaging/debian/rules",
        "packaging/slackbuilds/rofi-hub.SlackBuild",
    ):
        file = ROOT / path
        if file.is_file() and not (file.stat().st_mode & 0o111):
            fail(f"{path}: not executable")

    for message in warnings:
        print(f"check-packaging-sync: warning: {message}", file=sys.stderr)
    if errors:
        for message in errors:
            print(f"check-packaging-sync: FAIL: {message}", file=sys.stderr)
        return 1
    print(f"check-packaging-sync: ok (rofi-hub {version}, screen-time {st_version})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
