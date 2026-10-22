"""Offline tests for the corpus manifest and helpers."""

from __future__ import annotations

import hashlib
import json
from typing import TYPE_CHECKING

import pytest

from corpus import fetch

if TYPE_CHECKING:
    from pathlib import Path

SHA = "a" * 40
TIER1 = [
    "stripe",
    "github",
    "openai",
    "kubernetes",
    "twilio",
    "cloudflare",
    "azure-storage",
    "slack",
    "plaid",
    "square",
    "digitalocean",
    "discord",
    "docker",
    "jira",
]


def spec(**overrides: object) -> dict[str, object]:
    base: dict[str, object] = {
        "name": "demo",
        "url": f"https://raw.githubusercontent.com/acme/api/{SHA}/openapi.json",
        "format": "json",
        "source": {"repo": "acme/api", "sha": SHA, "path": "openapi.json"},
        "sha256": "b" * 64,
        "size": 10,
    }
    base.update(overrides)
    return base


def doc(tier1: list[dict[str, object]], tier2: list[dict[str, object]] | None = None) -> str:
    return "# c\n" + json.dumps({"version": 1, "tier1": tier1, "tier2": tier2 or []})


def test_real_manifest_is_valid() -> None:
    manifest = fetch.parse_manifest(fetch.MANIFEST.read_text(encoding="utf-8"))
    assert [s.name for s in manifest.tier1] == TIER1
    assert [g.name for g in manifest.tier2] == ["apis-guru"]


def test_parses_good_spec() -> None:
    manifest = fetch.parse_manifest(doc([spec()]))
    assert manifest.tier1[0].source_sha == SHA


def test_non_github_spec_needs_no_source() -> None:
    entry = spec(url="https://example.invalid/openapi.json", source=None)
    assert fetch.parse_manifest(doc([entry])).tier1[0].source_sha is None


@pytest.mark.parametrize(
    "overrides",
    [
        {"sha256": "short"},
        {"url": "http://example.invalid/x.json", "source": None},
        {"url": "https://raw.githubusercontent.com/acme/api/main/openapi.json"},
        {"source": None},
        {"source": {"repo": "acme/api", "sha": "main", "path": "openapi.json"}},
        {"format": "xml"},
        {"size": 0},
        {"name": "Nope"},
    ],
)
def test_rejects_bad_specs(overrides: dict[str, object]) -> None:
    with pytest.raises(fetch.ManifestError):
        fetch.parse_manifest(doc([spec(**overrides)]))


def test_rejects_bad_tier2_and_duplicates() -> None:
    git = {"name": "dir", "kind": "git", "repo": "a/b", "url": "https://github.com/a/b.git"}
    with pytest.raises(fetch.ManifestError):
        fetch.parse_manifest(doc([], [{**git, "sha": "abc"}]))
    with pytest.raises(fetch.ManifestError):
        fetch.parse_manifest(doc([], [{**git, "sha": SHA, "kind": "tarball"}]))
    with pytest.raises(fetch.ManifestError):
        fetch.parse_manifest(doc([spec(), spec()]))


def test_cached_file_short_circuits(tmp_path: Path) -> None:
    body = b'{"openapi": "3.1.0"}'
    entry = spec(sha256=hashlib.sha256(body).hexdigest(), url="https://example.invalid/x.json")
    entry["source"] = None
    parsed = fetch.parse_manifest(doc([entry])).tier1[0]
    path = fetch.cache_path(parsed, tmp_path)
    path.parent.mkdir(parents=True)
    path.write_bytes(body)
    # A verified cached copy is reused without touching the (unreachable) URL.
    assert fetch.fetch_spec(parsed, tmp_path) == path
