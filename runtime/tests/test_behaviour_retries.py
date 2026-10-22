# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Fault injection against the rendered runtime: retries, backoff, retry-after, timeouts."""

# ruff: noqa: ANN401 - the rendered SDK is imported at test time, so it is typed as Any.

from __future__ import annotations

import contextlib
from datetime import UTC, datetime, timedelta
from email.utils import format_datetime
from typing import Any

import httpx
import pytest


class Recorder:
    """A MockTransport handler replaying scripted responses and recording requests."""

    def __init__(self, *responses: httpx.Response | Exception) -> None:
        self.script = list(responses)
        self.requests: list[httpx.Request] = []

    def __call__(self, request: httpx.Request) -> httpx.Response:
        self.requests.append(request)
        item = self.script.pop(0) if len(self.script) > 1 else self.script[0]
        if isinstance(item, Exception):
            raise item
        return item


@pytest.fixture
def sleeps(sdk: Any, monkeypatch: pytest.MonkeyPatch) -> list[float]:
    recorded: list[float] = []
    base = sdk._base_client  # noqa: SLF001

    async def fake_async_sleep(seconds: float) -> None:
        recorded.append(seconds)

    monkeypatch.setattr(base.time, "sleep", recorded.append)
    monkeypatch.setattr(base.anyio, "sleep", fake_async_sleep)
    monkeypatch.setattr(base, "random", lambda: 0.0)
    return recorded


def client(sdk: Any, recorder: Recorder, **kwargs: Any) -> Any:
    return sdk.Fer(http_client=httpx.Client(transport=httpx.MockTransport(recorder)), **kwargs)


def ok() -> httpx.Response:
    return httpx.Response(200, json={"ok": True})


def test_defaults_match_stainless(sdk: Any) -> None:
    assert sdk.DEFAULT_MAX_RETRIES == 2
    assert httpx.Timeout(timeout=60, connect=5.0) == sdk.DEFAULT_TIMEOUT
    c = sdk.Fer()
    assert c.max_retries == 2
    assert c.timeout == httpx.Timeout(timeout=60, connect=5.0)


@pytest.mark.parametrize("status", [408, 409, 429, 500, 502, 503, 504])
def test_retryable_statuses_are_retried(sdk: Any, sleeps: list[float], status: int) -> None:
    rec = Recorder(httpx.Response(status, json={}), ok())
    assert client(sdk, rec).get("/x", cast_to=object) == {"ok": True}
    assert len(rec.requests) == 2
    assert len(sleeps) == 1


@pytest.mark.parametrize("status", [400, 401, 403, 404, 422])
def test_client_errors_are_not_retried(sdk: Any, sleeps: list[float], status: int) -> None:
    rec = Recorder(httpx.Response(status, json={"error": "nope"}))
    with pytest.raises(sdk.APIStatusError) as info:
        client(sdk, rec).get("/x", cast_to=object)
    assert info.value.status_code == status
    assert len(rec.requests) == 1
    assert sleeps == []


def test_max_retries_two_then_raises(sdk: Any, sleeps: list[float]) -> None:
    rec = Recorder(httpx.Response(500, json={}))
    with pytest.raises(sdk.InternalServerError):
        client(sdk, rec).get("/x", cast_to=object)
    assert len(rec.requests) == 3
    assert [r.headers["x-stainless-retry-count"] for r in rec.requests] == ["0", "1", "2"]
    assert sleeps == [0.5, 1.0]


def test_per_request_max_retries(sdk: Any, sleeps: list[float]) -> None:
    rec = Recorder(httpx.Response(503, json={}))
    with pytest.raises(sdk.InternalServerError):
        client(sdk, rec).get("/x", cast_to=object, options={"max_retries": 0})
    assert len(rec.requests) == 1
    assert sleeps == []


def test_backoff_doubles_and_caps_at_eight_seconds(sdk: Any, sleeps: list[float]) -> None:
    rec = Recorder(httpx.Response(500, json={}))
    with pytest.raises(sdk.InternalServerError):
        client(sdk, rec, max_retries=6).get("/x", cast_to=object)
    assert sleeps == [0.5, 1.0, 2.0, 4.0, 8.0, 8.0]


