# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Retry policy matching Stainless SDK semantics.

Pure functions only: jitter and the current time are injected so behaviour is
deterministic in tests.
"""

from __future__ import annotations

from email.utils import parsedate_to_datetime
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Mapping

INITIAL_RETRY_DELAY = 0.5
MAX_RETRY_DELAY = 8.0
MAX_RETRY_AFTER = 60.0
MAX_JITTER = 0.25

_RETRYABLE_STATUSES = frozenset({408, 409, 429})
_SERVER_ERROR_MIN = 500


def should_retry(status: int) -> bool:
    """Return whether an HTTP status code is retryable (408, 409, 429, 5xx)."""
    return status in _RETRYABLE_STATUSES or status >= _SERVER_ERROR_MIN


def _to_float(value: str) -> float | None:
    try:
        return float(value)
    except ValueError:
        return None


def _parse_retry_after(headers: Mapping[str, str] | None, now: float | None) -> float | None:
    if not headers:
        return None
    lowered = {k.lower(): v for k, v in headers.items()}
    ms = lowered.get("retry-after-ms")
    if ms is not None:
        parsed_ms = _to_float(ms)
        if parsed_ms is not None:
            return parsed_ms / 1000.0
    value = lowered.get("retry-after")
    if value is None:
        return None
    seconds = _to_float(value)
    if seconds is not None or now is None:
        return seconds
    try:
        when = parsedate_to_datetime(value)
    except (TypeError, ValueError):
        return None
    return when.timestamp() - now


def retry_delay(
    attempt: int,
    retry_after_header: Mapping[str, str] | None = None,
    *,
    jitter: float = 0.0,
    now: float | None = None,
) -> float:
    """Compute seconds to wait before retry number ``attempt`` (0-based).

    A server-provided ``retry-after-ms`` / ``retry-after`` value in (0, 60] wins.
    Otherwise the delay is ``min(0.5 * 2**attempt, 8)`` scaled by ``1 - jitter``,
    where ``jitter`` (clamped to [0, 0.25]) is supplied by the caller.
    ``now`` (epoch seconds) enables HTTP-date ``retry-after`` values.
    """
    server = _parse_retry_after(retry_after_header, now)
    if server is not None and 0 < server <= MAX_RETRY_AFTER:
        return server
    base = min(INITIAL_RETRY_DELAY * (2.0 ** max(attempt, 0)), MAX_RETRY_DELAY)
    return base * (1.0 - min(max(jitter, 0.0), MAX_JITTER))
