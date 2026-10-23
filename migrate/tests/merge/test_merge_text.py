# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
from hypothesis import given
from hypothesis import strategies as st

from ferrule.merge.text import OURS_MARKER, THEIRS_MARKER, Region, merge3

BASE = b"a\nb\nc\nd\ne\n"

lines = st.lists(st.sampled_from([b"x\n", b"y\n", b"z\n", b"w"]), max_size=8).map(b"".join)


def test_disjoint_edits_merge_cleanly() -> None:
    result = merge3(BASE, b"A\nb\nc\nd\ne\n", b"a\nb\nc\nd\nE\n")
    assert result.clean
    assert result.content == b"A\nb\nc\nd\nE\n"


def test_same_edit_on_both_sides_is_taken_once() -> None:
    result = merge3(BASE, b"a\nB\nc\nd\ne\n", b"a\nB\nc\nd\ne\n")
    assert result.content == b"a\nB\nc\nd\ne\n"


def test_overlapping_edits_conflict_with_markers() -> None:
    result = merge3(BASE, b"a\nOURS\nc\nd\ne\n", b"a\nTHEIRS\nc\nd\ne\n")
    assert not result.clean
    assert result.conflicts == (Region(2, 8),)
    out = result.content.splitlines(keepends=True)
    assert out[1] == OURS_MARKER
    assert out[7] == THEIRS_MARKER
    assert b"OURS\n" in out
    assert b"THEIRS\n" in out


def test_unterminated_last_lines_get_newlines_inside_markers() -> None:
    result = merge3(b"a", b"b", b"c")
    assert result.content.count(b"\n") == 7


@given(lines, lines)
def test_unchanged_side_yields_the_other(base: bytes, other: bytes) -> None:
    assert merge3(base, base, other).content == other
    assert merge3(base, other, base).content == other


@given(lines, lines)
def test_identical_sides_merge_to_that_side(base: bytes, side: bytes) -> None:
    assert merge3(base, side, side).content == side
