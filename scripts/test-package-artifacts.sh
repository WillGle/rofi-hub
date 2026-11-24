#!/usr/bin/env python3
"""Validate release .deb, .rpm, and .apk artifacts against packaging/MANIFEST.

Every artifact must contain exactly the files produced by
packaging/install-tree.sh, with the expected package metadata. Run after
scripts/build-package-artifacts.sh.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "packaging/MANIFEST"


def die(message: str) -> None:
    print(f"test-package-artifacts: FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def ok(message: str) -> None:
    print(f"test-package-artifacts: ok: {message}")


def run(*args: str) -> str:
    result = subprocess.run(args, capture_output=True, text=True)
    if result.returncode != 0:
        die(f"{' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout


def expected_files() -> set[str]:
    files = set()
    for line in MANIFEST.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        mode, path = line.split(maxsplit=1)
        if mode not in {"644", "755"}:
            die(f"bad mode in MANIFEST: {line}")
        files.add(path)
    if not files:
        die("MANIFEST is empty")
    return files


def compare(label: str, actual: set[str], expected: set[str]) -> None:
    missing = sorted(expected - actual)
    extra = sorted(actual - expected)
    if missing or extra:
        if missing:
            print(f"  missing from {label}: {missing}", file=sys.stderr)
        if extra:
            print(f"  extra in {label}: {extra}", file=sys.stderr)
        die(f"{label} file list does not match MANIFEST")
    ok(f"{label} contains all {len(expected)} manifest files")


def version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^\[workspace\.package\][^\[]*?^version = "([^"]+)"', text, re.M | re.S)
    if not match:
        die("cannot read workspace version")
    return match.group(1)


def check_deb(path: Path, expected: set[str], want: str) -> None:
    if not path.is_file():
        die(f"missing {path}")
    fields = {}
    for line in run("dpkg-deb", "--field", str(path)).splitlines():
        if ":" in line and not line.startswith(" "):
            key, value = line.split(":", 1)
            fields[key.strip()] = value.strip()
    if fields.get("Package") != "rofi-hub":
        die(f"{path.name}: package is {fields.get('Package')!r}")
    if fields.get("Version") != f"{want}-1":
        die(f"{path.name}: version is {fields.get('Version')!r}")
    if "rofi" not in fields.get("Depends", "") or "bash" not in fields.get("Depends", ""):
        die(f"{path.name}: unexpected Depends {fields.get('Depends')!r}")
    listing = run("dpkg-deb", "--contents", str(path))
    actual = set()
    modes = {}
    for line in listing.splitlines():
        match = re.search(r"^(.)[rwx-]{9}.*?\s\./(.+)$", line)
        if not match:
            continue
        kind, name = match.groups()
        if kind != "d":
            actual.add("/" + name)
            modes["/" + name] = line[1:10]
    compare("deb", actual, expected)
    for path_name, mode in modes.items():
        if path_name.startswith("/usr/bin/"):
            if mode != "rwxr-xr-x":
                die(f"{path.name}: {path_name} mode is {mode}, expected rwxr-xr-x")
    ok(f"deb metadata: {fields.get('Package')} {fields.get('Version')} {fields.get('Architecture')}")


def check_rpm(path: Path, expected: set[str], want: str) -> None:
    if not path.is_file():
        die(f"missing {path}")
    query = "%{NAME}\n%{VERSION}\n%{RELEASE}\n%{ARCH}\n%{LICENSE}\n"
    values = run("rpm", "-qp", "--qf", query, str(path)).splitlines()
    if values[:3] != ["rofi-hub", want, "1"] or values[4] != "MIT":
        die(f"{path.name}: unexpected rpm metadata {values}")
    if values[3] not in {"x86_64", "aarch64"}:
        die(f"{path.name}: unexpected rpm architecture {values[3]!r}")
    actual = set()
    modes = {}
    listing = run("rpm", "-qp", "--qf", "[%{FILEMODES} %{FILENAMES}\\n]", str(path))
    for line in listing.splitlines():
        if " " not in line:
            continue
        raw_mode, name = line.split(" ", 1)
        mode = int(raw_mode, 10)
        if mode & 0o170000 == 0o040000:
            continue
        actual.add(name)
        modes[name] = mode & 0o777
    compare("rpm", actual, expected)
    for name, mode in modes.items():
        if name.startswith("/usr/bin/") and mode != 0o755:
            die(f"{path.name}: {name} mode is {mode:o}, expected 755")
    ok(f"rpm metadata: rofi-hub {want}-1 {values[3]}")


def check_apk(path: Path, expected: set[str], want: str) -> None:
    if not path.is_file():
        die(f"missing {path}")
    with tarfile.open(path, "r:gz") as archive:
        members = archive.getmembers()
        names = {member.name for member in members}
        if ".PKGINFO" not in names:
            die(f"{path.name}: missing .PKGINFO")
        pkginfo = archive.extractfile(".PKGINFO").read().decode()  # type: ignore[union-attr]
        file_names = {
            member.name for member in members if member.isfile() and member.name != ".PKGINFO"
        }
    fields = {}
    depends = []
    for line in pkginfo.splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            key, value = key.strip(), value.strip()
            if key == "depend":
                depends.append(value)
            else:
                fields[key] = value
    if fields.get("pkgname") != "rofi-hub":
        die(f"{path.name}: pkgname is {fields.get('pkgname')!r}")
    if fields.get("pkgver") != f"{want}-r0":
        die(f"{path.name}: pkgver is {fields.get('pkgver')!r}")
    if "rofi" not in depends or "bash" not in depends:
        die(f"{path.name}: unexpected depends {depends!r}")
    actual = {"/" + name for name in file_names}
    compare("apk", actual, expected)
    ok(f"apk metadata: rofi-hub {fields.get('pkgver')}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifacts", nargs="*", help="artifact paths (default: dist/*.deb dist/*.rpm dist/*.apk)")
    args = parser.parse_args()
    expected = expected_files()
    want = version()

    paths = [Path(item) for item in args.artifacts]
    if not paths:
        paths = sorted(ROOT.glob("dist/*.deb")) + sorted(ROOT.glob("dist/*.rpm")) + sorted(ROOT.glob("dist/*.apk"))
    if not paths:
        die("no artifacts found")

    seen = set()
    for path in paths:
        if path.suffix == ".deb":
            check_deb(path, expected, want)
            seen.add("deb")
        elif path.suffix == ".rpm":
            check_rpm(path, expected, want)
            seen.add("rpm")
        elif path.suffix == ".apk":
            check_apk(path, expected, want)
            seen.add("apk")
        else:
            die(f"unknown artifact type: {path}")

    missing = {"deb", "rpm", "apk"} - seen
    if missing:
        die(f"missing artifact types: {', '.join(sorted(missing))}")
    print("test-package-artifacts: all checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
