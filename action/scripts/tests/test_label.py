# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
from __future__ import annotations

import json
from pathlib import Path

import pytest

from action.scripts import label

FIXTURES = Path(__file__).parent / "fixtures"


@pytest.mark.parametrize(
    ("breaking", "additive", "expected"),
    [(0, 0, "patch"), (0, 3, "minor"), (1, 0, "major"), (2, 5, "major")],
)
def test_bump(breaking: int, additive: int, expected: str) -> None:
    assert label.bump(breaking, additive) == expected


def test_parse_counts() -> None:
    text = (FIXTURES / "breaking.json").read_text()
    assert label.parse_report(text) == (1, 1)


def test_parse_falls_back_to_lists() -> None:
    assert label.parse_report(json.dumps({"breaking": [], "additive": [{}, {}]})) == (0, 2)


@pytest.mark.parametrize(
    "text",
    ["not json", "[]", '{"breaking_count": -1}', '{"breaking_count": true}', '{"additive": 3}'],
)
def test_parse_rejects_bad_reports(text: str) -> None:
    with pytest.raises(label.ReportError):
        label.parse_report(text)


def test_enforced_breaking_fails() -> None:
    verdict = label.decide(1, 0, [""], enforce=True)
    assert verdict.failed
    assert verdict.label == "major"


def test_override_label_allows_breaking() -> None:
    verdict = label.decide(1, 0, ["bug", " ferrule:allow-breaking"], enforce=True)
    assert verdict.allowed
    assert not verdict.failed


def test_unenforced_breaking_passes() -> None:
    assert not label.decide(1, 0, [], enforce=False).failed


@pytest.mark.parametrize(
    ("fixture", "code", "expected"),
    [("breaking.json", 1, "major"), ("additive.json", 0, "minor"), ("clean.json", 0, "patch")],
)
def test_main_writes_outputs(tmp_path: Path, fixture: str, code: int, expected: str) -> None:
    out = tmp_path / "out"
    assert label.main([str(FIXTURES / fixture), "--github-output", str(out)]) == code
    assert f"label={expected}" in out.read_text().splitlines()


def test_main_override() -> None:
    report = str(FIXTURES / "breaking.json")
    assert label.main([report, "--labels", "ferrule:allow-breaking"]) == 0
    assert label.main([report, "--enforce", "false"]) == 0


def test_main_missing_report(tmp_path: Path) -> None:
    assert label.main([str(tmp_path / "nope.json")]) == 2


def test_main_rejects_bad_bool() -> None:
    with pytest.raises(SystemExit):
        label.main([str(FIXTURES / "clean.json"), "--enforce", "yes"])
