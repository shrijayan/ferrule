# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Offline tests for the oracle manifest and helpers."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, cast

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


CONFIG_MD5 = "c" * 32
SPEC_MD5 = "b" * 32
ABC_MD5 = "900150983cd24fb0d6963f7d28e17f72"  # md5(b"abc")
SPEC_URL = "https://example.invalid/specs/demo.yml"


def config_entry(**overrides: object) -> dict[str, object]:
    base: dict[str, object] = {
        "name": "demo-config",
        "group": "demo",
        "kind": "config",
        "repo": "acme/demo-config",
        "url": "https://github.com/acme/demo-config.git",
        "sha": "e" * 40,
        "commit_date": "2026-01-01T00:00:00Z",
        "pairs_with": "demo-python",
        "paired": "proven",
        "config_path": "stainless.yml",
        "spec": {"path": "openapi.yml"},
        "stats": None,
        "verify_md5": {"stainless.yml": CONFIG_MD5, "openapi.yml": SPEC_MD5},
    }
    base.update(overrides)
    return base


def sdk_entry(**overrides: object) -> dict[str, object]:
    return entry(pairs_with="demo-config", license="Apache-2.0", **overrides)


def test_real_manifest_config_entries() -> None:
    oracles = fetch.parse_manifest(fetch.MANIFEST.read_text(encoding="utf-8"))
    configs = fetch.select(oracles, [], "config")
    assert configs
    assert "onebusaway-config" in {o.name for o in configs}
    by_name = {o.name: o for o in oracles}
    for config in configs:
        assert config.paired in fetch.PAIRED
        assert config.config_path is not None
        assert config.spec is not None
        sdk = by_name[cast("str", config.pairs_with)]
        assert sdk.kind == "sdk"
        assert sdk.license in fetch.LICENSES
    assert all(o.kind == "sdk" for o in fetch.select(oracles, [], "sdk"))


def test_parses_proven_config_pair() -> None:
    config, sdk = fetch.parse_manifest(doc(config_entry(), sdk_entry()))
    assert config.config_path == "stainless.yml"
    assert config.spec == fetch.Spec(path="openapi.yml", url=None, md5=SPEC_MD5)
    assert config.paired == "proven"
    assert sdk.license == "Apache-2.0"
    assert fetch.select([config, sdk], ["demo"], "config") == [config]


def test_parses_spec_url_and_nearest() -> None:
    raw = config_entry(
        paired="nearest",
        spec={"url": SPEC_URL, "md5": "d" * 32},
        verify_md5={"stainless.yml": "f" * 32},
    )
    config, _ = fetch.parse_manifest(doc(raw, sdk_entry()))
    assert config.spec == fetch.Spec(path=None, url=SPEC_URL, md5="d" * 32)
    assert config.paired == "nearest"


def test_old_style_sdk_entry_still_parses() -> None:
    (oracle,) = fetch.parse_manifest(doc(entry()))
    assert oracle.license is None
    assert oracle.config_path is None
    assert oracle.spec is None
    assert oracle.paired is None


@pytest.mark.parametrize(
    "overrides",
    [
        {"config_path": None},
        {"config_path": "../stainless.yml"},
        {"config_path": "missing.yml"},
        {"paired": "maybe"},
        {"paired": None},
        {"pairs_with": None},
        {"spec": None},
        {"spec": {"path": "other.yml"}},
        {"spec": {"path": "openapi.yml", "url": SPEC_URL}},
        {"spec": {"url": "http://example.invalid/demo.yml", "md5": SPEC_MD5}},
        {"spec": {"url": "https://example.invalid/demo.exe", "md5": SPEC_MD5}},
        {"spec": {"url": SPEC_URL, "md5": "nope"}},
        {"verify_md5": {"stainless.yml": "0" * 32, "openapi.yml": SPEC_MD5}},
        {"verify_md5": {"stainless.yml": CONFIG_MD5, "openapi.yml": "0" * 32}},
        {"paired": "nearest"},
    ],
)
def test_rejects_bad_config_entries(overrides: dict[str, object]) -> None:
    raw = config_entry()
    for key, value in overrides.items():
        if value is None and key in {"config_path", "paired", "spec"}:
            del raw[key]
        else:
            raw[key] = value
    with pytest.raises(fetch.ManifestError):
        fetch.parse_manifest(doc(raw, sdk_entry()))


@pytest.mark.parametrize(
    "sdk",
    [
        entry(pairs_with="demo-config"),  # no license
        entry(pairs_with="demo-config", license="GPL-3.0"),
        entry(pairs_with="demo-config", license="MIT", spec={"path": "openapi.yml"}),
    ],
)
def test_rejects_bad_paired_sdk(sdk: dict[str, object]) -> None:
    with pytest.raises(fetch.ManifestError):
        fetch.parse_manifest(doc(config_entry(), sdk))


def test_config_must_pair_with_an_sdk() -> None:
    other = config_entry(name="other-config", pairs_with="demo-config")
    with pytest.raises(fetch.ManifestError):
        fetch.parse_manifest(doc(config_entry(), sdk_entry(), other))


def test_fetch_spec_from_checkout(tmp_path: Path) -> None:
    raw = config_entry(verify_md5={"stainless.yml": CONFIG_MD5, "openapi.yml": ABC_MD5})
    config, _ = fetch.parse_manifest(
        doc(raw, sdk_entry(stats={**STATS, "openapi_spec_hash": ABC_MD5}))
    )
    (tmp_path / "openapi.yml").write_bytes(b"abc")
    assert fetch.fetch_spec(config, tmp_path, tmp_path / "cache") == tmp_path / "openapi.yml"
    (tmp_path / "openapi.yml").write_bytes(b"abd")
    with pytest.raises(fetch.VerifyError):
        fetch.fetch_spec(config, tmp_path, tmp_path / "cache")


def test_fetch_spec_from_url(tmp_path: Path) -> None:
    raw = config_entry(
        paired="nearest",
        spec={"url": SPEC_URL, "md5": ABC_MD5},
        verify_md5={"stainless.yml": "f" * 32},
    )
    config, _ = fetch.parse_manifest(doc(raw, sdk_entry()))
    calls: list[str] = []

    def fake(url: str) -> bytes:
        calls.append(url)
        return b"abc"

    cache = tmp_path / "cache"
    target = fetch.fetch_spec(config, tmp_path, cache, fake)
    assert target == cache / "specs" / "demo-config.yml"
    assert target.read_bytes() == b"abc"
    assert fetch.fetch_spec(config, tmp_path, cache, fake) == target
    assert calls == [SPEC_URL]  # cached copy reused

    with pytest.raises(fetch.VerifyError):
        fetch.fetch_spec(config, tmp_path, tmp_path / "other", lambda _url: b"abd")
    assert not (tmp_path / "other" / "specs" / "demo-config.yml").exists()


def test_download_refuses_plain_http() -> None:
    with pytest.raises(fetch.VerifyError):
        fetch.download("http://example.invalid/spec.yml")


def test_parse_args() -> None:
    assert fetch.parse_args([]) == ([], None)
    assert fetch.parse_args(["--kind", "config", "demo"]) == (["demo"], "config")
    with pytest.raises(SystemExit):
        fetch.parse_args(["--kind", "binary"])


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
