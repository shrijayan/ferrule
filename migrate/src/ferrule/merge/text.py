# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Pure line-based three-way merge (diff3 semantics) over bytes.

The base is aligned with each side using :class:`difflib.SequenceMatcher`; base ranges both
sides left unchanged become sync points, and each gap between sync points is resolved
independently: a side that did not change the gap yields to the other, identical changes
merge, anything else is a conflict wrapped in ``diff3``-style markers.
"""

from __future__ import annotations

from dataclasses import dataclass
from difflib import SequenceMatcher

OURS_MARKER = b"<<<<<<< ours (custom code)\n"
BASE_MARKER = b"||||||| base (previous generation)\n"
SPLIT_MARKER = b"=======\n"
THEIRS_MARKER = b">>>>>>> theirs (new generation)\n"

type Lines = tuple[bytes, ...]


@dataclass(frozen=True)
class Region:
    """A conflicted region, as 1-based inclusive line numbers in the merged output."""

    start: int
    end: int


@dataclass(frozen=True)
class TextMerge:
    """The merged bytes and every conflicted region (empty when the merge is clean)."""

    content: bytes
    conflicts: tuple[Region, ...]

    @property
    def clean(self) -> bool:
        """Whether the merge needed no conflict markers."""
        return not self.conflicts


@dataclass(frozen=True)
class _Sync:
    base: tuple[int, int]
    ours: tuple[int, int]
    theirs: tuple[int, int]


def split_lines(data: bytes) -> Lines:
    """Split ``data`` into lines that keep their terminators."""
    return tuple(data.splitlines(keepends=True))


def _matches(base: Lines, side: Lines) -> list[tuple[int, int, int]]:
    matcher = SequenceMatcher(None, base, side, autojunk=False)
    return [(m.a, m.b, m.size) for m in matcher.get_matching_blocks() if m.size]


def _sync_points(base: Lines, ours: Lines, theirs: Lines) -> list[_Sync]:
    """Return the base ranges both sides kept verbatim, plus an empty end sentinel."""
    ours_m, theirs_m = _matches(base, ours), _matches(base, theirs)
    out: list[_Sync] = []
    i = j = 0
    while i < len(ours_m) and j < len(theirs_m):
        ob, oo, on = ours_m[i]
        tb, tt, tn = theirs_m[j]
        lo, hi = max(ob, tb), min(ob + on, tb + tn)
        if lo < hi:
            out.append(
                _Sync(
                    (lo, hi),
                    (oo + lo - ob, oo + hi - ob),
                    (tt + lo - tb, tt + hi - tb),
                ),
            )
        if ob + on < tb + tn:
            i += 1
        else:
            j += 1
    end = (len(base), len(ours), len(theirs))
    out.append(_Sync((end[0], end[0]), (end[1], end[1]), (end[2], end[2])))
    return out


def _terminated(lines: Lines) -> list[bytes]:
    if lines and not lines[-1].endswith(b"\n"):
        return [*lines[:-1], lines[-1] + b"\n"]
    return list(lines)


def merge3(base: bytes, ours: bytes, theirs: bytes) -> TextMerge:
    """Three-way merge ``ours`` and ``theirs`` against their common ``base``."""
    b, o, t = split_lines(base), split_lines(ours), split_lines(theirs)
    out: list[bytes] = []
    conflicts: list[Region] = []
    pb = po = pt = 0
    for sync in _sync_points(b, o, t):
        gb, go, gt = b[pb : sync.base[0]], o[po : sync.ours[0]], t[pt : sync.theirs[0]]
        if gt in (go, gb):
            out.extend(go)
        elif go == gb:
            out.extend(gt)
        else:
            start = len(out) + 1
            out.append(OURS_MARKER)
            out.extend(_terminated(go))
            out.append(BASE_MARKER)
            out.extend(_terminated(gb))
            out.append(SPLIT_MARKER)
            out.extend(_terminated(gt))
            out.append(THEIRS_MARKER)
            conflicts.append(Region(start, len(out)))
        out.extend(b[sync.base[0] : sync.base[1]])
        pb, po, pt = sync.base[1], sync.ours[1], sync.theirs[1]
    return TextMerge(b"".join(out), tuple(conflicts))
