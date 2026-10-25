# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Undo a hand edit that a file still carries (pure: bytes in, bytes out).

Stainless merges custom code into every later generation, so a maintainer's edit to a
generated file lives on in the bot's commits after it. ``ferrule adopt`` finds such an edit
by replaying the maintainer's commit backwards on the file as it is now: each hunk whose new
side the file still contains is reverted, and a hunk the file no longer contains (a later
generation overwrote it) is skipped. What is left is the file without the custom code, which
is the base of the patch adopt records.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

_HEADER = re.compile(rb"^@@ -\d+(?:,\d+)? \+(?P<start>\d+)(?:,\d+)? @@")
_NO_NEWLINE = b"\\"
_BODY = frozenset({b" ", b"-", b"+"})

type Lines = list[bytes]


@dataclass(frozen=True)
class Hunk:
    """One hunk of a unified diff: its 1-based start in the new file and both sides."""

    start: int
    old: tuple[bytes, ...]
    new: tuple[bytes, ...]


def split_lines(data: bytes) -> Lines:
    r"""Split ``data`` on ``\n`` only, each line keeping its terminator (git's notion)."""
    parts = data.split(b"\n")
    lines = [part + b"\n" for part in parts[:-1]]
    if parts[-1]:
        lines.append(parts[-1])
    return lines


def _hunk(start: int, body: Lines) -> Hunk:
    old: Lines = []
    new: Lines = []
    tag = b""
    for line in body:
        if line.startswith(_NO_NEWLINE):
            # "\ No newline at end of file": the previous line has no terminator.
            for side in {b" ": (old, new), b"-": (old,), b"+": (new,)}.get(tag, ()):
                side[-1] = side[-1].removesuffix(b"\n")
            continue
        tag, text = line[:1], line[1:]
        if tag not in _BODY:
            break  # the next file's header
        if tag != b"+":
            old.append(text)
        if tag != b"-":
            new.append(text)
    return Hunk(start, tuple(old), tuple(new))


def parse_hunks(diff: bytes) -> list[Hunk]:
    """Return the hunks of a one-file unified diff, in order; a binary diff has none."""
    chunks: list[tuple[int, Lines]] = []
    for line in split_lines(diff):
        header = _HEADER.match(line)
        if header is not None:
            chunks.append((int(header["start"]), []))
        elif chunks:
            chunks[-1][1].append(line)
    return [_hunk(start, body) for start, body in chunks]


def _find(lines: Lines, block: tuple[bytes, ...], near: int) -> int | None:
    """Return the start of ``block`` in ``lines`` closest to ``near``, or None."""
    size = len(block)
    starts = [i for i in range(len(lines) - size + 1) if tuple(lines[i : i + size]) == block]
    return min(starts, key=lambda i: (abs(i - near), i)) if starts else None


def revert(data: bytes, diff: bytes, generated: bytes = b"") -> bytes:
    """Undo every hunk of ``diff`` that ``data`` still contains; leave the others.

    A hunk whose new side ``generated`` (the regenerated file) also contains is left too: the
    generator writes that text itself, so it is no longer custom code (a maintainer's
    formatting fix that a later Stainless release made on its own, say).
    """
    lines = split_lines(data)
    regenerated = split_lines(generated)
    shift = 0
    for hunk in parse_hunks(diff):
        if not hunk.new:
            continue  # nothing anchors a hunk whose new side is empty
        at = _find(lines, hunk.new, hunk.start - 1 + shift)
        if at is None or _find(regenerated, hunk.new, 0) is not None:
            continue
        lines[at : at + len(hunk.new)] = hunk.old
        shift += len(hunk.old) - len(hunk.new)
    return b"".join(lines)
