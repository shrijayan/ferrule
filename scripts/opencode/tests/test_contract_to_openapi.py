# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
from __future__ import annotations

import copy
from typing import TYPE_CHECKING, Any, cast

import pytest

from scripts.opencode.contract_to_openapi import (
    ContractError,
    convert,
    convert_path,
    dumps,
    main,
    rewrite_refs,
)

if TYPE_CHECKING:
    from pathlib import Path

ERROR = {"status": 404, "tag": "NotFound", "schema": {"$ref": "#/$defs/NotFound"}}


def _contract() -> dict[str, Any]:
    return {
        "generator": "test",
        "opencodeVersion": "9.9.9",
        "effectVersion": "1.0.0",
        "groups": [
            {
                "name": "session",
                "endpoints": [
                    {
                        "clientPath": ["get"],
                        "operationId": "session.get",
                        "method": "GET",
                        "path": "/api/session/:sessionID",
                        "summary": "Get session",
                        "deprecated": False,
                        "input": [
                            {
                                "name": "sessionID",
                                "source": "params",
                                "optional": False,
                                "schema": {"type": "string"},
                            },
                            {
                                "name": "location",
                                "source": "query",
                                "optional": True,
                                "schema": {"anyOf": [{"type": "object"}, {"type": "null"}]},
                            },
                        ],
                        "success": {
                            "kind": "value",
                            "status": 200,
                            "unwrapData": False,
                            "schema": {"$ref": "#/$defs/Session"},
                        },
                        "errors": [ERROR, {**ERROR, "tag": "Gone"}],
                    },
                    {
                        "clientPath": ["prompt"],
                        "operationId": "session.prompt",
                        "method": "POST",
                        "path": "/api/session/:sessionID",
                        "deprecated": True,
                        "input": [
                            {
                                "name": "text",
                                "source": "payload",
                                "optional": False,
                                "schema": {"type": "string"},
                            },
                            {
                                "name": "x-ticket",
                                "source": "headers",
                                "optional": True,
                                "schema": {"type": "string"},
                            },
                        ],
                        "payload": {"mode": "struct"},
                        "success": {"kind": "void", "status": 204},
                        "errors": [],
                    },
                ],
            },
            {
                "name": "fs",
                "endpoints": [
                    {
                        "clientPath": ["read"],
                        "operationId": "fs.read",
                        "method": "GET",
                        "path": "/api/fs/read/*",
                        "deprecated": False,
                        "input": [
                            {
                                "name": "path",
                                "source": "wildcard",
                                "optional": False,
                                "schema": {"type": "string"},
                            },
                        ],
                        "success": {"kind": "binary", "status": 200},
                        "errors": [],
                    },
                    {
                        "clientPath": ["events"],
                        "operationId": "fs.events",
                        "method": "PUT",
                        "path": "/api/fs/events",
                        "deprecated": False,
                        "input": [
                            {
                                "name": "payload",
                                "source": "payload",
                                "optional": False,
                                "schema": {"type": "string"},
                            },
                        ],
                        "payload": {"mode": "binary"},
                        "success": {
                            "kind": "stream",
                            "status": 200,
                            "schema": {"$ref": "#/$defs/Event"},
                        },
                        "errors": [],
                    },
                ],
            },
        ],
        "definitions": {
            "Session": {"type": "object", "properties": {"e": {"$ref": "#/$defs/Event"}}},
            "Event": {"$schema": "x", "type": "string"},
            "NotFound": {"type": "object"},
        },
    }


def test_convert_path() -> None:
    assert convert_path("/api/:a/x/:b") == "/api/{a}/x/{b}"
    assert convert_path("/api/fs/read/*") == "/api/fs/read/{path}"


def test_rewrite_refs_drops_schema_and_moves_refs() -> None:
    assert rewrite_refs({"$schema": "x", "items": [{"$ref": "#/$defs/A"}]}) == {
        "items": [{"$ref": "#/components/schemas/A"}],
    }


def test_fold_record_merges_struct_and_record() -> None:
    record = {"type": "object", "additionalProperties": {"type": "string"}}
    schema: dict[str, object] = {"type": "object", "properties": {"a": {}}, "allOf": [record]}
    assert rewrite_refs(schema) == {
        "type": "object",
        "properties": {"a": {}},
        "additionalProperties": {"type": "string"},
    }
    kept = {"type": "object", "allOf": [record, record]}
    assert rewrite_refs(kept) == kept


