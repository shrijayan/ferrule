# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Rendered runtime files are byte-identical to the pinned oracle SDKs' files.

Skipped when the oracle cache is absent (`python3 oracles/fetch.py <name>`).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import pytest

from ferrule_runtime import render_files

CACHE = Path(__file__).resolve().parents[2] / "oracles" / "cache"


@dataclass(frozen=True)
class Oracle:
    name: str
    package_dir: str
    package: str
    client: str
    options: dict[str, Any] = field(default_factory=dict[str, Any])
    known_drift: frozenset[str] = frozenset()
    """Files that use a Stainless feature the template does not cover (runtime/README.md)."""


ORACLES = [
    Oracle(
        "onebusaway-python",
        "src/onebusaway",
        "onebusaway",
        "OnebusawaySDK",
        {},
    ),
    Oracle(
        "groq-python",
        "src/groq",
        "groq",
        "Groq",
        {"repo_url": "https://github.com/groq/groq-python", "min_python": (3, 10)},
        frozenset({"_streaming.py"}),
    ),
    Oracle(
        "lithic-python",
        "src/lithic",
        "lithic",
        "Lithic",
        {"webhooks": True},
        frozenset({"_base_client.py", "_types.py", "_response.py"}),
    ),
    Oracle("isaacus-python", "src/isaacus", "isaacus", "Isaacus"),
    Oracle(
        "perplexity-py",
        "src/perplexity",
        "perplexity",
        "Perplexity",
        {"timeout": 900},
        frozenset({"_streaming.py", "_compat.py"}),
    ),
    Oracle("knock-mgmt-python", "src/knock_mapi", "knock_mapi", "KnockMgmt"),
    Oracle(
        "imagekit-python",
        "src/imagekitio",
        "imagekitio",
        "ImageKit",
        {
            "webhooks": True,
            "repo_url": "https://github.com/imagekit-developer/imagekit-python",
            "default_branch": "master",
        },
    ),
]


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_matches_oracle_bytes(oracle: Oracle) -> None:
    root = CACHE / oracle.name / oracle.package_dir
    if not root.is_dir():
        pytest.skip(f"oracle cache absent: python3 oracles/fetch.py {oracle.name}")
    files = render_files(oracle.package, oracle.client, **oracle.options)
    mismatched = sorted(
        rel
        for rel, text in files.items()
        if rel not in oracle.known_drift
        and (not (root / rel).is_file() or (root / rel).read_bytes() != text.encode())
    )
    assert mismatched == []


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_known_drift_is_still_drift(oracle: Oracle) -> None:
    root = CACHE / oracle.name / oracle.package_dir
    if not root.is_dir():
        pytest.skip(f"oracle cache absent: python3 oracles/fetch.py {oracle.name}")
    files = render_files(oracle.package, oracle.client, **oracle.options)
    stale = sorted(
        rel for rel in oracle.known_drift if (root / rel).read_bytes() == files[rel].encode()
    )
    assert stale == []


def test_render_covers_every_onebusaway_runtime_file() -> None:
    root = CACHE / "onebusaway-python" / "src" / "onebusaway"
    if not root.is_dir():
        pytest.skip("oracle cache absent: python3 oracles/fetch.py onebusaway")
    generated = {"__init__.py", "_client.py", "_version.py"}
    on_disk = {
        p.relative_to(root).as_posix()
        for p in root.rglob("*")
        if p.is_file()
        and "__pycache__" not in p.parts
        and p.relative_to(root).parts[0] not in {"resources", "types", "lib"}
    }
    assert on_disk - generated == set(render_files("onebusaway", "OnebusawaySDK"))
