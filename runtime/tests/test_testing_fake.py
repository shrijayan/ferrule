# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``<pkg>.testing``: the fake server, rendered into a throwaway SDK and driven like SDK code."""
# ruff: noqa: ANN401 - the rendered SDK is imported at test time, so it is typed as Any.

from __future__ import annotations

import importlib
import json
import pkgutil
import sys
from typing import TYPE_CHECKING, Any

import httpx
import pytest
from conftest import CLIENT, INIT, SCHEMES

from ferrule_runtime import render_files

if TYPE_CHECKING:
    from collections.abc import Iterator

PACKAGE = "ferfake"

USER = {
    "type": "object",
    "required": ["id", "name", "created_at", "tags", "role", "active", "score", "parent"],
    "properties": {
        "id": {"type": "string", "example": "usr_1"},
        "name": {"type": "string"},
        "created_at": {"type": "string", "format": "date-time"},
        "tags": {"type": "array", "items": {"type": "string", "enum": ["a", "b"]}},
        "role": {"$ref": "#/schemas/Role"},
        "active": {"type": "boolean"},
        "score": {"type": ["number", "null"], "minimum": 1},
        "parent": {"$ref": "#/schemas/User"},
        "nickname": {"type": "string"},
        "plan": {"type": "string", "default": "free"},
    },
}
ROUTES = {
    "version": 1,
    "client_options": {"custom_headers": {"X-Fake": "1"}},
    "schemas": {
        "User": USER,
        "Role": {
            "allOf": [
                {"type": "object", "required": ["n"], "properties": {"n": {"type": "integer"}}}
            ]
        },
    },
    "routes": [
        {"method": "get", "path": "/users/{user_id}", "response": {"$ref": "#/schemas/User"}},
        {
            "method": "get",
            "path": "/users/me",
            "response": {
                "type": "object",
                "required": ["me"],
                "properties": {"me": {"const": True}},
            },
        },
        {
            "method": "get",
            "path": "/users",
            "response": {
                "type": "object",
                "required": ["data"],
                "properties": {
                    "data": {"type": "array", "items": {"$ref": "#/schemas/User"}},
                    "next_cursor": {"type": "string"},
                },
            },
        },
        {"method": "post", "path": "/users", "status": 201, "response": {"$ref": "#/schemas/User"}},
        {"method": "delete", "path": "/users/{user_id}", "status": 204},
        {
            "method": "get",
            "path": "/health",
            "response": {"type": "string"},
            "content_type": "text/plain",
        },
        {
            "method": "get",
            "path": "/broken",
            "status": 500,
            "response": {
                "type": "object",
                "required": ["error"],
                "properties": {"error": {"type": "string"}},
            },
        },
    ],
}
MODELS = """\
from typing import List, Optional

from ._models import BaseModel


class User(BaseModel):
    id: str
    name: str
    tags: List[str]
    plan: Optional[str] = None
    nickname: Optional[str] = None
"""


@pytest.fixture(scope="module")
def sdk(tmp_path_factory: pytest.TempPathFactory) -> Iterator[Any]:
    root = tmp_path_factory.mktemp("fake")
    pkg = root / PACKAGE
    files = render_files(PACKAGE, "Fer", pagination=SCHEMES, testing=True)
    files |= {
        "_client.py": CLIENT,
        "__init__.py": INIT,
        "_version.py": '__version__ = "0.0.1"\n',
        "models.py": MODELS,
        "testing/_routes.json": json.dumps(ROUTES),
    }
    for rel, text in files.items():
        path = pkg / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    sys.path.insert(0, str(root))
    try:
        module = importlib.import_module(PACKAGE)
        for sub in ("testing", "models", "pagination"):
            importlib.import_module(f"{PACKAGE}.{sub}")
        yield module
    finally:
        sys.path.remove(str(root))
        for name in [m for m in sys.modules if m == PACKAGE or m.startswith(f"{PACKAGE}.")]:
            del sys.modules[name]


EXPECTED_USER = {
    "id": "usr_1",
    "name": "string",
    "created_at": "1970-01-01T00:00:00Z",
    "tags": ["a"],
    "role": {"n": 0},
    "active": False,
    "score": 1.0,
    "plan": "free",
}


def test_testing_is_opt_in() -> None:
    assert not any(rel.startswith("testing/") for rel in render_files("acme", "Acme"))
    files = render_files("acme", "Acme", testing=True)
    assert set(files) - set(render_files("acme", "Acme")) == {
        "testing/__init__.py",
        "testing/_fake.py",
    }
    assert "from .._client import Acme, AsyncAcme" in files["testing/_fake.py"]
    assert files == render_files("acme", "Acme", testing=True)


def test_sync_calls(sdk: Any) -> None:
    user_cls = sdk.models.User
    with sdk.testing.fake() as client:
        user = client.get("/users/u1", cast_to=user_cls)
        assert (user.id, user.name, user.tags, user.plan) == ("usr_1", "string", ["a"], "free")
        assert client.get("/users/me", cast_to=object) == {"me": True}

        page = client.get_api_list(
            "/users", model=user_cls, page=sdk.pagination.SyncCursor[user_cls]
        )
        assert [u.id for u in page] == ["usr_1"]

        created = client.post("/users", body={"name": "x"}, cast_to=object)
        assert created == EXPECTED_USER
        assert client.delete("/users/u1", cast_to=httpx.Response).status_code == 204
        assert client.get("/health", cast_to=str) == "string"


