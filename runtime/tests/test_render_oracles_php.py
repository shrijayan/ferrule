# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""The rendered PHP runtime is byte-identical to the pinned oracle package's files.

The oracle is a checkout at ``oracles/cache/<name>`` (pin in ``runtime/README.md``); the test
skips when it is absent. imagekit-php is Apache-2.0 (its ``LICENSE`` and ``composer.json``);
the template comes from it.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pytest

from ferrule_runtime import PhpOptions, render_php_files

CACHE = Path(__file__).resolve().parents[2] / "oracles" / "cache"


@dataclass(frozen=True)
class Oracle:
    name: str
    options: PhpOptions


ORACLES = [
    # imagekit's client_settings leave timeout and retries at their defaults.
    Oracle("imagekit-php", PhpOptions(namespace="Imagekit")),
]


def _root(oracle: Oracle) -> Path:
    root = CACHE / oracle.name
    if not (root / "composer.json").is_file():
        pytest.skip(f"oracle checkout absent: oracles/cache/{oracle.name}")
    return root


def _matches(root: Path, rel: str, text: str) -> bool:
    path = root / rel
    return path.is_file() and path.read_bytes() == text.encode()


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_matches_oracle_bytes(oracle: Oracle) -> None:
    root = _root(oracle)
    files = render_php_files(oracle.options)
    assert sorted(rel for rel, text in files.items() if not _matches(root, rel, text)) == []


def _on_disk(root: Path) -> set[str]:
    found = {"src/Core.php", "src/RequestOptions.php"}
    for base in ("src/Core", "tests/Core"):
        found |= {p.relative_to(root).as_posix() for p in (root / base).rglob("*.php")}
    return found


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_covers_every_runtime_file(oracle: Oracle) -> None:
    root = _root(oracle)
    assert set(render_php_files(oracle.options)) == _on_disk(root)