def test_fold_non_empty_array_drops_prefix_items() -> None:
    # Effect's NonEmptyArray (`Form.Fields`): its first item repeats `items` in `prefixItems`.
    # Forge does not read `prefixItems` and made the array an empty object.
    item = {"$ref": "#/$defs/A"}
    schema = {"type": "array", "prefixItems": [item], "minItems": 1, "items": item}
    assert rewrite_refs(schema) == {
        "type": "array",
        "minItems": 1,
        "items": {"$ref": "#/components/schemas/A"},
    }
    tuple_ = {"type": "array", "prefixItems": [item, {"type": "string"}], "items": item}
    assert "prefixItems" in cast("dict[str, object]", rewrite_refs(tuple_))


def test_document_shape() -> None:
    doc = convert(_contract())
    assert doc["openapi"] == "3.1.0"
    assert doc["info"]["version"] == "9.9.9"
    assert [t["name"] for t in doc["tags"]] == ["session", "fs"]
    assert list(doc["paths"]) == [
        "/api/fs/events",
        "/api/fs/read/{path}",
        "/api/session/{sessionID}",
    ]
    assert list(doc["components"]["schemas"]) == ["Event", "NotFound", "Session"]
    assert doc["components"]["schemas"]["Event"] == {"type": "string"}


def test_value_operation() -> None:
    op = convert(_contract())["paths"]["/api/session/{sessionID}"]["get"]
    assert op["tags"] == ["session"]
    assert op["x-opencode-client-path"] == ["get"]
    path_param, query_param = op["parameters"]
    assert path_param == {
        "name": "sessionID",
        "in": "path",
        "required": True,
        "schema": {"type": "string"},
    }
    assert query_param["style"] == "form"
    assert query_param["required"] is False
    ok = op["responses"]["200"]["content"]["application/json"]["schema"]
    assert ok == {"$ref": "#/components/schemas/Session"}
    not_found = op["responses"]["404"]
    assert not_found["description"] == "NotFound, Gone"
    assert len(not_found["content"]["application/json"]["schema"]["anyOf"]) == 2


def test_struct_payload_and_void() -> None:
    op = convert(_contract())["paths"]["/api/session/{sessionID}"]["post"]
    assert op["deprecated"] is True
    assert op["parameters"][0]["in"] == "header"
    schema = op["requestBody"]["content"]["application/json"]["schema"]
    assert schema == {
        "type": "object",
        "properties": {"text": {"type": "string"}},
        "required": ["text"],
    }
    assert op["responses"] == {"204": {"description": "Success"}}


def test_binary_and_stream() -> None:
    paths = convert(_contract())["paths"]
    read = paths["/api/fs/read/{path}"]["get"]
    assert read["parameters"][0]["in"] == "path"
    assert "application/octet-stream" in read["responses"]["200"]["content"]
    events = paths["/api/fs/events"]["put"]
    # A binary payload is bytes (`format: binary`), as generators read raw request content.
    assert events["requestBody"]["content"]["application/octet-stream"]["schema"] == {
        "type": "string",
        "format": "binary",
    }
    assert "text/event-stream" in events["responses"]["200"]["content"]


def test_deterministic_and_pure() -> None:
    contract = _contract()
    before = copy.deepcopy(contract)
    assert dumps(convert(contract)) == dumps(convert(_contract()))
    assert contract == before


@pytest.mark.parametrize(
    ("path", "value"),
    [
        (("input", 0, "source"), "cookie"),
        (("success", "kind"), "page"),
        (("payload",), {"mode": "multipart"}),
    ],
)
def test_unknown_shapes_raise(path: tuple[Any, ...], value: object) -> None:
    contract = _contract()
    target: Any = contract["groups"][0]["endpoints"][1]
    for key in path[:-1]:
        target = target[key]
    target[path[-1]] = value
    with pytest.raises(ContractError):
        convert(contract)


def test_duplicate_operation_raises() -> None:
    contract = _contract()
    endpoints = contract["groups"][0]["endpoints"]
    endpoints.append(copy.deepcopy(endpoints[0]))
    with pytest.raises(ContractError, match="duplicate"):
        convert(contract)


def test_main_writes_file(tmp_path: Path) -> None:
    src = tmp_path / "contract.json"
    out = tmp_path / "openapi.json"
    src.write_text(dumps(_contract()), encoding="utf-8")
    assert main([str(src), str(out)]) == 0
    assert out.read_text(encoding="utf-8") == dumps(convert(_contract()))


def test_main_reports_bad_contract(tmp_path: Path) -> None:
    src = tmp_path / "contract.json"
    src.write_text("{}", encoding="utf-8")
    assert main([str(src), str(tmp_path / "out.json")]) == 1
