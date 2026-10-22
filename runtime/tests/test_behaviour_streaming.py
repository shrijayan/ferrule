# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""SSE parsing and streaming through the rendered runtime."""

# ruff: noqa: ANN401 - the rendered SDK is imported at test time, so it is typed as Any.

from __future__ import annotations

from typing import Any

import httpx
import pytest


def events(sdk: Any, chunks: list[bytes]) -> list[Any]:
    return list(sdk._streaming.SSEDecoder().iter_bytes(iter(chunks)))  # noqa: SLF001


def test_sse_basic_event(sdk: Any) -> None:
    (sse,) = events(sdk, [b'event: completion\ndata: {"a": 1}\nid: 7\nretry: 1000\n\n'])
    assert (sse.event, sse.data, sse.id, sse.retry) == ("completion", '{"a": 1}', "7", 1000)
    assert sse.json() == {"a": 1}


def test_sse_multiline_data_and_comments(sdk: Any) -> None:
    (sse,) = events(sdk, [b": keep-alive\ndata: line1\ndata: line2\n\n"])
    assert sse.data == "line1\nline2"
    assert sse.event is None


def test_sse_chunk_boundaries_and_crlf(sdk: Any) -> None:
    chunks = [b"data: fi", b"rst\r\n\r", b"\ndata: second\r\r", b"data: third\n", b"\n"]
    assert [e.data for e in events(sdk, chunks)] == ["first", "second", "third"]


def test_sse_ignores_invalid_retry_and_null_id(sdk: Any) -> None:
    (sse,) = events(sdk, [b"retry: soon\nid: a\x00b\ndata: x\nunknown: y\n\n"])
    assert sse.retry is None
    assert sse.id is None
    assert sse.data == "x"


def test_sse_field_without_space_and_no_value(sdk: Any) -> None:
    (sse,) = events(sdk, [b"data:x\nevent\n\n"])
    assert sse.data == "x"
    assert sse.event is None  # an empty event name means "message"


def test_sse_unterminated_trailing_event_is_dropped(sdk: Any) -> None:
    assert [e.data for e in events(sdk, [b"data: a\n\ndata: b"])] == ["a"]


def test_sse_last_event_id_persists(sdk: Any) -> None:
    first, second = events(sdk, [b"id: 1\ndata: a\n\ndata: b\n\n"])
    assert (first.id, second.id) == ("1", "1")


def _sse_client(sdk: Any, body: bytes) -> Any:
    def handler(_request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, headers={"content-type": "text/event-stream"}, content=body)

    return sdk.Fer(http_client=httpx.Client(transport=httpx.MockTransport(handler)))


def test_stream_yields_parsed_json(sdk: Any) -> None:
    body = b'data: {"n": 1}\n\ndata: {"n": 2}\n\n'
    stream = _sse_client(sdk, body).post(
        "/s",
        cast_to=object,
        body={},
        stream=True,
        stream_cls=sdk._streaming.Stream[object],  # noqa: SLF001
    )
    assert list(stream) == [{"n": 1}, {"n": 2}]
    assert stream.response.is_closed


def test_stream_close_early_closes_response(sdk: Any) -> None:
    body = b'data: {"n": 1}\n\ndata: {"n": 2}\n\n'
    stream = _sse_client(sdk, body).post(
        "/s",
        cast_to=object,
        body={},
        stream=True,
        stream_cls=sdk._streaming.Stream[object],  # noqa: SLF001
    )
    with stream as s:
        assert next(iter(s)) == {"n": 1}
    assert stream.response.is_closed


@pytest.mark.anyio
async def test_async_stream_yields_parsed_json(sdk: Any) -> None:
    body = b'data: {"n": 1}\n\nevent: x\ndata: {"n": 2}\n\n'

    def handler(_request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, headers={"content-type": "text/event-stream"}, content=body)

    client = sdk.AsyncFer(http_client=httpx.AsyncClient(transport=httpx.MockTransport(handler)))
    stream = await client.post(
        "/s",
        cast_to=object,
        body={},
        stream=True,
        stream_cls=sdk._streaming.AsyncStream[object],  # noqa: SLF001
    )
    assert [item async for item in stream] == [{"n": 1}, {"n": 2}]
