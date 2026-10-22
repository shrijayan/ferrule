# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Iterate every generated pagination scheme against an httpx MockTransport."""

# ruff: noqa: ANN401 - the rendered SDK is imported at test time, so it is typed as Any.

from __future__ import annotations

from typing import Any

import httpx
import pytest


class Pages:
    """Serve pages keyed by a query parameter; record the params of every request."""

    def __init__(self, key: str, pages: dict[str | None, dict[str, Any]]) -> None:
        self.key = key
        self.pages = pages
        self.seen: list[dict[str, str]] = []

    def __call__(self, request: httpx.Request) -> httpx.Response:
        params = dict(request.url.params)
        self.seen.append(params)
        return httpx.Response(200, json=self.pages[params.get(self.key)])


def sync_client(sdk: Any, handler: Pages) -> Any:
    return sdk.Fer(http_client=httpx.Client(transport=httpx.MockTransport(handler)))


def ids(items: list[Any]) -> list[Any]:
    return [i["id"] if isinstance(i, dict) else i.id for i in items]


def item_model(sdk: Any) -> Any:
    class Item(sdk.BaseModel):
        id: str | None = None

    return Item


def test_cursor(sdk: Any) -> None:
    handler = Pages(
        "cursor",
        {
            None: {"data": [{"id": 1}, {"id": 2}], "next_cursor": "c2"},
            "c2": {"data": [{"id": 3}], "next_cursor": "c3"},
            "c3": {"data": [{"id": 4}], "next_cursor": None},
        },
    )
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncCursor[object]
    )
    assert ids(list(page)) == [1, 2, 3, 4]
    assert [p.get("cursor") for p in handler.seen] == [None, "c2", "c3"]


def test_cursor_stops_on_empty_page(sdk: Any) -> None:
    handler = Pages("cursor", {None: {"data": [], "next_cursor": "c2"}})
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncCursor[object]
    )
    assert list(page) == []
    assert not page.has_next_page()


def test_nested_cursor(sdk: Any) -> None:
    handler = Pages(
        "after",
        {
            None: {"data": [{"id": 1}], "page_info": {"after": "a"}},
            "a": {"data": [{"id": 2}], "page_info": {"after": None}},
        },
    )
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncNestedCursor[object]
    )
    assert ids(list(page)) == [1, 2]

    handler = Pages("after", {None: {"data": [{"id": 1}]}})
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncNestedCursor[object]
    )
    assert page.next_page_info() is None


def test_cursor_id_forwards_and_backwards(sdk: Any) -> None:
    handler = Pages(
        "starting_after",
        {
            None: {"data": [{"id": "a"}, {"id": "b"}], "has_more": True},
            "b": {"data": [{"id": "c"}], "has_more": False},
        },
    )
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=item_model(sdk), page=sdk.pagination.SyncCursorIDPage[item_model(sdk)]
    )
    assert ids(list(page)) == ["a", "b", "c"]

    handler = Pages("ending_before", {"z": {"data": [{"id": "y"}, {"id": "x"}], "has_more": True}})
    page = sync_client(sdk, handler).get_api_list(
        "/l",
        model=item_model(sdk),
        page=sdk.pagination.SyncCursorIDPage[item_model(sdk)],
        options={"params": {"ending_before": "z"}},
    )
    info = page.next_page_info()
    assert info.params == {"ending_before": "y"}


def test_cursor_id_items_without_id_stop(sdk: Any) -> None:
    handler = Pages("starting_after", {None: {"data": [{"name": "no id"}], "has_more": True}})
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=item_model(sdk), page=sdk.pagination.SyncCursorIDPage[item_model(sdk)]
    )
    assert page.next_page_info() is None


def test_cursor_url(sdk: Any) -> None:
    handler = Pages(
        "p",
        {
            None: {"data": [{"id": 1}], "next": "http://fer.test/l?p=2"},
            "2": {"data": [{"id": 2}], "next": None},
        },
    )
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncCursorURLPage[object]
    )
    assert ids(list(page)) == [1, 2]


def test_offset_with_total(sdk: Any) -> None:
    handler = Pages(
        "offset",
        {
            None: {"items": [{"id": 1}, {"id": 2}], "total": 3},
            "2": {"items": [{"id": 3}], "total": 3},
        },
    )
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncOffsetPage[object]
    )
    assert ids(list(page)) == [1, 2, 3]
    assert len(handler.seen) == 2


def test_offset_without_total_stops_on_empty_page(sdk: Any) -> None:
    handler = Pages("offset", {None: {"items": [{"id": 1}]}, "1": {"items": []}})
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncOpenOffsetPage[object]
    )
    assert ids(list(page)) == [1]

    handler = Pages("offset", {None: {"items": [{"id": 1}]}})
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncOffsetPage[object]
    )
    assert page.next_page_info() is None


