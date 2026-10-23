# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule rescue``: save the OpenAPI spec a Stainless SDK's ``.stats.yml`` points to.

``.stats.yml`` names the spec (``openapi_spec_url``) and its md5 (``openapi_spec_hash``). The
parsing and checks here are pure; the only network access is :func:`https_fetch`, and
:func:`rescue` takes the fetcher as a parameter so tests never touch the network.
"""

from __future__ import annotations

import hashlib
import re
import urllib.request
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import IO, TYPE_CHECKING, override
from urllib.parse import urlsplit

from ferrule.errors import MISSING_INPUT, fail
from ferrule.generate import ensure_inside

if TYPE_CHECKING:
    from collections.abc import Callable
    from http.client import HTTPMessage


# FER41xx: rescue and adopt. Kept here so errors.py stays free of merge conflicts.
HASH_MISMATCH = "FER4101"
BAD_STATS = "FER4103"
FETCH_FAILED = "FER4104"

type Fetcher = Callable[[str, int, float], bytes]

STATS_FILE = ".stats.yml"
MAX_SPEC_BYTES = 64 * 1024 * 1024
TIMEOUT_SECONDS = 60.0
SPEC_SUFFIXES = frozenset({".yml", ".yaml", ".json"})
_LINE = re.compile(r"^(?P<key>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?P<value>.*?)\s*$")
_MD5 = re.compile(r"^[0-9a-f]{32}$")


@dataclass(frozen=True)
class Stats:
    """The fields of ``.stats.yml`` rescue needs."""

    spec_url: str
    spec_hash: str

    @property
    def suffix(self) -> str:
        """The spec's file suffix, from its URL (``.yml`` when the URL names none)."""
        suffix = PurePosixPath(urlsplit(self.spec_url).path).suffix.lower()
        return ".yaml" if suffix == ".yaml" else suffix if suffix in SPEC_SUFFIXES else ".yml"


@dataclass(frozen=True)
class Rescued:
    """A spec that matched its recorded hash."""

    stats: Stats
    data: bytes


def _unquote(value: str) -> str:
    value = value.split(" #", 1)[0].strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":  # noqa: PLR2004
        return value[1:-1]
    return value


def parse_stats(text: str) -> Stats:
    """Read ``openapi_spec_url`` and ``openapi_spec_hash`` from ``.stats.yml`` text.

    ``.stats.yml`` is a flat ``key: value`` file, so it is read line by line instead of
    pulling in a YAML parser.
    """
    fields: dict[str, str] = {}
    for line in text.splitlines():
        match = _LINE.match(line)
        if match is not None:
            fields[match["key"]] = _unquote(match["value"])
    url = fields.get("openapi_spec_url", "")
    digest = fields.get("openapi_spec_hash", "").lower()
    if not url:
        raise fail(
            BAD_STATS,
            "openapi_spec_url is missing",
            f"{STATS_FILE}#openapi_spec_url",
            "run rescue in a Stainless-generated SDK checkout",
        )
    if not _MD5.match(digest):
        raise fail(
            BAD_STATS,
            f"openapi_spec_hash {digest!r} is not an md5 hex digest",
            f"{STATS_FILE}#openapi_spec_hash",
            "restore .stats.yml from the last Stainless commit",
        )
    check_url(url)
    return Stats(url, digest)


def check_url(url: str) -> None:
    """Refuse anything but a plain ``https://`` URL without credentials."""
    parts = urlsplit(url)
    if parts.scheme != "https" or not parts.hostname or parts.username or parts.password:
        raise fail(
            BAD_STATS,
            f"refusing to download {url!r}: only https URLs are allowed",
            f"{STATS_FILE}#openapi_spec_url",
            "point openapi_spec_url at an https:// URL",
        )


def check_hash(stats: Stats, data: bytes) -> Rescued:
    """Return the spec when its md5 equals ``openapi_spec_hash``; raise FER4101 otherwise."""
    actual = hashlib.md5(data, usedforsecurity=False).hexdigest()
    if actual != stats.spec_hash:
        raise fail(
            HASH_MISMATCH,
            f"downloaded spec has md5 {actual}, .stats.yml records {stats.spec_hash}",
            f"{STATS_FILE}#openapi_spec_hash",
            "the hosted spec changed or is gone; recover it from another copy",
        )
    return Rescued(stats, data)


class _HttpsOnly(urllib.request.HTTPRedirectHandler):
    @override
    def redirect_request(
        self,
        req: urllib.request.Request,
        fp: IO[bytes],
        code: int,
        msg: str,
        headers: HTTPMessage,
        newurl: str,
    ) -> urllib.request.Request | None:
        check_url(newurl)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def https_fetch(url: str, limit: int, timeout: float) -> bytes:  # pragma: no cover - network
    """GET ``url`` over https (redirects too), reading at most ``limit`` bytes."""
    check_url(url)
    opener = urllib.request.build_opener(_HttpsOnly)
    with opener.open(urllib.request.Request(url), timeout=timeout) as resp:  # noqa: S310 - https checked
        data: bytes = resp.read(limit + 1)
    return data


def rescue(
    repo: Path,
    fetch: Fetcher = https_fetch,
    limit: int = MAX_SPEC_BYTES,
    timeout: float = TIMEOUT_SECONDS,
) -> Rescued:
    """Download and check the spec ``repo/.stats.yml`` points to; writes nothing."""
    stats_path = ensure_inside(repo, PurePosixPath(STATS_FILE))
    if not stats_path.is_file():
        raise fail(
            MISSING_INPUT,
            f"{stats_path} does not exist",
            "REPO",
            "pass the root of a Stainless-generated SDK checkout",
        )
    stats = parse_stats(stats_path.read_text(encoding="utf-8"))
    try:
        data = fetch(stats.spec_url, limit, timeout)
    except OSError as exc:
        raise fail(
            FETCH_FAILED,
            f"download failed: {exc}",
            f"{STATS_FILE}#openapi_spec_url",
            "check the network, or recover the spec from another copy",
        ) from exc
    if len(data) > limit:
        raise fail(
            FETCH_FAILED,
            f"spec is larger than {limit} bytes",
            f"{STATS_FILE}#openapi_spec_url",
            "recover the spec from another copy",
        )
    return check_hash(stats, data)


def write_spec(rescued: Rescued, out: Path) -> Path:
    """Write the checked spec to ``out``, refusing to write through a symlink."""
    if out.is_symlink():
        raise fail(MISSING_INPUT, f"{out} is a symlink", "--out", "pass a regular file path")
    out.write_bytes(rescued.data)
    return out
