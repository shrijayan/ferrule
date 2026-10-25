# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""The template engine every language shares: ``//`` directives, slots and gated paths."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from ferrule_runtime import RenderError, render_tree
from ferrule_runtime._render import resolve_directives

if TYPE_CHECKING:
    from pathlib import Path


def test_slash_directives_and_slots() -> None:
    text = (
        "a\n//ferrule:if x\nb\n//ferrule:slot s\n//ferrule:else\nc\n//ferrule:endif\n"
        "//ferrule:slot s\n"
    )
    assert resolve_directives(text, {"x": True}, "t", comment="//", slots={"s": "S\n"}) == (
        "a\nb\nS\nS\n"
    )
    assert resolve_directives(text, {"x": False}, "t", comment="//", slots={"s": ""}) == "a\nc\n"


@pytest.mark.parametrize(
    ("text", "slots", "match"),
    [
        ("//ferrule:slot nope\n", {}, "unknown template slot"),
        ("//ferrule:slot s\n", None, "unknown template slot"),
        ("//ferrule:iff x\n", {}, "malformed directive"),
        ("//ferrule:else\n", {}, "//ferrule:else without //ferrule:if"),
        ("//ferrule:if x\n", {}, "unterminated //ferrule:if"),
    ],
)
def test_slash_directive_errors(text: str, slots: dict[str, str] | None, match: str) -> None:
    with pytest.raises(RenderError, match=match):
        resolve_directives(text, {"x": True}, "t", comment="//", slots=slots)


def test_render_tree_gates_prefixes_and_rejects_leftover_tokens(tmp_path: Path) -> None:
    (tmp_path / "keep").mkdir()
    (tmp_path / "drop").mkdir()
    (tmp_path / "keep" / "a.ts").write_text("__FerruleClient__\n")
    (tmp_path / "drop" / "b.ts").write_text("__FERRULE_UNKNOWN__\n")
    files = render_tree(
        tmp_path,
        replacements={"__FerruleClient__": "Acme"},
        flags={},
        comment="//",
        gated={"drop/": False, "keep/": True},
    )
    assert files == {"keep/a.ts": "Acme\n"}
    with pytest.raises(RenderError, match="__FERRULE_UNKNOWN__"):
        render_tree(tmp_path, replacements={}, flags={}, comment="//")


def test_fits_keeps_the_one_line_form_when_it_fits() -> None:
    text = "#ferrule:if fits\nshort\n#ferrule:else\nwrapped\n#ferrule:endif\n"
    assert resolve_directives(text, {}, "t", fits=lambda line: len(line) <= 5) == "short\n"
    assert resolve_directives(text, {}, "t", fits=lambda line: len(line) <= 4) == "wrapped\n"


@pytest.mark.parametrize(
    "text",
    [
        "#ferrule:if fits\na\nb\n#ferrule:endif\n",  # the branch is one line
        "#ferrule:if fits\n#ferrule:if x\n#ferrule:endif\n#ferrule:endif\n",
        "#ferrule:if fits\n",
    ],
)
def test_fits_needs_a_one_line_branch(text: str) -> None:
    with pytest.raises(RenderError, match="if fits needs"):
        resolve_directives(text, {"x": True}, "t", fits=lambda _line: True)


def test_fits_needs_a_line_length() -> None:
    with pytest.raises(RenderError, match="if fits needs"):
        resolve_directives("#ferrule:if fits\na\n#ferrule:endif\n", {}, "t")


def test_render_tree_replaces_path_tokens_and_measures_rendered_lines(tmp_path: Path) -> None:
    (tmp_path / "__ferrule_pkg_path__").mkdir()
    (tmp_path / "__ferrule_pkg_path__" / "__FerruleClient__.kt").write_text(
        "//ferrule:if fits\nval __FerruleClient__\n//ferrule:else\nval\n    __FerruleClient__\n"
        "//ferrule:endif\n"
    )

    def render(client: str) -> dict[str, str]:
        return render_tree(
            tmp_path,
            replacements={"__ferrule_pkg_path__": "org/acme", "__FerruleClient__": client},
            flags={},
            comment="//",
            line_length=lambda rel: 10 if rel.endswith(".kt") else None,
        )

    assert render("Acme") == {"org/acme/Acme.kt": "val Acme\n"}
    assert render("AcmeClient") == {"org/acme/AcmeClient.kt": "val\n    AcmeClient\n"}
