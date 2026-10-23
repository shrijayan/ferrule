# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Pure three-way merge of whole SDK trees: base (previous generation), ours, theirs.

``base`` is the previous ``generated`` branch tree, ``ours`` the integrated branch (generated
code plus custom edits) and ``theirs`` the new generator output. Trees are ``{path: bytes}``
with POSIX, repo-relative paths.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from typing import TYPE_CHECKING

from ferrule.merge.ast_merge import merge_python
from ferrule.merge.text import Region, merge3

if TYPE_CHECKING:
    from collections.abc import Mapping

    from ferrule.merge.rules import KeepRules

type Tree = Mapping[str, bytes]

TEXT_CONFLICT = "FER4301"
DELETED_BY_USER = "FER4302"
DELETED_BY_GENERATOR = "FER4303"
BINARY_CONFLICT = "FER4304"


class Outcome(StrEnum):
    """How one path was resolved."""

    UNCHANGED = "unchanged"
    THEIRS = "theirs"
    OURS = "ours"
    KEPT = "kept"
    MERGED = "merged"
    AST = "ast"
    DELETED = "deleted"
    CONFLICT = "conflict"


@dataclass(frozen=True)
class Conflict:
    """A path the merge could not settle on its own; ``blocking`` ones need a conflict PR."""

    code: str
    path: str
    message: str
    fix: str
    blocking: bool = True
    regions: tuple[Region, ...] = ()

    def render(self) -> str:
        """Format like every Ferrule diagnostic, with the conflicted line ranges."""
        where = self.path
        if self.regions:
            where += ":" + ",".join(f"{r.start}-{r.end}" for r in self.regions)
        return f"{self.code} at {where}: {self.message}. Fix: {self.fix}"

    def to_json(self) -> dict[str, object]:
        """Serialise for the ``--report`` file."""
        return {
            "code": self.code,
            "path": self.path,
            "message": self.message,
            "fix": self.fix,
            "blocking": self.blocking,
            "regions": [{"start": r.start, "end": r.end} for r in self.regions],
        }


@dataclass(frozen=True)
class MergeResult:
    """The merged tree, how each path was resolved, and what needs a human."""

    files: dict[str, bytes]
    outcomes: dict[str, Outcome]
    conflicts: tuple[Conflict, ...]

    @property
    def blocking(self) -> tuple[Conflict, ...]:
        """Conflicts that must be resolved before the result can land."""
        return tuple(c for c in self.conflicts if c.blocking)

    def to_json(self) -> dict[str, object]:
        """Serialise for the ``--report`` file (the tree itself is not included)."""
        counts: dict[str, int] = {}
        for outcome in self.outcomes.values():
            counts[outcome] = counts.get(outcome, 0) + 1
        return {
            "clean": not self.blocking,
            "counts": dict(sorted(counts.items())),
            "outcomes": {p: o for p, o in sorted(self.outcomes.items()) if o != Outcome.UNCHANGED},
            "conflicts": [c.to_json() for c in self.conflicts],
        }


def _is_text(*blobs: bytes) -> bool:
    try:
        for blob in blobs:
            blob.decode("utf-8")
    except UnicodeDecodeError:
        return False
    return not any(b"\0" in blob for blob in blobs)


@dataclass(frozen=True)
class _Resolved:
    content: bytes | None
    outcome: Outcome
    conflict: Conflict | None = None


def _content_merge(path: str, base: bytes, ours: bytes, theirs: bytes) -> _Resolved:
    if not _is_text(base, ours, theirs):
        return _Resolved(
            ours,
            Outcome.CONFLICT,
            Conflict(
                BINARY_CONFLICT,
                path,
                "binary file changed by both the generator and custom code; kept custom",
                "compare with the new generated file and commit the right version",
            ),
        )
    text = merge3(base, ours, theirs)
    if text.clean:
        return _Resolved(text.content, Outcome.MERGED)
    if path.endswith((".py", ".pyi")):
        merged = merge_python(base.decode(), ours.decode(), theirs.decode())
        if merged is not None:
            return _Resolved(merged.encode(), Outcome.AST)
    return _Resolved(
        text.content,
        Outcome.CONFLICT,
        Conflict(
            TEXT_CONFLICT,
            path,
            f"{len(text.conflicts)} region(s) changed by both the generator and custom code",
            "resolve the conflict markers, or move the custom code into lib/",
            regions=text.conflicts,
        ),
    )


def _resolve(  # noqa: PLR0911 - one return per row of the three-way table
    path: str, base: bytes | None, ours: bytes | None, theirs: bytes | None, keep: KeepRules
) -> _Resolved:
    if keep.keeps(path):
        return _Resolved(ours, Outcome.UNCHANGED if ours == theirs else Outcome.KEPT)
    if ours == theirs:
        return _Resolved(ours, Outcome.UNCHANGED)
    if ours == base:
        return _Resolved(theirs, Outcome.THEIRS if theirs is not None else Outcome.DELETED)
    if theirs == base:
        return _Resolved(ours, Outcome.OURS)
    if ours is None:
        return _Resolved(
            None,
            Outcome.CONFLICT,
            Conflict(
                DELETED_BY_USER,
                path,
                "custom code deleted a file the generator changed; left deleted",
                "restore it from the generated branch, or add it to keep_files",
            ),
        )
    if theirs is None:
        return _Resolved(
            ours,
            Outcome.KEPT,
            Conflict(
                DELETED_BY_GENERATOR,
                path,
                "the generator no longer writes this edited file; kept custom version",
                "delete it if the custom code is obsolete, or add it to keep_files",
                blocking=False,
            ),
        )
    return _content_merge(path, base or b"", ours, theirs)


def merge_trees(base: Tree, ours: Tree, theirs: Tree, *, keep: KeepRules) -> MergeResult:
    """Merge the new generation ``theirs`` into ``ours``, using ``base`` as common ancestor."""
    files: dict[str, bytes] = {}
    outcomes: dict[str, Outcome] = {}
    conflicts: list[Conflict] = []
    for path in sorted(base.keys() | ours.keys() | theirs.keys()):
        resolved = _resolve(path, base.get(path), ours.get(path), theirs.get(path), keep)
        if resolved.content is not None:
            files[path] = resolved.content
        outcomes[path] = resolved.outcome
        if resolved.conflict is not None:
            conflicts.append(resolved.conflict)
    return MergeResult(files, outcomes, tuple(conflicts))
