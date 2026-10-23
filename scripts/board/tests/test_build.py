# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import json
from pathlib import Path

import pytest

from oracles.fetch import Oracle
from scripts.board.build import BoardError, build_board, build_row, config_groups, main


def oracle(name: str, group: str, kind: str, repo: str) -> Oracle:
    return Oracle(name, group, kind, repo, "https://x", "0" * 40, "", None, {}, None)


ORACLES = [
    oracle("acme-python", "acme", "sdk", "acme/acme-python"),
    oracle("acme-config", "acme", "config", "acme/sdk-config"),
    oracle("zeta-python", "zeta", "sdk", "zeta/zeta-python"),
]


def report(name: str = "acme", extra: dict[str, object] | None = None) -> dict[str, object]:
    base: dict[str, object] = {
        "oracle": name,
        "sdk": {"name": f"{name}-python", "sha": "a" * 40},
        "config": {"name": f"{name}-config", "sha": "b" * 40},
        "api": {"breaking": 1, "additive": 3, "breaking_kinds": {}, "additive_kinds": {}},
        "generate": {"ok": True, "deterministic": True, "digest": "d", "files": 1},
        "closeness": 93.666,
        "tests": {"status": "failed", "passed": 10, "failed": 2, "errors": 1},
    }
    base.update(extra or {})
    return base


def test_row_fields() -> None:
    row = build_row(report(), {"acme-python": "acme/acme-python"}, "2026-10-23")
    assert row.repo == "acme/acme-python"
    assert (row.breaking, row.additive, row.closeness) == (1, 3, 93.67)
    assert (row.tests_passed, row.tests_failed, row.tests_status) == (10, 3, "failed")
    assert row.deterministic
    assert row.generated == "2026-10-23"


def test_report_date_wins_and_missing_tests() -> None:
    row = build_row(report(extra={"generated": "2026-01-02", "tests": None}), {}, "2026-10-23")
    assert row.generated == "2026-01-02"
    assert row.repo is None
    assert build_row(report(extra={"closeness": None}), {}, None).closeness is None
    assert (row.tests_passed, row.tests_failed, row.tests_status) == (0, 0, "skipped")


@pytest.mark.parametrize(
    ("patch", "where"),
    [
        ({"closeness": "x"}, "acme.closeness"),
        ({"api": {"breaking": True, "additive": 0}}, "acme.api.breaking"),
        ({"sdk": []}, "acme.sdk"),
    ],
)
def test_bad_reports(patch: dict[str, object], where: str) -> None:
    with pytest.raises(BoardError, match=where):
        build_row(report(extra=patch), {}, None)


def test_board_is_sorted_and_deterministic() -> None:
    reports: dict[str, object] = {"z.json": report("zeta"), "a.json": report("acme")}
    first = build_board(reports, ORACLES, None)
    assert first == build_board(dict(reversed(reports.items())), ORACLES, None)
    names = [r["name"] for r in json.loads(first)]
    assert names == ["acme", "zeta"]
    assert first.endswith("\n")


def test_config_groups() -> None:
    assert config_groups(ORACLES) == ["acme"]


def test_main(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    reports = tmp_path / "reports"
    reports.mkdir()
    (reports / "acme.json").write_text(json.dumps(report("onebusaway")), encoding="utf-8")
    manifest = Path(__file__).resolve().parents[3] / "oracles" / "manifest.yaml"
    out = tmp_path / "out" / "fidelity.json"
    args = ["--reports", str(reports), "--manifest", str(manifest), "--out", str(out)]
    assert main([*args, "--generated", "2026-10-23"]) == 0
    rows = json.loads(out.read_text(encoding="utf-8"))
    assert rows[0]["repo"] == "OneBusAway/python-sdk"
    assert rows[0]["generated"] == "2026-10-23"
    assert main(["--manifest", str(manifest), "--list-oracles"]) == 0
    assert "onebusaway\n" in capsys.readouterr().out
    bad = json.dumps(report(extra={"sdk": []}))
    (reports / "acme.json").write_text(bad, encoding="utf-8")
    assert main(args) == 1
