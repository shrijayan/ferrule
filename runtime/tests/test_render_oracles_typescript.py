# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""The rendered TypeScript runtime is byte-identical to the pinned oracle SDKs' files.

Oracles are checkouts at ``oracles/cache/<name>`` (pins in ``runtime/README.md``); a test skips
when its checkout is absent. Only the Apache-2.0 repositories feed the template; opencode-sdk-js
(MIT) is compared, never copied.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

import pytest

from ferrule_runtime import TypeScriptOptions, render_typescript_files

CACHE = Path(__file__).resolve().parents[2] / "oracles" / "cache"

# Older Stainless TypeScript generations (2025): same files, older text.
_KNOCK_2025 = frozenset(
    {
        "src/core/resource.ts",
        "src/internal/headers.ts",
        "src/internal/parse.ts",
        "src/internal/qs/formats.ts",
        "src/internal/qs/stringify.ts",
        "src/internal/qs/utils.ts",
        "src/internal/request-options.ts",
        "src/internal/to-file.ts",
        "src/internal/types.ts",
        "src/internal/uploads.ts",
        "src/internal/utils.ts",
        "src/internal/utils/env.ts",
        "src/internal/utils/log.ts",
        "src/internal/utils/path.ts",
        "src/internal/utils/query.ts",  # absent: query stringifying still lived in client.ts
        "src/internal/utils/values.ts",
    }
)
_PAPR_2025 = frozenset(
    {
        "src/index.ts",
        "src/internal/parse.ts",
        "src/internal/types.ts",
        "src/internal/utils.ts",
        "src/internal/utils/env.ts",
        "src/internal/utils/query.ts",  # absent, as for knock-node
    }
)
_OPENCODE_2025 = frozenset(
    {
        "src/internal/parse.ts",
        "src/internal/request-options.ts",
        "src/internal/to-file.ts",
        "src/internal/types.ts",
        "src/internal/utils.ts",
        "src/internal/utils/env.ts",
        "src/internal/utils/query.ts",  # absent, as for knock-node
        "src/internal/utils/values.ts",
    }
)


@dataclass(frozen=True)
class Oracle:
    name: str
    options: TypeScriptOptions
    known_drift: frozenset[str] = frozenset()
    """Files whose text comes from another Stainless generation (runtime/README.md)."""
    generated: frozenset[str] = field(default_factory=frozenset[str])
    """Runtime-adjacent files the plugin generates (pagination classes from the config)."""


ORACLES = [
    Oracle("isaacus-typescript", TypeScriptOptions(client_name="Isaacus")),
    Oracle("imagekit-typescript", TypeScriptOptions(client_name="ImageKit")),
    Oracle(
        "knock-mgmt-typescript",
        TypeScriptOptions(client_name="KnockMgmt", query_array_format="comma", pagination=True),
        generated=frozenset({"src/core/pagination.ts", "src/pagination.ts"}),
    ),
    Oracle(
        "knock-typescript",
        TypeScriptOptions(
            client_name="Knock",
            query_array_format="brackets",
            pagination=True,
            redacted_headers=(),
        ),
        _KNOCK_2025,
        frozenset({"src/core/pagination.ts", "src/pagination.ts"}),
    ),
    Oracle(
        "papr-typescript",
        TypeScriptOptions(client_name="Papr", redacted_headers=("x-session-token", "x-api-key")),
        _PAPR_2025,
    ),
    Oracle(
        "opencode-sdk-js",
        TypeScriptOptions(client_name="Opencode", redacted_headers=()),
        _OPENCODE_2025,
        # SSE streaming: no Apache-2.0 current-layout oracle ships it, so it is not templated.
        frozenset({"src/core/streaming.ts", "src/internal/decoders/line.ts", "src/streaming.ts"}),
    ),
]
# Generated per SDK: the client, its version and resources; lib/ is hand-written code.
GENERATED = frozenset({"src/client.ts", "src/version.ts"})


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
    files = render_typescript_files(oracle.options)
    mismatched = sorted(
        rel
        for rel, text in files.items()
        if rel not in oracle.known_drift and not _matches(root, rel, text)
    )
    assert mismatched == []


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_known_drift_is_still_drift(oracle: Oracle) -> None:
    root = _root(oracle)
    files = render_typescript_files(oracle.options)
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
    rendered = set(render_typescript_files(oracle.options))
    expected = on_disk - GENERATED - oracle.generated
    assert expected - rendered == set()
    assert rendered - expected <= oracle.known_drift
