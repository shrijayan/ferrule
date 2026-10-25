# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""The rendered ``node`` TypeScript runtime is byte-identical to the pinned oracle SDKs' files.

Oracles are checkouts at ``oracles/cache/<name>`` (pins in ``runtime/README.md``); a test skips
when its checkout is absent. ogx-typescript (MIT, the ``node`` layout too) is not compared: it
adds its own licence header to every file.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

import pytest

from ferrule_runtime import TypeScriptNodeOptions, render_typescript_node_files

CACHE = Path(__file__).resolve().parents[2] / "oracles" / "cache"


@dataclass(frozen=True)
class Oracle:
    name: str
    options: TypeScriptNodeOptions
    known_drift: frozenset[str] = frozenset()
    """Files whose text comes from another Stainless generation (runtime/README.md)."""
    generated: frozenset[str] = field(default_factory=frozenset[str])
    """Runtime-adjacent files the plugin generates (pagination classes from the config)."""


ORACLES = [
    Oracle(
        "onebusaway-typescript",
        TypeScriptNodeOptions(
            client_name="OnebusawaySDK", package_name="onebusaway-sdk", repo="OneBusAway/js-sdk"
        ),
    ),
    Oracle(
        "lumaai-typescript",
        TypeScriptNodeOptions(
            client_name="LumaAI", package_name="lumaai", repo="lumalabs/lumaai-node"
        ),
        # 2026-04: `readEnv` returned `?.trim() ?? undefined` (an empty variable was a value).
        frozenset({"src/core.ts"}),
    ),
    Oracle(
        "honcho-typescript",
        TypeScriptNodeOptions(
            client_name="Honcho",
            package_name="@honcho-ai/core",
            repo="plastic-labs/honcho-node-core",
        ),
        # 2025: `stringifyQuery` still lived in `core.ts` (no `internal/utils/query.ts`), no
        # `content-length: 0` check, no path query merge, older retry-after handling and readEnv.
        frozenset({"src/core.ts", "src/internal/utils/query.ts"}),
        frozenset({"src/pagination.ts"}),
    ),
]
# Generated per SDK: the client (`src/index.ts`), its version and resources; lib/ is hand-written.
GENERATED = frozenset({"src/index.ts", "src/version.ts"})


def _root(oracle: Oracle) -> Path:
    root = CACHE / oracle.name
    if not (root / "src").is_dir():
        pytest.skip(f"oracle checkout absent: oracles/cache/{oracle.name}")
    return root


def _matches(root: Path, rel: str, text: str) -> bool:
    path = root / rel
    return path.is_file() and path.read_bytes() == text.encode()


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_matches_oracle_bytes(oracle: Oracle) -> None:
    root = _root(oracle)
    files = render_typescript_node_files(oracle.options)
    mismatched = sorted(
        rel
        for rel, text in files.items()
        if rel not in oracle.known_drift and not _matches(root, rel, text)
    )
    assert mismatched == []


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_known_drift_is_still_drift(oracle: Oracle) -> None:
    root = _root(oracle)
    files = render_typescript_node_files(oracle.options)
    stale = sorted(rel for rel in oracle.known_drift if _matches(root, rel, files[rel]))
    assert stale == []


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_covers_every_runtime_file(oracle: Oracle) -> None:
    root = _root(oracle)
    on_disk = {
        p.relative_to(root).as_posix()
        for p in (root / "src").rglob("*")
        if p.is_file() and p.relative_to(root / "src").parts[0] not in {"resources", "lib"}
    }
    rendered = set(render_typescript_node_files(oracle.options))
    expected = on_disk - GENERATED - oracle.generated
    assert expected - rendered == set()
    assert rendered - expected <= oracle.known_drift