def test_backoff_jitter_bounds(
    sdk: Any, sleeps: list[float], monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(sdk._base_client, "random", lambda: 1.0)  # noqa: SLF001
    rec = Recorder(httpx.Response(500, json={}))
    with pytest.raises(sdk.InternalServerError):
        client(sdk, rec, max_retries=5).get("/x", cast_to=object)
    assert sleeps == [0.375, 0.75, 1.5, 3.0, 6.0]


@pytest.mark.parametrize(
    ("headers", "expected"),
    [
        ({"retry-after": "2"}, 2.0),
        ({"retry-after": "0.25"}, 0.25),
        ({"retry-after-ms": "1500"}, 1.5),
        ({"retry-after-ms": "1500", "retry-after": "9"}, 1.5),
        ({"retry-after": "61"}, 0.5),
        ({"retry-after": "0"}, 0.5),
        ({"retry-after": "soon"}, 0.5),
    ],
)
def test_retry_after(
    sdk: Any, sleeps: list[float], headers: dict[str, str], expected: float
) -> None:
    rec = Recorder(httpx.Response(429, headers=headers, json={}), ok())
    client(sdk, rec).get("/x", cast_to=object)
    assert sleeps == [expected]


def test_retry_after_http_date(sdk: Any, sleeps: list[float]) -> None:
    when = format_datetime(datetime.now(UTC) + timedelta(seconds=30), usegmt=True)
    rec = Recorder(httpx.Response(503, headers={"retry-after": when}, json={}), ok())
    client(sdk, rec).get("/x", cast_to=object)
    assert 25 < sleeps[0] <= 30


@pytest.mark.parametrize(("status", "header", "calls"), [(500, "false", 1), (400, "true", 2)])
def test_x_should_retry_overrides_status(
    sdk: Any, sleeps: list[float], status: int, header: str, calls: int
) -> None:
    rec = Recorder(httpx.Response(status, headers={"x-should-retry": header}, json={}), ok())
    with contextlib.suppress(sdk.APIStatusError):
        client(sdk, rec).get("/x", cast_to=object)
    assert len(rec.requests) == calls
    assert len(sleeps) == calls - 1


def test_idempotency_key_is_stable_across_retries(sdk: Any, sleeps: list[float]) -> None:
    rec = Recorder(httpx.Response(500, json={}), httpx.Response(500, json={}), ok())
    client(sdk, rec).post("/x", cast_to=object, body={"a": 1})
    keys = {r.headers["idempotency-key"] for r in rec.requests}
    assert len(rec.requests) == 3
    assert len(keys) == 1
    assert next(iter(keys)).startswith("stainless-python-retry-")
    assert len(sleeps) == 2


def test_get_requests_have_no_idempotency_key(sdk: Any, sleeps: list[float]) -> None:
    rec = Recorder(ok())
    client(sdk, rec).get("/x", cast_to=object)
    assert "idempotency-key" not in rec.requests[0].headers
    assert sleeps == []


def test_timeout_is_sent_and_overridable(sdk: Any) -> None:
    rec = Recorder(ok())
    c = client(sdk, rec)
    c.get("/x", cast_to=object)
    c.get("/x", cast_to=object, options={"timeout": 3.0})
    client(sdk, rec, timeout=7.0).get("/x", cast_to=object)
    timeouts = [r.extensions["timeout"] for r in rec.requests]
    assert timeouts[0] == httpx.Timeout(timeout=60, connect=5.0).as_dict()
    assert timeouts[1] == httpx.Timeout(3.0).as_dict()
    assert timeouts[2] == httpx.Timeout(7.0).as_dict()


def test_timeouts_are_retried_then_raise(sdk: Any, sleeps: list[float]) -> None:
    rec = Recorder(httpx.ReadTimeout("slow"))
    with pytest.raises(sdk.APITimeoutError):
        client(sdk, rec).get("/x", cast_to=object)
    assert len(rec.requests) == 3
    assert sleeps == [0.5, 1.0]


def test_timeout_then_success(sdk: Any, sleeps: list[float]) -> None:
    rec = Recorder(httpx.ConnectTimeout("slow"), ok())
    assert client(sdk, rec).get("/x", cast_to=object) == {"ok": True}
    assert len(sleeps) == 1


def test_connection_errors_are_retried_then_raise(sdk: Any, sleeps: list[float]) -> None:
    rec = Recorder(httpx.ConnectError("refused"))
    with pytest.raises(sdk.APIConnectionError):
        client(sdk, rec).get("/x", cast_to=object)
    assert len(rec.requests) == 3
    assert len(sleeps) == 2


@pytest.mark.anyio
async def test_async_retries_backoff_and_retry_after(sdk: Any, sleeps: list[float]) -> None:
    rec = Recorder(
        httpx.Response(500, json={}),
        httpx.Response(429, headers={"retry-after": "3"}, json={}),
        ok(),
    )
    c = sdk.AsyncFer(http_client=httpx.AsyncClient(transport=httpx.MockTransport(rec)))
    assert await c.get("/x", cast_to=object) == {"ok": True}
    assert sleeps == [0.5, 3.0]


@pytest.mark.anyio
async def test_async_max_retries_and_timeout(sdk: Any, sleeps: list[float]) -> None:
    rec = Recorder(httpx.ReadTimeout("slow"))
    c = sdk.AsyncFer(http_client=httpx.AsyncClient(transport=httpx.MockTransport(rec)))
    with pytest.raises(sdk.APITimeoutError):
        await c.get("/x", cast_to=object)
    assert len(rec.requests) == 3
    assert sleeps == [0.5, 1.0]
