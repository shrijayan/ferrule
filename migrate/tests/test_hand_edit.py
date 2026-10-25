# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
from ferrule.hand_edit import Hunk, parse_hunks, revert, split_lines

BASE = b"".join(b"line%d\n" % i for i in range(1, 9))

# A maintainer's commit: line8 -> custom8, plus a method after it (isaacus Span.decode).
EDIT = b"""diff --git a/foo.py b/foo.py
index 1111111..2222222 100644
--- a/foo.py
+++ b/foo.py
@@ -5,4 +5,5 @@
 line5
 line6
 line7
-line8
+custom8
+def decode(): ...
"""


def test_split_lines_keeps_terminators_and_a_last_partial_line() -> None:
    assert split_lines(b"a\r\nb\nc") == [b"a\r\n", b"b\n", b"c"]
    assert split_lines(b"a\n") == [b"a\n"]
    assert split_lines(b"") == []


def test_parse_hunks() -> None:
    assert parse_hunks(EDIT) == [
        Hunk(
            5,
            (b"line5\n", b"line6\n", b"line7\n", b"line8\n"),
            (b"line5\n", b"line6\n", b"line7\n", b"custom8\n", b"def decode(): ...\n"),
        )
    ]
    assert parse_hunks(b"Binary files a/x.png and b/x.png differ\n") == []


def test_parse_hunks_reads_no_newline_markers() -> None:
    diff = (
        b"@@ -1,2 +1,2 @@\n a\n-b\n\\ No newline at end of file\n+c\n\\ No newline at end of file\n"
    )
    assert parse_hunks(diff) == [Hunk(1, (b"a\n", b"b"), (b"a\n", b"c"))]
    context = b"@@ -1,2 +1,2 @@\n-a\n+b\n z\n\\ No newline at end of file\n"
    assert parse_hunks(context) == [Hunk(1, (b"a\n", b"z"), (b"b\n", b"z"))]


def test_parse_hunks_stops_at_the_next_file() -> None:
    diff = b"@@ -1 +1 @@\n-a\n+b\ndiff --git a/y b/y\n@@ -1 +1 @@\n-c\n+d\n"
    assert parse_hunks(diff) == [Hunk(1, (b"a\n",), (b"b\n",)), Hunk(1, (b"c\n",), (b"d\n",))]


def test_revert_undoes_an_edit_a_later_generation_carried() -> None:
    # The bot regenerated line2 and kept the edit; reverting leaves the bot's line2.
    carried = BASE.replace(b"line2", b"bot2").replace(b"line8\n", b"custom8\ndef decode(): ...\n")
    assert revert(carried, EDIT) == BASE.replace(b"line2", b"bot2")


def test_revert_finds_a_moved_hunk_nearest_its_line() -> None:
    moved = b"top\n" * 20 + BASE.replace(b"line8\n", b"custom8\ndef decode(): ...\n")
    assert revert(moved, EDIT) == b"top\n" * 20 + BASE


def test_revert_skips_a_hunk_the_file_no_longer_has() -> None:
    assert revert(BASE, EDIT) == BASE


def test_revert_skips_a_hunk_the_generator_now_writes_itself() -> None:
    carried = BASE.replace(b"line8\n", b"custom8\ndef decode(): ...\n")
    assert revert(carried, EDIT, generated=b"header\n" + carried) == carried


def test_revert_skips_a_hunk_with_an_empty_new_side() -> None:
    deleted = b"@@ -1,2 +0,0 @@\n-a\n-b\n"
    assert revert(b"x\n", deleted) == b"x\n"


def test_revert_tracks_the_shift_of_earlier_hunks() -> None:
    text = b"a\nB\nB\nc\nd\ne\nf\ng\nh\nZ\n"
    diff = b"@@ -1,2 +1,3 @@\n a\n+B\n B\n@@ -8,3 +9,3 @@\n g\n h\n-z\n+Z\n"
    assert revert(text, diff) == b"a\nB\nc\nd\ne\nf\ng\nh\nz\n"
