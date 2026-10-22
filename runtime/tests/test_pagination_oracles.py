# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Generated pagination.py is byte-identical to the pinned oracle SDKs' pagination.py."""

from __future__ import annotations

from pathlib import Path

import pytest

from ferrule_runtime import PageField, PageModel, PageScheme, render_pagination

CACHE = Path(__file__).resolve().parents[2] / "oracles" / "cache"

ITEMS = "List[_T]"
OPT_STR = "Optional[str]"
OPT_INT = "Optional[int]"

LITHIC = (
    PageScheme(
        name="CursorPage",
        kind="cursor_id",
        items="data",
        fields=(PageField("data", ITEMS), PageField("has_more", "bool")),
        has_more="has_more",
        param="starting_after",
        before_param="ending_before",
        item_id="token",
    ),
    PageScheme(
        name="SinglePage",
        kind="fake_page",
        items="data",
        fields=(PageField("data", ITEMS), PageField("has_more", "bool")),
        has_more="has_more",
    ),
)

HONCHO = (
    PageScheme(
        name="Page",
        kind="page_number",
        items="items",
        fields=(
            PageField("total", OPT_INT, optional=True),
            PageField("items", ITEMS),
            PageField("page", OPT_INT, optional=True),
            PageField("size", OPT_INT, optional=True),
            PageField("pages", OPT_INT, optional=True),
        ),
        param="page",
        path=("page",),
        total_path=("pages",),
    ),
)


def _page_info_cursor(prefix: str, items: str) -> PageScheme:
    info = f"{prefix}CursorPageInfo"
    return PageScheme(
        name=f"{prefix}Cursor",
        kind="cursor",
        items=items,
        fields=(
            PageField(items, ITEMS),
            PageField("page_info", f"Optional[{info}]", optional=True),
        ),
        models=(PageModel(info, (PageField("after", OPT_STR, optional=True),)),),
        param="after",
        path=("page_info", "after"),
    )


KNOCK_MGMT = (_page_info_cursor("Entries", "entries"),)

KNOCK = (
    _page_info_cursor("Entries", "entries"),
    _page_info_cursor("Items", "items"),
    PageScheme(
        name="SlackChannelsCursor",
        kind="cursor",
        items="slack_channels",
        fields=(
            PageField("next_cursor", OPT_STR, optional=True),
            PageField("slack_channels", ITEMS),
        ),
        param="query_options.cursor",
        path=("next_cursor",),
    ),
    PageScheme(
        name="MsTeamsPagination",
        kind="cursor",
        items="ms_teams_teams",
        fields=(
            PageField("skip_token", OPT_STR, optional=True),
            PageField("ms_teams_teams", ITEMS),
        ),
        param="query_options.$skiptoken",
        path=("skip_token",),
    ),
)

OGX = (
    PageScheme(
        name="OpenAICursorPage",
        kind="cursor",
        items="data",
        fields=(
            PageField("data", ITEMS),
            PageField("has_more", "Optional[bool]", optional=True),
            PageField("last_id", OPT_STR, optional=True),
        ),
        has_more="has_more",
        param="after",
        path=("last_id",),
    ),
)

CASES = [
    ("lithic-python", "src/lithic", LITHIC),
    ("honcho-python-core", "src/honcho_core", HONCHO),
    ("knock-mgmt-python", "src/knock_mapi", KNOCK_MGMT),
    ("knock-python", "src/knockapi", KNOCK),
    ("ogx-client-python", "src/ogx_client", OGX),
]


@pytest.mark.parametrize(("oracle", "package_dir", "schemes"), CASES, ids=[c[0] for c in CASES])
def test_pagination_matches_oracle(
    oracle: str, package_dir: str, schemes: tuple[PageScheme, ...]
) -> None:
    path = CACHE / oracle / package_dir / "pagination.py"
    if not path.is_file():
        pytest.skip(f"oracle cache absent: python3 oracles/fetch.py {oracle}")
    assert render_pagination(schemes) == path.read_bytes().decode()
