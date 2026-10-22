# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Turn a ``ferrule verify --json`` report into a semver label and a pass/fail verdict.

Any breaking change is ``major``, any additive change is ``minor``, anything else is
``patch``. With enforcement on, a breaking change fails unless the PR carries the
override label ``ferrule:allow-breaking``.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Literal

if TYPE_CHECKING:
    from collections.abc import Iterable, Sequence

ALLOW_BREAKING = "ferrule:allow-breaking"

Bump = Literal["major", "minor", "patch"]


class ReportError(ValueError):
    """The verify report is not in the expected shape."""


@dataclass(frozen=True)
class Verdict:
    """What the action does with one report."""

    label: Bump
    breaking: int
    additive: int
    allowed: bool
    failed: bool

    def outputs(self) -> dict[str, str]:
        """Return the step outputs, as written to ``$GITHUB_OUTPUT``."""
        return {
            "label": self.label,
            "breaking": str(self.breaking),
            "additive": str(self.additive),
        }


def _count(report: dict[str, object], key: str) -> int:
    value = report.get(f"{key}_count")
    if value is None:
        items = report.get(key, [])
        if not isinstance(items, list):
            msg = f"report field {key!r} must be a list"
            raise ReportError(msg)
        return len(items)  # pyright: ignore[reportUnknownArgumentType]
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        msg = f"report field {key}_count must be a non-negative integer"
        raise ReportError(msg)
    return value


def parse_report(text: str) -> tuple[int, int]:
    """Return ``(breaking, additive)`` counts from the JSON text of a verify report."""
    try:
        data: object = json.loads(text)
    except json.JSONDecodeError as exc:
        msg = f"report is not JSON: {exc}"
        raise ReportError(msg) from exc
    if not isinstance(data, dict):
        msg = "report must be a JSON object"
        raise ReportError(msg)
    raw: dict[object, object] = data  # pyright: ignore[reportUnknownVariableType]
    report = {str(k): v for k, v in raw.items()}
    return _count(report, "breaking"), _count(report, "additive")


def bump(breaking: int, additive: int) -> Bump:
    """Return the semver bump for these counts."""
    if breaking:
        return "major"
    if additive:
        return "minor"
    return "patch"


def decide(breaking: int, additive: int, labels: Iterable[str], *, enforce: bool) -> Verdict:
    """Combine counts, PR labels and the ``verify`` input into a verdict."""
    allowed = ALLOW_BREAKING in {label.strip() for label in labels}
    return Verdict(
        label=bump(breaking, additive),
        breaking=breaking,
        additive=additive,
        allowed=allowed,
        failed=enforce and breaking > 0 and not allowed,
    )


def _bool(value: str) -> bool:
    match value.strip().lower():
        case "true":
            return True
        case "false":
            return False
        case _:
            msg = f"expected true or false, got {value!r}"
            raise argparse.ArgumentTypeError(msg)


def build_parser() -> argparse.ArgumentParser:
    """Return the CLI parser."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path, help="ferrule verify --json output")
    parser.add_argument("--enforce", type=_bool, default=True, help="fail on unlabeled breaking")
    parser.add_argument("--labels", default="", help="comma-separated labels already on the PR")
    parser.add_argument("--github-output", type=Path, help="append outputs here ($GITHUB_OUTPUT)")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    """Print the verdict, write outputs, and return 1 on an unlabeled breaking change."""
    args = build_parser().parse_args(argv)
    try:
        breaking, additive = parse_report(args.report.read_text(encoding="utf-8"))
    except (OSError, ReportError) as exc:
        sys.stderr.write(f"label: {exc}\n")
        return 2
    verdict = decide(breaking, additive, args.labels.split(","), enforce=args.enforce)
    lines = [f"{k}={v}" for k, v in verdict.outputs().items()]
    sys.stdout.write("\n".join(lines) + "\n")
    if args.github_output is not None:
        with args.github_output.open("a", encoding="utf-8") as out:
            out.write("\n".join(lines) + "\n")
    if verdict.failed:
        sys.stderr.write(
            f"::error::{breaking} breaking change(s); add the {ALLOW_BREAKING} label to accept\n",
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
