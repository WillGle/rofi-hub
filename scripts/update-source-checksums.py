#!/usr/bin/env python3
"""Propagate the packaged source-archive checksums to every recipe.

Run after scripts/package-source.sh. Keeps Arch, Alpine, Void, SlackBuilds,
Gentoo, and check-packaging-sync in lockstep with dist/rofi-hub-<ver>-source.tar.gz.
"""

from __future__ import annotations

import hashlib
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def version() -> str:
    data = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    return data["workspace"]["package"]["version"]


def digests(path: Path) -> dict[str, str]:
    data = path.read_bytes()
    return {
        "md5": hashlib.md5(data).hexdigest(),
        "sha256": hashlib.sha256(data).hexdigest(),
        "sha512": hashlib.sha512(data).hexdigest(),
        "blake2b": hashlib.blake2b(data, digest_size=64).hexdigest(),
        "size": str(len(data)),
    }


def replace_once(path: Path, pattern: str, replacement: str) -> None:
    text = path.read_text(encoding="utf-8")
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.MULTILINE)
    if count != 1:
        raise SystemExit(f"{path}: expected one match for {pattern!r}, got {count}")
    path.write_text(updated, encoding="utf-8")
    print(f"updated {path.relative_to(ROOT)}")


def main() -> int:
    ver = version()
    archive = ROOT / "dist" / f"rofi-hub-{ver}-source.tar.gz"
    if not archive.is_file():
        raise SystemExit(f"missing source archive: {archive} (run scripts/package-source.sh)")
    sums = digests(archive)

    replace_once(ROOT / "packaging/arch/PKGBUILD",
                 r"sha256sums=\('[0-9a-f]{64}'\)",
                 f"sha256sums=('{sums['sha256']}')")
    replace_once(ROOT / "packaging/arch/.SRCINFO",
                 r"^\tsha256sums = [0-9a-f]{64}$",
                 f"\tsha256sums = {sums['sha256']}")
    replace_once(ROOT / "packaging/alpine/APKBUILD",
                 r'^sha512sums="[0-9a-f]{128}  rofi-hub-\$pkgver\.tar\.gz"$',
                 f'sha512sums="{sums["sha512"]}  rofi-hub-$pkgver.tar.gz"')
    replace_once(ROOT / "packaging/void/template",
                 r"^checksum=[0-9a-f]{64}$",
                 f"checksum={sums['sha256']}")
    replace_once(ROOT / "packaging/slackbuilds/rofi-hub.info",
                 r'^MD5SUM="[0-9a-f]{32}"$',
                 f'MD5SUM="{sums["md5"]}"')
    replace_once(ROOT / "packaging/gentoo/Manifest",
                 rf"^DIST rofi-hub-{re.escape(ver)}\.tar\.gz .*$",
                 f"DIST rofi-hub-{ver}.tar.gz {sums['size']} "
                 f"BLAKE2B {sums['blake2b']} SHA512 {sums['sha512']}")
    print(f"source checksums: sha256={sums['sha256']}")
    print(f"                  sha512={sums['sha512']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
