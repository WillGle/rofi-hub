#!/usr/bin/env python3
"""Merge per-architecture release indexes into one signed index.

Each native bundle (x86_64 or aarch64) writes `index-<arch>.json` with its own
tarball URLs and checksums. The release job must keep both mappings; writing
both bundles to the same `index.json` would lose one architecture.

Usage:
    merge-release-index.py OUTPUT INDEX...

The merged index uses schema 2 and flat per-architecture keys:

    "url_x86_64": "rofi-calendar-1.2.0-x86_64.tar.gz",
    "sha256_x86_64": "...",
    "url_aarch64": "rofi-calendar-1.2.0-aarch64.tar.gz",
    "sha256_aarch64": "..."

Schema 1 inputs are accepted so a single-architecture local bundle can still be
merged; the architecture is inferred from the tarball suffix.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys
from typing import NoReturn

ARCHES = ("aarch64", "x86_64")
SUFFIX = re.compile(r"-(aarch64|x86_64)\.tar\.gz$")


def fail(message: str) -> NoReturn:
    print(f"merge-release-index: {message}", file=sys.stderr)
    raise SystemExit(1)


def load(path: pathlib.Path) -> dict:
    try:
        index = json.loads(path.read_text())
    except OSError as error:
        fail(f"cannot read {path}: {error}")
    except json.JSONDecodeError as error:
        fail(f"{path} is not valid JSON: {error}")
    if index.get("schema") not in (1, 2):
        fail(f"{path}: unsupported index schema {index.get('schema')!r}")
    if not isinstance(index.get("installer", {}).get("version"), str):
        fail(f"{path}: installer.version is missing")
    if not isinstance(index.get("apps"), list) or not index["apps"]:
        fail(f"{path}: apps must be a non-empty array")
    return index


def artifacts(entry: dict, path: pathlib.Path) -> dict[str, tuple[str, str]]:
    app_id = entry.get("id")
    found: dict[str, tuple[str, str]] = {}
    for arch in ARCHES:
        url = entry.get(f"url_{arch}")
        sha = entry.get(f"sha256_{arch}")
        if url is None and sha is None:
            continue
        if not isinstance(url, str) or not isinstance(sha, str) or not url or not sha:
            fail(f"{path}: {app_id} has an incomplete {arch} artifact")
        found[arch] = (url, sha)

    if found:
        return found

    # Backward-compatible schema 1 entry.
    url = entry.get("url")
    sha = entry.get("sha256")
    match = SUFFIX.search(url) if isinstance(url, str) else None
    if not match:
        fail(
            f"{path}: {app_id} has no per-architecture artifact and its URL "
            "does not end in -x86_64.tar.gz or -aarch64.tar.gz"
        )
    if not isinstance(sha, str) or not sha:
        fail(f"{path}: {app_id} is missing sha256")
    found[match.group(1)] = (url, sha)
    return found


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    output = pathlib.Path(argv[1])
    inputs = [pathlib.Path(argument) for argument in argv[2:]]

    merged: dict[str, dict] = {}
    order: list[str] = []
    expected_arches: set[str] = set()
    installer_version: str | None = None

    for path in inputs:
        index = load(path)
        version = index["installer"]["version"]
        if installer_version is not None and installer_version != version:
            fail(
                f"installer version mismatch: {installer_version} vs {version} "
                f"in {path}"
            )
        installer_version = version

        for entry in index["apps"]:
            app_id = entry.get("id")
            if not isinstance(app_id, str) or not app_id:
                fail(f"{path}: app entry is missing id")
            version = entry.get("version")
            if not isinstance(version, str) or not version:
                fail(f"{path}: {app_id} is missing version")
            entry_artifacts = artifacts(entry, path)
            expected_arches.update(entry_artifacts)

            current = merged.get(app_id)
            if current is None:
                current = {
                    "id": app_id,
                    "version": version,
                    "requires": list(entry.get("requires", [])),
                    "optional": bool(entry.get("optional", False)),
                    "artifacts": {},
                }
                merged[app_id] = current
                order.append(app_id)
            else:
                if current["version"] != version:
                    fail(f"{app_id}: version mismatch across indexes")
                if current["requires"] != list(entry.get("requires", [])):
                    fail(f"{app_id}: requires mismatch across indexes")
                if current["optional"] != bool(entry.get("optional", False)):
                    fail(f"{app_id}: optional mismatch across indexes")

            for arch, (url, sha) in entry_artifacts.items():
                previous = current["artifacts"].get(arch)
                if previous is not None and previous != (url, sha):
                    fail(f"{app_id}: conflicting {arch} artifact across indexes")
                current["artifacts"][arch] = (url, sha)

    for app_id in order:
        present = set(merged[app_id]["artifacts"])
        missing = expected_arches - present
        if missing:
            fail(
                f"{app_id}: missing artifacts for "
                f"{', '.join(sorted(missing))} (found {', '.join(sorted(present))})"
            )

    apps = []
    for app_id in order:
        entry = merged[app_id]
        apps.append(
            {
                "id": entry["id"],
                "version": entry["version"],
                "requires": entry["requires"],
                "optional": entry["optional"],
                "artifacts": entry["artifacts"],
            }
        )

    out = {
        "schema": 2,
        "architectures": sorted(expected_arches),
        "installer": {"version": installer_version or "0.0.0"},
        "apps": apps,
    }

    # Flatten each artifact map into schema-2 keys understood by install.sh's
    # dependency-free JSON reader.
    for entry in out["apps"]:
        artifacts_map = entry.pop("artifacts")
        for arch, (url, sha) in sorted(artifacts_map.items()):
            entry[f"url_{arch}"] = url
            entry[f"sha256_{arch}"] = sha

    output.write_text(json.dumps(out, indent=2, sort_keys=True) + "\n")
    print(
        f"merge-release-index: wrote {output} "
        f"({len(apps)} apps, {', '.join(out['architectures'])})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
