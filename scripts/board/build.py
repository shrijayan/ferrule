# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Build ``docs/site/src/data/fidelity.json`` from ``reports/parity/*.json``.

One row per oracle. Output is deterministic: rows are sorted by name, and the generated date
comes from the report (``generated``) or from ``--generated``, never from the clock.

Usage: python -m scripts.board.build [--reports DIR] [--manifest FILE] [--out FILE]
                                     [--generated YYYY-MM-DD]
       python -m scripts.board.build --list-oracles
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import cast

from oracles.fetch import Oracle, parse_manifest

ROOT = Path(__file__).resolve().parents[2]


class BoardError(Exception):
    """A report is missing a field or has the wrong type."""


@dataclass(frozen=True, slots=True)
class Row:
    """One line of the fidelity board."""

    name: str
    repo: str | None
    sha: str
    breaking: int
    additive: int
    closeness: float | None
    tests_passed: int
    tests_failed: int
    tests_status: str
    deterministic: bool
    generated: str | None


def _obj(raw: object, where: str) -> dict[str, object]:
    if not isinstance(raw, dict):
        msg = f"{where}: expected an object"
        raise BoardError(msg)
    return cast("dict[str, object]", raw)


def _int(d: dict[str, object], key: str, where: str) -> int:
    value = d.get(key)
    if isinstance(value, bool) or not isinstance(value, int):
        msg = f"{where}.{key}: expected an integer"
        raise BoardError(msg)
    return value


def _str(d: dict[str, object], key: str, where: str) -> str:
    value = d.get(key)
    if not isinstance(value, str):
        msg = f"{where}.{key}: expected a string"
        raise BoardError(msg)
    return value


def build_row(report: object, repos: dict[str, str], generated: str | None) -> Row:
    """Turn one parity report into a board row; ``repos`` maps sdk oracle name to owner/repo."""
    top = _obj(report, "report")
    oracle = _str(top, "oracle", "report")
    sdk = _obj(top.get("sdk"), f"{oracle}.sdk")
    api = _obj(top.get("api"), f"{oracle}.api")
    gen = _obj(top.get("generate"), f"{oracle}.generate")
    closeness = top.get("closeness")
    if closeness is not None and (
        isinstance(closeness, bool) or not isinstance(closeness, int | float)
    ):
        msg = f"{oracle}.closeness: expected a number"
        raise BoardError(msg)
    tests_raw = top.get("tests")
    tests = _obj(tests_raw, f"{oracle}.tests") if tests_raw is not None else {}
    passed = tests.get("passed", 0)
    failed = tests.get("failed", 0)
    errors = tests.get("errors", 0)
    status = tests.get("status", "skipped")
    stamp = top.get("generated", generated)
    return Row(
        name=oracle,
        repo=repos.get(_str(sdk, "name", f"{oracle}.sdk")),
        sha=_str(sdk, "sha", f"{oracle}.sdk"),
        breaking=_int(api, "breaking", f"{oracle}.api"),
        additive=_int(api, "additive", f"{oracle}.api"),
        closeness=None if closeness is None else round(float(closeness), 2),
        tests_passed=passed if isinstance(passed, int) else 0,
        tests_failed=(failed if isinstance(failed, int) else 0)
        + (errors if isinstance(errors, int) else 0),
        tests_status=status if isinstance(status, str) else "skipped",
        deterministic=gen.get("deterministic") is True,
        generated=stamp if isinstance(stamp, str) else None,
    )


def build_board(reports: dict[str, object], oracles: list[Oracle], generated: str | None) -> str:
    """Render the board JSON (sorted, trailing newline) from parsed reports keyed by file name."""
    repos = {o.name: o.repo for o in oracles if o.kind == "sdk"}
    rows = [build_row(reports[key], repos, generated) for key in sorted(reports)]
    rows.sort(key=lambda r: r.name)
    return json.dumps([asdict(r) for r in rows], indent=2) + "\n"


def config_groups(oracles: list[Oracle]) -> list[str]:
    """Return the groups that have a config oracle, i.e. the ones parity can regenerate."""
    return sorted({o.group for o in oracles if o.kind == "config"})


def main(argv: list[str] | None = None) -> int:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description="Build the fidelity board data file.")
    parser.add_argument("--reports", type=Path, default=ROOT / "reports" / "parity")
    parser.add_argument("--manifest", type=Path, default=ROOT / "oracles" / "manifest.yaml")
    parser.add_argument("--out", type=Path, default=ROOT / "docs/site/src/data/fidelity.json")
    parser.add_argument("--generated", help="date for reports that carry none (YYYY-MM-DD)")
    parser.add_argument("--list-oracles", action="store_true", help="print config groups")
    args = parser.parse_args(argv)
    oracles = parse_manifest(cast("Path", args.manifest).read_text(encoding="utf-8"))
    if args.list_oracles:
        sys.stdout.write("".join(f"{g}\n" for g in config_groups(oracles)))
        return 0
    reports_dir = cast("Path", args.reports)
    reports: dict[str, object] = {
        p.name: json.loads(p.read_text(encoding="utf-8"))
        for p in sorted(reports_dir.glob("*.json"))
    }
    try:
        board = build_board(reports, oracles, cast("str | None", args.generated))
    except BoardError as exc:
        sys.stderr.write(f"board: {exc}\n")
        return 1
    out = cast("Path", args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(board, encoding="utf-8")
    sys.stdout.write(f"board: {len(reports)} oracle(s) -> {out}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