def test_errors(sdk: Any) -> None:
    with sdk.testing.fake() as client:
        with pytest.raises(sdk.InternalServerError) as broken:
            client.get("/broken", cast_to=object)
        assert broken.value.body == {"error": "string"}
        with pytest.raises(sdk.NotFoundError, match="no fake route for GET /nope"):
            client.get("/nope", cast_to=object)


def test_overrides(sdk: Any) -> None:
    def user(request: httpx.Request) -> dict[str, object]:
        return {"id": request.url.path.rsplit("/", 1)[-1], "name": "n", "tags": []}

    overrides: dict[str, object] = {
        "GET /users/{user_id}": user,
        "POST /users": httpx.Response(409, json={"error": "taken"}),
        "GET /extra": [1, 2],
    }
    with sdk.testing.fake(overrides=overrides) as client:
        assert client.get("/users/u7", cast_to=sdk.models.User).id == "u7"
        assert client.get("/extra", cast_to=object) == [1, 2]
        with pytest.raises(sdk.ConflictError):
            client.post("/users", body={}, cast_to=object)


def test_http_client_and_base_url_prefix(sdk: Any) -> None:
    client = sdk.Fer(base_url="https://api.example.com/v1", http_client=sdk.testing.http_client())
    assert client.get("/users/u1", cast_to=object) == EXPECTED_USER
    client.close()

    transport = sdk.testing.FakeTransport()
    client = sdk.Fer(http_client=httpx.Client(transport=transport))
    client.post("/users", body={"name": "x"}, cast_to=object)
    assert json.loads(transport.requests[0].content) == {"name": "x"}
    client.close()


def test_client_options_from_routes_file(sdk: Any) -> None:
    def echo(request: httpx.Request) -> dict[str, str]:
        return dict(request.headers)

    with sdk.testing.fake(overrides={"GET /echo": echo}) as client:
        assert client.get("/echo", cast_to=object)["x-fake"] == "1"


def test_custom_routes(sdk: Any) -> None:
    routes = [sdk.testing.Route("GET", "/x/{a}", response={"type": "integer", "minimum": 3})]
    assert repr(routes[0]) == "Route('GET /x/{a}')"
    with sdk.testing.fake(routes=routes) as client:
        assert client.get("/x/1", cast_to=object) == 3
        with pytest.raises(sdk.NotFoundError):
            client.get("/users/u1", cast_to=object)


def test_example_defaults(sdk: Any) -> None:
    example = sdk.testing.example
    assert example(None) is None
    assert example({"$ref": "#/schemas/User"}) == EXPECTED_USER
    assert example({"examples": [5]}) == 5
    assert example({"oneOf": [{"type": "integer"}, {"type": "string"}]}) == 0
    assert (
        example({"anyOf": [{"format": "uuid", "type": "string"}]})
        == "00000000-0000-0000-0000-000000000000"
    )
    assert example({"type": "string", "minLength": 3}) == "string"
    assert example({"type": "string", "format": "date", "minLength": 12}) == "1970-01-01xx"
    assert example({"items": {"type": "boolean"}, "minItems": 2}) == [False, False]
    assert example({"properties": {}}) == {}
    assert example({"type": "null"}) is None
    assert example({"allOf": [{"type": "integer"}]}) == 0
    cyclic = {"Node": {"type": "array", "items": {"$ref": "#/schemas/Node"}}}
    assert example({"$ref": "#/schemas/Node"}, cyclic) == []
    assert example({"$ref": "#/schemas/Node"}, {"Node": {"$ref": "#/schemas/Node"}}) is None
    assert example({"allOf": [{"$ref": "#/schemas/A"}]}, {"A": {"$ref": "#/schemas/A"}}) == {}


def test_deterministic(sdk: Any) -> None:
    with sdk.testing.fake() as one, sdk.testing.fake() as two:
        a = one.get("/users", cast_to=httpx.Response).content
        b = two.get("/users", cast_to=httpx.Response).content
    assert a == b
    assert sdk.testing.load_routes()[0].key == "GET /users/{user_id}"


@pytest.mark.anyio
async def test_async_calls(sdk: Any) -> None:
    async with sdk.testing.async_fake() as client:
        user = await client.get("/users/u1", cast_to=sdk.models.User)
        assert user.id == "usr_1"
        page = client.get_api_list(
            "/users", model=sdk.models.User, page=sdk.pagination.AsyncCursor[sdk.models.User]
        )
        assert [u.id async for u in page] == ["usr_1"]
        assert await client.post("/users", body={"name": "x"}, cast_to=object) == EXPECTED_USER
        with pytest.raises(sdk.NotFoundError):
            await client.get("/nope", cast_to=object)


def test_missing_routes_file(sdk: Any, monkeypatch: pytest.MonkeyPatch) -> None:
    def missing(_package: str, resource: str) -> bytes:
        raise FileNotFoundError(resource)

    monkeypatch.setattr(pkgutil, "get_data", missing)
    assert sdk.testing.load_routes() == []

    def empty(_package: str, _resource: str) -> None:
        return None

    monkeypatch.setattr(pkgutil, "get_data", empty)
    assert sdk.testing.load_routes() == []
