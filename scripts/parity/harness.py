# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Pure helpers for the parity harness (scripts/parity/run.py): no file or network access."""

from __future__ import annotations

import re
import xml.etree.ElementTree as ET  # parses our own pytest junit output
from dataclasses import asdict, dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence
    from pathlib import Path

    from oracles.fetch import Oracle

MOCK_HOST = "127.0.0.1"
MOCK_PORT = 4010
# Mock servers, pinned here (never taken from oracle files). Stainless SDKs generated before
# 2026 use Prism; later ones use Steady. scripts/mock in the oracle tells us which one.
STEADY = ("npm", "exec", "--yes", "--package=@stdy/cli@0.22.1", "--", "steady")
STEADY_FLAGS = (
    "--validator-query-array-format=repeat",
    "--validator-form-array-format=repeat",
    "--validator-query-object-format=brackets",
    "--validator-form-object-format=brackets",
)
PRISM = ("npm", "exec", "--yes", "--package=@stoplight/prism-cli@5.14.2", "--", "prism", "mock")

_REQ_RE = re.compile(r"^([A-Za-z0-9][A-Za-z0-9._-]*)(\[[^\]]*\])?==([^\s;]+)")


class HarnessError(ValueError):
    """The manifest or an oracle file does not fit the harness."""


@dataclass(frozen=True)
class Pair:
    """A config oracle and the SDK generated from it."""

    config: Oracle
    sdk: Oracle


def resolve_pair(oracles: Sequence[Oracle], wanted: str) -> Pair:
    """Find the config/sdk pair named by a group, a config name or an sdk name."""
    by_name = {o.name: o for o in oracles}
    for config in oracles:
        if config.kind != "config" or config.pairs_with is None:
            continue
        if wanted in {config.name, config.group, config.pairs_with}:
            return Pair(config, by_name[config.pairs_with])
    msg = f"FER4020 {wanted}: no config oracle with a paired sdk in oracles/manifest.yaml"
    raise HarnessError(msg)


def normalize_name(name: str) -> str:
    """PEP 503 normalized distribution name."""
    return re.sub(r"[-_.]+", "-", name).lower()


def lock_requirements(lock_text: str, exclude: Sequence[str] = ()) -> list[str]:
    """Pinned ``name==version`` lines from a rye/uv lock, by name only.

    Editable, path, URL and option lines are dropped, as is the oracle package itself.
    """
    skip = {normalize_name(n) for n in exclude}
    out: list[str] = []
    for raw in lock_text.splitlines():
        m = _REQ_RE.match(raw.strip())
        if m and normalize_name(m.group(1)) not in skip:
            out.append(f"{m.group(1)}{m.group(2) or ''}=={m.group(3)}")
    return sorted(set(out), key=str.lower)


def mock_command(mock_script: str, spec: Path, port: int = MOCK_PORT) -> tuple[str, ...]:
    """Return the pinned mock server command matching what the oracle's scripts/mock uses."""
    if "@stdy/cli" in mock_script:
        return (*STEADY, "--host", MOCK_HOST, "-p", str(port), *STEADY_FLAGS, str(spec))
    return (*PRISM, "-h", MOCK_HOST, "-p", str(port), str(spec))


@dataclass(frozen=True)
class Outcomes:
    """pytest outcome counts from a junit report."""

    passed: int = 0
    failed: int = 0
    errors: int = 0
    skipped: int = 0

    @property
    def total(self) -> int:
        """All collected test cases."""
        return self.passed + self.failed + self.errors + self.skipped

    def as_dict(self) -> dict[str, int]:
        """JSON-ready counts including the total."""
        return {**asdict(self), "total": self.total}


def parse_junit(xml_text: str) -> Outcomes:
    """Count outcomes per test case in a pytest junit XML report."""
    root = ET.fromstring(xml_text)  # noqa: S314 - our own pytest output
    passed = failed = errors = skipped = 0
    for case in root.iter("testcase"):
        tags = {child.tag for child in case}
        if "error" in tags:
            errors += 1
        elif "failure" in tags:
            failed += 1
        elif "skipped" in tags:
            skipped += 1
        else:
            passed += 1
    return Outcomes(passed, failed, errors, skipped)


def breakage_kinds(kinds: Sequence[str]) -> dict[str, int]:
    """Histogram of change kinds, sorted by kind."""
    counts: dict[str, int] = {}
    for kind in kinds:
        counts[kind] = counts.get(kind, 0) + 1
    return dict(sorted(counts.items()))
