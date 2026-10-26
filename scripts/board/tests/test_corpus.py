# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import json
from pathlib import Path

import pytest

from corpus.fetch import Compare, Spec
from scripts.board.corpus import (
    NO_COMPARE,
    CorpusBoardError,
    build,
    build_row,
    first_line,
    gap_titles,
    main,
)

SHA = "a" * 40
GAPS_MD = (
    "| # | Need | Interface today | Workaround | Upstream |\n"
    "|---|------|-----------------|------------|----------|\n"
    "| G24 | Read Swagger 2.0 specs | x | y | z |\n"
    "| G29 | Bundle `$ref`s into other files | x | y | z |\n"
)
TITLES = gap_titles(GAPS_MD)


def spec(name: str = "slack", compare: Compare | None = None) -> Spec:
    return Spec(name, f"https://x/{name}.json", "json", "b" * 64, 10, SHA, compare)


def report(name: str = "slack", **stages: dict[str, object]) -> dict[str, object]:
    return {
        "spec": name,
        "stages": stages,
        "counts": {
            "operations": 3,
            "generated_methods": 2,
            "skipped": 1,
            "skipped_by_code": {"FER201": 1},
        },
        "models_added": ["file"],
        "fer_codes": {"FER1610": 1},
        "seconds": 9.0,
    }


OK: dict[str, object] = {"ok": True, "seconds": 1.0}


def test_gap_titles() -> None:
    assert TITLES == {"G24": "Read Swagger 2.0 specs", "G29": "Bundle `$ref`s into other files"}


def test_first_line_drops_local_paths() -> None:
    error = (
        "pyright exited 1 with 3 errors:\n"
        "  /home/u/ferrule/.cache/corpus/slack/sdk/src/a.py:1:1 - error: bad\n"
    )
    assert first_line(error) == "pyright exited 1 with 3 errors: src/a.py:1:1 - error: bad"
    assert first_line("") == ""


def test_row_for_a_spec_that_stopped() -> None:
    raw = report(
        fetch={**OK, "format": "swagger 2.0", "bytes": 5},
        convert={**OK, "swagger2": True, "codes": {"FER1610": 1}},
        generate={
            "ok": False,
            "error": "ferrule-gen exited 1 in forge:\n$ref x is to another document",
        },
    )
    row = build_row(spec(), raw, TITLES)
    assert (row["reached"], row["failed_at"], row["swagger2"]) == ("convert", "generate", True)
    stages = row["stages"]
    assert isinstance(stages, list)
    assert stages[2] == {
        "name": "generate",
        "status": "fail",
        "detail": "ferrule-gen exited 1 in forge: $ref x is to another document",
    }
    assert stages[3] == {"name": "render", "status": "not-run"}
    assert row["gaps"] == [
        {"id": "G24", "title": "Read Swagger 2.0 specs", "blocking": False},
        {"id": "G29", "title": "Bundle `$ref`s into other files", "blocking": True},
    ]
    assert row["compare"] == {"status": "none", "reason": NO_COMPARE["slack"]}
    assert row["tests"] is None
    assert row["deterministic"] is None


def test_row_with_checks_tests_and_compare() -> None:
    compare = Compare("acme/acme-python", SHA, "acme")
    raw = report(
        "acme",
        render={**OK, "digest": "d1", "files": 4},
        ruff={**OK, "errors": 0},
        pyright={
            "ok": False,
            "error": "pyright exited 1 with 7 errors:\nx",
            "errors": 7,
            "causes": {"reportRedeclaration": 7},
        },
        tests={
            "ok": False,
            "error": "pytest exit 1",
            "passed": 5,
            "failed": 1,
            "errors": 0,
            "skipped": 0,
            "total": 6,
        },
        determinism={"ok": True},
        compare={**OK, "breaking": 2, "additive": 1, "closeness": 12.5},
    )
    row = build_row(spec("acme", compare), raw, TITLES)
    assert row["check_errors"] == {"ruff": 0, "pyright": 7}
    assert row["tests"] == {"passed": 5, "failed": 1, "errors": 0, "skipped": 0, "total": 6}
    stages = row["stages"]
    assert isinstance(stages, list)
    assert {
        "name": "pyright",
        "status": "fail",
        "detail": "pyright exited 1 with 7 errors: x",
        "causes": {"reportRedeclaration": 7},
    } in stages
    assert row["deterministic"] is True
    assert row["generation_seconds"] == 1.0
    compared = row["compare"]
    assert isinstance(compared, dict)
    assert (compared["status"], compared["breaking"], compared["closeness"]) == ("pass", 2, 12.5)


def test_missing_report_and_wrong_spec() -> None:
    assert build_row(spec(), None, TITLES)["ran"] is False
    with pytest.raises(CorpusBoardError):
        build_row(spec(), report("other"), TITLES)


def test_build_is_deterministic_and_dated() -> None:
    specs = [spec("slack"), spec("docker")]
    reports: dict[str, object] = {"slack": report()}
    text = build(specs, reports, TITLES, "2026-10-26", "ferrule-gen 0.0.0")
    assert text == build(
        specs, dict(reversed(reports.items())), TITLES, "2026-10-26", "ferrule-gen 0.0.0"
    )
    doc = json.loads(text)
    assert [r["name"] for r in doc["specs"]] == ["slack", "docker"]
    assert doc["run_date"] == "2026-10-26"
    with pytest.raises(CorpusBoardError):
        build(specs, reports, TITLES, "yesterday", None)


def test_main(tmp_path: Path) -> None:
    reports = tmp_path / "reports"
    reports.mkdir()
    (reports / "slack.json").write_text(json.dumps(report(fetch=OK)))
    manifest = tmp_path / "manifest.yaml"
    entry = {
        "name": "slack",
        "url": "https://example.test/slack.json",
        "format": "json",
        "source": None,
        "sha256": "b" * 64,
        "size": 10,
    }
    manifest.write_text(json.dumps({"version": 1, "tier1": [entry], "tier2": []}))
    gaps = tmp_path / "gaps.md"
    gaps.write_text(GAPS_MD)
    out = tmp_path / "corpus.json"
    argv = ["--date", "2026-10-26", "--reports", str(reports), "--manifest", str(manifest)]
    assert main([*argv, "--gaps", str(gaps), "--out", str(out)]) == 0
    assert json.loads(out.read_text())["specs"][0]["reached"] == "fetch"
    assert main([*argv[:1], "bad", *argv[2:], "--gaps", str(gaps), "--out", str(out)]) == 1
