# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Paths the generator never touches: ``lib/``, ``examples/`` and stainless.yml ``keep_files``."""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from functools import cached_property
from typing import TYPE_CHECKING, cast

if TYPE_CHECKING:
    from collections.abc import Mapping

PROTECTED_DIRS = frozenset({"lib"})
"""Directory names that are custom code wherever they appear (``src/<pkg>/lib/``)."""

PROTECTED_ROOTS = frozenset({"examples"})
"""Top-level directories the generator leaves alone."""


def glob_to_regex(pattern: str) -> re.Pattern[str]:
    """Compile a ``keep_files`` glob: ``**`` spans directories, ``*`` and ``?`` do not."""
    out: list[str] = []
    i = 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out.append("(?:.*/)?")
            i += 3
        elif pattern.startswith("**", i):
            out.append(".*")
            i += 2
        elif pattern[i] == "*":
            out.append("[^/]*")
            i += 1
        elif pattern[i] == "?":
            out.append("[^/]")
            i += 1
        else:
            out.append(re.escape(pattern[i]))
            i += 1
    body = "".join(out).rstrip("/")
    # A pattern naming a directory keeps everything below it.
    return re.compile(f"{body}(?:/.*)?")


@dataclass(frozen=True)
class KeepRules:
    """Which paths always keep the custom (``ours``) side of a merge."""

    keep_files: tuple[str, ...] = ()
    protect_lib: bool = field(default=True)

    @classmethod
    def from_config(cls, config: Mapping[str, object], target: str = "python") -> KeepRules:
        """Read ``targets.<target>.keep_files`` from a parsed stainless.yml."""
        node: object = config
        for name in ("targets", target, "keep_files"):
            node = cast("Mapping[str, object]", node).get(name) if isinstance(node, dict) else None
        items = cast("list[object]", node) if isinstance(node, list) else []
        return cls(tuple(str(item) for item in items))

    @cached_property
    def _patterns(self) -> tuple[re.Pattern[str], ...]:
        return tuple(glob_to_regex(p.removeprefix("./")) for p in self.keep_files)

    def keeps(self, path: str) -> bool:
        """Whether ``path`` (POSIX, repo-relative) is custom code the merge must not touch."""
        parts = path.split("/")
        if self.protect_lib and (
            parts[0] in PROTECTED_ROOTS or any(p in PROTECTED_DIRS for p in parts[:-1])
        ):
            return True
        return any(p.fullmatch(path) for p in self._patterns)
