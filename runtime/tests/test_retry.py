# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
from ferrule_runtime import retry_delay, should_retry
from hypothesis import given
from hypothesis import strategies as st


def test_should_retry() -> None:
    for s in (408, 409, 429, 500, 503, 599):
        assert should_retry(s)
    for s in (200, 400, 401, 404, 422):
        assert not should_retry(s)


def test_exponential_backoff() -> None:
    assert retry_delay(0) == 0.5
    assert retry_delay(1) == 1.0
    assert retry_delay(4) == 8.0
    assert retry_delay(10) == 8.0
    assert retry_delay(-3) == 0.5
    assert retry_delay(1, jitter=0.25) == 0.75


def test_retry_after_headers() -> None:
    assert retry_delay(0, {"Retry-After-Ms": "1500"}) == 1.5
    assert retry_delay(0, {"retry-after": "3"}) == 3.0
    assert retry_delay(0, {"retry-after": "120"}) == 0.5
    assert retry_delay(0, {"retry-after-ms": "bad", "retry-after": "2"}) == 2.0
    assert retry_delay(0, {}) == 0.5
    assert retry_delay(0, {"x": "y"}) == 0.5
    assert retry_delay(0, {"retry-after": "soon"}) == 0.5


def test_retry_after_http_date() -> None:
    hdr = {"retry-after": "Wed, 21 Oct 2015 07:28:10 GMT"}
    now = 1445412480.0  # 07:28:00 GMT
    assert retry_delay(0, hdr, now=now) == 10.0
    assert retry_delay(0, {"retry-after": "garbage"}, now=now) == 0.5


@given(st.integers(min_value=0, max_value=100), st.floats(min_value=-1, max_value=2))
def test_delay_bounds(attempt: int, jitter: float) -> None:
    assert 0.375 <= retry_delay(attempt, jitter=jitter) <= 8.0
