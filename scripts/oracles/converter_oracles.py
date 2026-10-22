# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Phase 2 gate: convert every config oracle and compare with committed snapshots.

Usage: python3 -m scripts.oracles.converter_oracles [--update] [name ...]

Each `config` entry in oracles/manifest.yaml is converted with the converter CLI. The
gate fails on any error diagnostic or on output that differs from the snapshot in
converter/test/__snapshots__/oracles/<name>.json.snap (see `digest`). Oracle files are
data: they are read by the converter and never run.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import cast

type Json = dict[str, object]

ROOT = Path(__file__).resolve().parents[2]
CACHE = ROOT / "oracles" / "cache"
SNAPSHOTS = ROOT / "converter" / "test" / "__snapshots__" / "oracles"
CLI = ROOT / "converter" / "src" / "cli.ts"


def objects(value: object) -> list[Json]:
    """Return the JSON objects in a list, dropping anything else."""
    items = cast("list[object]", value) if isinstance(value, list) else []
    return [cast("Json", v) for v in items if isinstance(v, dict)]


def members(value: object) -> Json:
    """Return a JSON object, or an empty one."""
    return cast("Json", value) if isinstance(value, dict) else {}


def config_entries() -> list[Json]:
    """Return config entries from the manifest (comments, then one JSON document)."""
    text = (ROOT / "oracles" / "manifest.yaml").read_text(encoding="utf-8")
    data: object = json.loads(re.sub(r"^#.*$", "", text, flags=re.MULTILINE))
    entries = objects(members(data).get("oracles", data))
    return [e for e in entries if e.get("kind") == "config"]


def spec_path(entry: Json) -> Path:
    """Return where fetch.py put the entry's spec."""
    name = str(entry["name"])
    spec = members(entry["spec"])
    if "path" in spec:
        return CACHE / name / str(spec["path"])
    matches = sorted((CACHE / "specs").glob(f"{name}.*"))
    if not matches:
        msg = f"{name}: spec not fetched; run python3 oracles/fetch.py --kind config"
        raise FileNotFoundError(msg)
    return matches[0]


def digest(converted: bytes) -> bytes:
    """Reduce converter output to what a snapshot may store.

    The converted spec is mostly third-party text (some under AGPL or no licence), so the
    snapshot keeps only the `x-` extensions the converter wrote, keyed by "VERB /path",
    plus a sha256 that still pins every byte. Values mirrored onto every operation (Forge
    gaps G2, G3) are stored once under "mirrored".
    """
    spec = members(json.loads(converted))
    ops: dict[str, Json] = {}
    for route, item in sorted(members(spec.get("paths")).items()):
        for verb, op in sorted(members(item).items()):
            ext = {k: v for k, v in sorted(members(op).items()) if k.startswith("x-")}
            if ext:
                ops[f"{verb.upper()} {route}"] = ext
    root = {k: v for k, v in sorted(spec.items()) if k.startswith("x-ferrule")}
    mirrored: dict[str, object] = {}
    for key in ("x-ferrule-client", "x-ferrule-models", "x-ferrule-package"):
        values = [op[key] for op in ops.values() if key in op]
        if values and all(v == values[0] for v in values):
            mirrored[key] = values[0]
            for op in ops.values():
                op.pop(key, None)
    body = {
        "sha256": hashlib.sha256(converted).hexdigest(),
        "root": root,
        "mirrored": mirrored,
        "operations": ops,
    }
    return (json.dumps(body, indent=2, ensure_ascii=False) + "\n").encode()


def convert(entry: Json, out: Path) -> tuple[int, str]:
    """Run the converter CLI; return its exit code and diagnostics output."""
    name = str(entry["name"])
    config = CACHE / name / str(entry["config_path"])
    proc = subprocess.run(  # noqa: S603 - fixed argv, our own converter
        ["node", str(CLI), str(spec_path(entry)), str(config), "-o", str(out)],  # noqa: S607
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode, (proc.stdout + proc.stderr).strip()


def main(argv: list[str] | None = None) -> int:
    """Convert, compare or update snapshots, and report."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--update", action="store_true", help="rewrite snapshots")
    parser.add_argument("names", nargs="*")
    args = parser.parse_args(argv)

    subprocess.run(  # noqa: S603 - fixed argv, our own fetch script
        [sys.executable, str(ROOT / "oracles" / "fetch.py"), "--kind", "config"],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    SNAPSHOTS.mkdir(parents=True, exist_ok=True)
    failed: list[str] = []
    for entry in config_entries():
        name = str(entry["name"])
        if args.names and name not in args.names:
            continue
        out = ROOT / ".cache" / "converter-oracles" / f"{name}.json"
        out.parent.mkdir(parents=True, exist_ok=True)
        code, log = convert(entry, out)
        snap = SNAPSHOTS / f"{name}.json.snap"
        if code != 0:
            status = f"FAIL (exit {code})"
        elif args.update:
            snap.write_bytes(digest(out.read_bytes()))
            status = "updated"
        elif not snap.exists():
            status = "FAIL (no snapshot; run with --update)"
        elif snap.read_bytes() != digest(out.read_bytes()):
            status = "FAIL (differs from snapshot)"
        else:
            status = "ok"
        sys.stdout.write(f"{status:<40} {name}\n")
        if log and code != 0:
            sys.stdout.write("".join(f"    {line}\n" for line in log.splitlines()[:40]))
        if status.startswith("FAIL"):
            failed.append(name)
    if failed:
        sys.stdout.write(f"converter:oracles failed: {', '.join(failed)}\n")
        return 1
    sys.stdout.write("converter:oracles: OK\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
