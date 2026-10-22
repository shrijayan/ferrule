# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the runtime into a throwaway package (``fersdk``) and import it for behaviour tests.

Only our own rendered copy is imported; oracle checkouts are never executed.
"""

from __future__ import annotations

import importlib
import sys
from typing import TYPE_CHECKING, Any

import pytest

from ferrule_runtime import PageField, PageModel, PageScheme, render_files

if TYPE_CHECKING:
    from collections.abc import Iterator

PACKAGE = "fersdk"
ITEMS = "List[_T]"

SCHEMES = (
    PageScheme(
        name="Cursor",
        kind="cursor",
        items="data",
        fields=(PageField("data", ITEMS), PageField("next_cursor", "Optional[str]", optional=True)),
        param="cursor",
        path=("next_cursor",),
    ),
    PageScheme(
        name="NestedCursor",
        kind="cursor",
        items="data",
        fields=(
            PageField("data", ITEMS),
            PageField("page_info", "Optional[NestedCursorPageInfo]", optional=True),
        ),
        models=(
            PageModel(
                "NestedCursorPageInfo", (PageField("after", "Optional[str]", optional=True),)
            ),
        ),
        param="after",
        path=("page_info", "after"),
    ),
    PageScheme(
        name="CursorIDPage",
        kind="cursor_id",
        items="data",
        fields=(PageField("data", ITEMS), PageField("has_more", "bool")),
        has_more="has_more",
        param="starting_after",
        before_param="ending_before",
        item_id="id",
    ),
    PageScheme(
        name="CursorURLPage",
        kind="cursor_url",
        items="data",
        fields=(PageField("data", ITEMS), PageField("next", "Optional[str]", optional=True)),
        path=("next",),
    ),
    PageScheme(
        name="OffsetPage",
        kind="offset",
        items="items",
        fields=(PageField("items", ITEMS), PageField("total", "Optional[int]", optional=True)),
        param="offset",
        total_path=("total",),
    ),
    PageScheme(
        name="OpenOffsetPage",
        kind="offset",
        items="items",
        fields=(PageField("items", ITEMS),),
        param="offset",
    ),
    PageScheme(
        name="PageNumberPage",
        kind="page_number",
        items="items",
        fields=(
            PageField("items", ITEMS),
            PageField("page", "Optional[int]", optional=True),
            PageField("pages", "Optional[int]", optional=True),
        ),
        param="page",
        path=("page",),
        total_path=("pages",),
    ),
    PageScheme(
        name="FakePage",
        kind="fake_page",
        items="data",
        fields=(PageField("data", ITEMS), PageField("has_more", "Optional[bool]", optional=True)),
        has_more="has_more",
    ),
)

# A hand-written stand-in for the generated _client.py / __init__.py / _version.py.
CLIENT = """\
from __future__ import annotations

from typing import Any, Mapping

import httpx

from . import _exceptions
from ._version import __version__
from ._exceptions import APIStatusError
from ._base_client import DEFAULT_MAX_RETRIES, SyncAPIClient, AsyncAPIClient

_STATUS = {
    400: _exceptions.BadRequestError,
    401: _exceptions.AuthenticationError,
    403: _exceptions.PermissionDeniedError,
    404: _exceptions.NotFoundError,
    409: _exceptions.ConflictError,
    422: _exceptions.UnprocessableEntityError,
    429: _exceptions.RateLimitError,
}


def _status_error(err_msg: str, *, body: object, response: httpx.Response) -> APIStatusError:
    cls = _STATUS.get(response.status_code)
    if cls is None and response.status_code >= 500:
        cls = _exceptions.InternalServerError
    return (cls or APIStatusError)(err_msg, response=response, body=body)


def _kwargs(base_url: str, max_retries: int, timeout: Any, kwargs: dict[str, Any]) -> Any:
    if timeout is not None:
        kwargs["timeout"] = timeout
    return dict(
        version=__version__,
        base_url=base_url,
        max_retries=max_retries,
        _strict_response_validation=False,
        **kwargs,
    )


class Fer(SyncAPIClient):
    def __init__(
        self,
        *,
        base_url: str = "http://fer.test",
        max_retries: int = DEFAULT_MAX_RETRIES,
        timeout: Any = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(**_kwargs(base_url, max_retries, timeout, kwargs))
        self._idempotency_header = "Idempotency-Key"

    def _make_status_error(self, err_msg: str, *, body: object, response: httpx.Response) -> Any:
        return _status_error(err_msg, body=body, response=response)


class AsyncFer(AsyncAPIClient):
    def __init__(
        self,
        *,
        base_url: str = "http://fer.test",
        max_retries: int = DEFAULT_MAX_RETRIES,
        timeout: Any = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(**_kwargs(base_url, max_retries, timeout, kwargs))
        self._idempotency_header = "Idempotency-Key"

    def _make_status_error(self, err_msg: str, *, body: object, response: httpx.Response) -> Any:
        return _status_error(err_msg, body=body, response=response)


Client = Fer
AsyncClient = AsyncFer
"""

INIT = """\
from . import _exceptions
from ._types import NOT_GIVEN, Omit, NotGiven, omit, not_given
from ._client import Fer, AsyncFer
from ._models import BaseModel
from ._exceptions import (
    APIError,
    FerError,
    ConflictError,
    NotFoundError,
    APIStatusError,
    RateLimitError,
    APITimeoutError,
    BadRequestError,
    APIConnectionError,
    AuthenticationError,
    InternalServerError,
    PermissionDeniedError,
    UnprocessableEntityError,
    APIResponseValidationError,
)
from ._base_client import DEFAULT_TIMEOUT, DEFAULT_MAX_RETRIES
"""


@pytest.fixture(scope="session")
def sdk(tmp_path_factory: pytest.TempPathFactory) -> Iterator[Any]:
    root = tmp_path_factory.mktemp("rendered")
    pkg = root / PACKAGE
    files = render_files(PACKAGE, "Fer", pagination=SCHEMES)
    files |= {"_client.py": CLIENT, "__init__.py": INIT, "_version.py": '__version__ = "0.0.1"\n'}
    for rel, text in files.items():
        path = pkg / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    sys.path.insert(0, str(root))
    try:
        module = importlib.import_module(PACKAGE)
        importlib.import_module(f"{PACKAGE}.pagination")
        importlib.import_module(f"{PACKAGE}._streaming")
        yield module
    finally:
        sys.path.remove(str(root))
        for name in [m for m in sys.modules if m == PACKAGE or m.startswith(f"{PACKAGE}.")]:
            del sys.modules[name]


@pytest.fixture
def anyio_backend() -> str:
    return "asyncio"
