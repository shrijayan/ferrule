# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule.lock``: what produced an adopted SDK, so a regeneration can be reproduced.

JSON with sorted keys and a trailing newline: the same inputs give byte-identical locks.

Version 2 names the generator (``ferrule-gen <version>``) and the Forge rev it was built with,
both from ``ferrule-gen --version``; a hash of the native binary would differ per platform.
Version 1 locks (``plugin_sha256`` of plugin.wasm) still parse: that hash is ignored and the
generator reads as ``unknown``.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, cast

if TYPE_CHECKING:
    from collections.abc import Mapping
    from pathlib import Path

LOCK_FILE = "ferrule.lock"
LOCK_VERSION = 2
UNKNOWN_GENERATOR = "unknown"


def sha256_bytes(data: bytes) -> str:
    """Hex sha256 of ``data``."""
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    """Hex sha256 of the file at ``path``."""
    return sha256_bytes(path.read_bytes())


@dataclass(frozen=True)
class Lock:
    """Inputs and outputs of one generation."""

    ferrule_version: str
    forge_commit: str
    generator: str
    spec_sha256: str
    config_sha256: str
    files: Mapping[str, str] = field(default_factory=dict[str, str])

    def render(self) -> str:
        """Render the lock file text."""
        doc = {
            "lock_version": LOCK_VERSION,
            "ferrule_version": self.ferrule_version,
            "forge_commit": self.forge_commit,
            "generator": self.generator,
            "spec_sha256": self.spec_sha256,
            "config_sha256": self.config_sha256,
            "files": dict(sorted(self.files.items())),
        }
        return json.dumps(doc, indent=2, sort_keys=True) + "\n"


def parse(text: str) -> Lock:
    """Read a lock written by :meth:`Lock.render`, or a version 1 lock (see the module doc)."""
    doc: dict[str, object] = json.loads(text)
    files = doc.get("files", {})
    if not isinstance(files, dict):
        msg = "ferrule.lock: files must be an object"
        raise TypeError(msg)
    return Lock(
        ferrule_version=str(doc["ferrule_version"]),
        forge_commit=str(doc["forge_commit"]),
        generator=str(doc.get("generator", UNKNOWN_GENERATOR)),
        spec_sha256=str(doc["spec_sha256"]),
        config_sha256=str(doc["config_sha256"]),
        files={str(k): str(v) for k, v in cast("dict[object, object]", files).items()},
    )
