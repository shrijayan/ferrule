"""Offline tests for the oracle manifest and helpers."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING

import pytest

from oracles import fetch

if TYPE_CHECKING:
    from pathlib import Path

GOOD_SHA = "a" * 40
STATS = {
    "configured_endpoints": 3,
    "openapi_spec_url": "https://example.invalid/spec.yml",
    "openapi_spec_hash": "b" * 32,
    "config_hash": "c" * 32,
}


def entry(**overrides: object) -> dict[str, object]:
    base: dict[str, object] = {
        "name": "demo-python",
        "group": "demo",
        "kind": "sdk",
        "repo": "acme/demo-python",
        "url": "https://github.com/acme/demo-python.git",
        "sha": GOOD_SHA,
        "commit_date": "2026-01-01T00:00:00Z",
        "pairs_with": None,
        "stats": dict(STATS),
        "verify_md5": {},
    }
    base.update(overrides)
    return base


def doc(*entries: dict[str, object]) -> str:
    return "# comment\n# more\n" + json.dumps({"version": 1, "oracles": list(entries)})


def test_real_manifest_is_valid() -> None:
    oracles = fetch.parse_manifest(fetch.MANIFEST.read_text(encoding="utf-8"))
    names = {o.name for o in oracles}
    assert {
        "onebusaway-config",
        "onebusaway-python",
        "groq-python",
        "lithic-python",
        "perplexity-py",
        "opencode-sdk-python",
    } <= names
    assert [o.name for o in fetch.select(oracles, ["onebusaway"])] == [
        "onebusaway-config",
        "onebusaway-python",
    ]


def test_parses_minimal_entry() -> None:
    (oracle,) = fetch.parse_manifest(doc(entry()))
    assert oracle.sha == GOOD_SHA
    assert oracle.stats is not None
    assert oracle.stats["configured_endpoints"] == "3"


@pytest.mark.parametrize(
    "overrides",
    [
        {"sha": "abc123"},
        {"name": "Bad_Name"},
        {"url": "https://evil.invalid/acme/demo-python.git"},
        {"kind": "binary"},
        {"stats": None},
        {"stats": {**STATS, "config_hash": "nope"}},
        {"stats": {**STATS, "configured_endpoints": 0}},
        {"verify_md5": {"../x": "d" * 32}},
        {"pairs_with": "missing"},
    ],
)
def test_rejects_bad_entries(overrides: dict[str, object]) -> None:
    with pytest.raises(fetch.ManifestError):
        fetch.parse_manifest(doc(entry(**overrides)))


def test_rejects_duplicates_and_bad_root() -> None:
    with pytest.raises(fetch.ManifestError):
        fetch.parse_manifest(doc(entry(), entry()))
    with pytest.raises(fetch.ManifestError):
        fetch.parse_manifest("not: [json")
    with pytest.raises(fetch.ManifestError):
        fetch.parse_manifest(json.dumps({"version": 2, "oracles": []}))


def test_select_unknown() -> None:
    oracles = fetch.parse_manifest(doc(entry()))
    assert fetch.select(oracles, []) == oracles
    with pytest.raises(fetch.ManifestError):
        fetch.select(oracles, ["nope"])


def test_parse_stats_yml() -> None:
    text = "configured_endpoints: 30\nopenapi_spec_url: https://x/y.yml\nconfig_hash: 'abc'\n"
    assert fetch.parse_stats_yml(text) == {
        "configured_endpoints": "30",
        "openapi_spec_url": "https://x/y.yml",
        "config_hash": "abc",
    }


def test_verify_files(tmp_path: Path) -> None:
    (oracle,) = fetch.parse_manifest(
        doc(entry(verify_md5={"a.txt": "900150983cd24fb0d6963f7d28e17f72"}))
    )
    (tmp_path / "a.txt").write_text("abc", encoding="utf-8")
    stats = "\n".join(f"{k}: {v}" for k, v in STATS.items())
    (tmp_path / ".stats.yml").write_text(stats, encoding="utf-8")
    fetch.verify_files(oracle, tmp_path)
    (tmp_path / "a.txt").write_text("abd", encoding="utf-8")
    with pytest.raises(fetch.VerifyError):
        fetch.verify_files(oracle, tmp_path)
