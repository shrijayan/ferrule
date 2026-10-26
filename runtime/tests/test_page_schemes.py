# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Page schemes derived from the converter's x-ferrule-page-schemes match the pinned oracles."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from test_pagination_oracles import HONCHO, KNOCK, KNOCK_MGMT, OGX

from ferrule_runtime import PageField, PageModel, PageScheme, page_schemes

SNAPSHOTS = Path(__file__).resolve().parents[2] / "crates/converter/tests/snapshots/oracles"


def configured(name: str) -> list[dict[str, object]]:
    data = json.loads((SNAPSHOTS / f"{name}.json.snap").read_text())
    return data["root"]["x-ferrule-page-schemes"]


@pytest.mark.parametrize(
    ("config", "expected"),
    [
        ("honcho-config", HONCHO),
        ("knock-config", KNOCK),
        ("knock-mgmt-config", KNOCK_MGMT),
        ("ogx-config", OGX),
    ],
)
def test_page_schemes_match_the_oracles(config: str, expected: tuple[PageScheme, ...]) -> None:
    assert page_schemes(configured(config)) == expected


def test_page_schemes_type_other_fields_and_nested_cursors() -> None:
    # The opencode SDK's `{data: [...], cursor: {next, previous}}` page.
    entry = {
        "name": "cursor_page",
        "className": "CursorPage",
        "kind": "cursor",
        "request": [{"name": "cursor", "type": "string", "purpose": "next_cursor_param"}],
        "response": [
            {"name": "data", "type": "array", "purpose": "items"},
            {
                "name": "cursor",
                "type": "object",
                "fields": [
                    {"name": "next", "type": "string", "purpose": "next_cursor_field"},
                    {"name": "count", "type": "number"},
                ],
            },
            {"name": "ids", "type": "array"},
            {"name": "raw"},
        ],
    }
    assert page_schemes([entry]) == (
        PageScheme(
            name="CursorPage",
            kind="cursor",
            items="data",
            fields=(
                PageField("data", "List[_T]"),
                PageField("cursor", "Optional[CursorPageCursor]", optional=True),
                PageField("ids", "Optional[List[object]]", optional=True),
                PageField("raw", "Optional[object]", optional=True),
            ),
            models=(
                PageModel(
                    "CursorPageCursor",
                    (
                        PageField("next", "Optional[str]", optional=True),
                        PageField("count", "Optional[float]", optional=True),
                    ),
                ),
            ),
            param="cursor",
            path=("cursor", "next"),
        ),
    )