def test_offset_rejects_non_integer_offset(sdk: Any) -> None:
    handler = Pages("offset", {"x": {"items": [{"id": 1}]}})
    page = sync_client(sdk, handler).get_api_list(
        "/l",
        model=object,
        page=sdk.pagination.SyncOpenOffsetPage[object],
        options={"params": {"offset": "x"}},
    )
    with pytest.raises(ValueError, match="offset"):
        page.next_page_info()


def test_page_number(sdk: Any) -> None:
    handler = Pages(
        "page",
        {
            None: {"items": [{"id": 1}], "page": 1, "pages": 2},
            "2": {"items": [{"id": 2}], "page": 2, "pages": 2},
        },
    )
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncPageNumberPage[object]
    )
    assert ids(list(page)) == [1, 2]
    assert [p.get("page") for p in handler.seen] == [None, "2"]

    handler = Pages("page", {None: {"items": [{"id": 1}]}})
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncPageNumberPage[object]
    )
    assert page.next_page_info().params == {"page": 2}


def test_fake_page(sdk: Any) -> None:
    handler = Pages("x", {None: {"data": [{"id": 1}], "has_more": True}})
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncFakePage[object]
    )
    assert ids(list(page)) == [1]
    assert not page.has_next_page()

    handler = Pages("x", {None: {"data": [{"id": 1}], "has_more": False}})
    page = sync_client(sdk, handler).get_api_list(
        "/l", model=object, page=sdk.pagination.SyncFakePage[object]
    )
    assert not page.has_next_page()


@pytest.mark.anyio
async def test_async_cursor_and_page_number(sdk: Any) -> None:
    handler = Pages(
        "cursor",
        {None: {"data": [{"id": 1}], "next_cursor": "c"}, "c": {"data": [{"id": 2}]}},
    )
    client = sdk.AsyncFer(http_client=httpx.AsyncClient(transport=httpx.MockTransport(handler)))
    paginator = client.get_api_list("/l", model=object, page=sdk.pagination.AsyncCursor[object])
    assert ids([item async for item in paginator]) == [1, 2]

    handler = Pages(
        "page",
        {
            None: {"items": [{"id": 1}], "page": 1, "pages": 2},
            "2": {"items": [{"id": 2}], "page": 2, "pages": 2},
        },
    )
    client = sdk.AsyncFer(http_client=httpx.AsyncClient(transport=httpx.MockTransport(handler)))
    paginator = client.get_api_list(
        "/l", model=object, page=sdk.pagination.AsyncPageNumberPage[object]
    )
    assert ids([item async for item in paginator]) == [1, 2]


@pytest.mark.anyio
@pytest.mark.parametrize(
    ("cls", "key", "pages"),
    [
        (
            "AsyncNestedCursor",
            "after",
            {None: {"data": [{"id": 1}], "page_info": {"after": "a"}}, "a": {"data": [{"id": 2}]}},
        ),
        (
            "AsyncCursorURLPage",
            "p",
            {
                None: {"data": [{"id": 1}], "next": "http://fer.test/l?p=2"},
                "2": {"data": [{"id": 2}]},
            },
        ),
        (
            "AsyncOffsetPage",
            "offset",
            {None: {"items": [{"id": 1}], "total": 2}, "1": {"items": [{"id": 2}], "total": 2}},
        ),
        (
            "AsyncOpenOffsetPage",
            "offset",
            {None: {"items": [{"id": 1}]}, "1": {"items": [{"id": 2}]}, "2": {"items": []}},
        ),
        ("AsyncFakePage", "x", {None: {"data": [{"id": 1}, {"id": 2}], "has_more": True}}),
    ],
)
async def test_async_schemes(
    sdk: Any, cls: str, key: str, pages: dict[str | None, dict[str, Any]]
) -> None:
    client = sdk.AsyncFer(
        http_client=httpx.AsyncClient(transport=httpx.MockTransport(Pages(key, pages)))
    )
    paginator = client.get_api_list("/l", model=object, page=getattr(sdk.pagination, cls)[object])
    assert ids([item async for item in paginator]) == [1, 2]


@pytest.mark.anyio
async def test_async_cursor_id(sdk: Any) -> None:
    pages: dict[str | None, dict[str, Any]] = {
        None: {"data": [{"id": "a"}], "has_more": True},
        "a": {"data": [{"id": "b"}], "has_more": False},
    }
    handler = Pages("starting_after", pages)
    client = sdk.AsyncFer(http_client=httpx.AsyncClient(transport=httpx.MockTransport(handler)))
    item = item_model(sdk)
    paginator = client.get_api_list("/l", model=item, page=sdk.pagination.AsyncCursorIDPage[item])
    assert ids([i async for i in paginator]) == ["a", "b"]
