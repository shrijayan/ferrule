# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from scripts.corpus.harness import (
    Operation,
    Refusal,
    add_model,
    api_endpoints,
    base_url,
    count_external_refs,
    count_schemas,
    drop_endpoint,
    endpoint_key,
    failed_at,
    fer_codes,
    identifier,
    method_of,
    operations,
    package_name,
    reached,
    recursive_schema,
    refusal,
    resource_of,
    snake,
    spec_format,
    summarize,
    synthesize,
    top_causes,
)
from scripts.corpus.run import endpoints_of

if TYPE_CHECKING:
    from pathlib import Path

SPEC: dict[str, object] = {
    "servers": [{"url": "/relative"}, {"url": "https://api.acme.test/v1"}],
    "paths": {
        "/users/{id}": {
            "get": {"operationId": "getUser", "tags": ["Users"]},
            "delete": {"operationId": "getUser", "tags": ["Users"]},
        },
        "/users": {"post": {"operationId": "CreateUser", "tags": ["Users"]}},
        "/{org}/billing/invoices": {"get": {}},
        "/": {"get": {"operationId": "class"}},
        "/x": {"parameters": [], "summary": "no operations"},
    },
}


@pytest.mark.parametrize(
    ("text", "expected"),
    [
        ("listUsers", "list_users"),
        ("List-Users", "list_users"),
        ("HTTPServer get", "http_server_get"),
        ("repos/list-for-org", "repos_list_for_org"),
        ("__", ""),
    ],
)
def test_snake(text: str, expected: str) -> None:
    assert snake(text) == expected


def test_identifier_fixes_digits_keywords_and_empty() -> None:
    assert identifier("2fa", "r") == "r_2fa"
    assert identifier("import", "m") == "import_"
    assert identifier("match", "m") == "match_"
    assert identifier("--", "r") == "r"


def test_package_name() -> None:
    assert package_name("azure-storage") == "corpus_azure_storage"


def test_base_url_prefers_absolute_server() -> None:
    assert base_url(SPEC, "acme") == "https://api.acme.test/v1"


def test_base_url_swagger2_and_fallback() -> None:
    swagger = {"host": "k8s.test", "basePath": "/api", "schemes": ["http"]}
    assert base_url(swagger, "k") == "http://k8s.test/api"
    assert base_url({"host": "h.test"}, "k") == "https://h.test"
    assert base_url({}, "slack") == "https://api.slack.test"


def test_operations_are_sorted_and_skip_non_verbs() -> None:
    assert [op.endpoint for op in operations(SPEC)] == [
        "get /",
        "post /users",
        "get /users/{id}",
        "delete /users/{id}",
        "get /{org}/billing/invoices",
    ]


def test_resource_and_method_fallbacks() -> None:
    op = Operation("get", "/{org}/billing/invoices", None, None)
    assert resource_of(op) == "billing"
    assert method_of(op) == "get_billing_invoices"
    assert resource_of(Operation("get", "/{id}", None, None)) == "root"
    assert resource_of(Operation("get", "/a", None, "Users & Groups")) == "users_groups"


def test_synthesize() -> None:
    result = synthesize(SPEC, "acme")
    assert result.config == {
        "organization": {"name": "acme"},
        "environments": {"production": "https://api.acme.test/v1"},
        "targets": {"python": {"package_name": "corpus_acme", "publish": {"pypi": False}}},
        "readme": {
            "example_requests": {
                "default": {"type": "request", "endpoint": "get /", "params": {}},
                "headline": {"type": "request", "endpoint": "get /", "params": {}},
            }
        },
        "resources": {
            "billing": {"methods": {"get_billing_invoices": "get /{org}/billing/invoices"}},
            "root": {"methods": {"class_": "get /"}},
            "users": {
                "methods": {
                    "create_user": "post /users",
                    "get_user": "get /users/{id}",
                    "get_user_2": "delete /users/{id}",
                }
            },
        },
    }
    assert (result.operations, result.resources, result.methods) == (5, 3, 5)
    assert result.renamed == ["delete /users/{id} -> get_user_2"]


def test_synthesize_is_deterministic() -> None:
    assert synthesize(SPEC, "acme") == synthesize(dict(reversed(SPEC.items())), "acme")


def test_synthesize_empty_spec() -> None:
    result = synthesize({}, "empty")
    assert result.config["resources"] == {}
    assert result.operations == 0


def test_fer_codes() -> None:
    assert fer_codes(["error FER1001 x", "FER1001 FER0002 FER201 FER12345 XFER1003"]) == {
        "FER0002": 1,
        "FER1001": 2,
        "FER201": 1,
    }


def report(spec: str, stages: dict[str, bool], codes: dict[str, int]) -> dict[str, object]:
    return {
        "spec": spec,
        "stages": {k: {"ok": v} for k, v in stages.items()},
        "fer_codes": codes,
        "counts": {"operations": 3},
        "seconds": 1.5,
    }


PASSED: dict[str, bool] = dict.fromkeys(
    (
        "fetch",
        "convert",
        "generate",
        "render",
        "deps",
        "ruff",
        "pyright",
        "mypy",
        "import",
        "tests",
        "determinism",
        "wheel",
    ),
    True,
)


def test_reached_and_failed_at() -> None:
    r = report("a", {"fetch": True, "convert": False}, {})
    assert (reached(r), failed_at(r)) == ("fetch", "convert")
    r = report("b", PASSED, {})
    assert (reached(r), failed_at(r)) == ("wheel", None)
    assert reached(report("c", {"fetch": False}, {})) == "none"


def test_summarize() -> None:
    reports = [
        report("zeta", {"fetch": True, "convert": False}, {"FER1001": 3, "FER1111": 1}),
        report("alpha", PASSED, {}),
        report("mid", {**PASSED, "tests": False}, {"FER1111": 1}),
    ]
    assert summarize(reports) == (
        "| spec | reached | failed at | ops | generated | skipped | files | FER codes | seconds |\n"
        "|------|---------|-----------|-----|-----------|---------|-------|-----------|---------|\n"
        "| alpha | wheel | - | 3 | - | - | - | - | 1.5 |\n"
        "| mid | import | tests | 3 | - | - | - | FER1111x1 | 1.5 |\n"
        "| zeta | fetch | convert | 3 | - | - | - | FER1001x3, FER1111x1 | 1.5 |\n"
        "\n"
        "Compile gate (ruff, pyright, mypy): 2 of 3 specs pass.\n"
        "\n"
        "| FER code | specs | occurrences |\n"
        "|----------|-------|-------------|\n"
        "| FER1111 | 2 | 2 |\n"
        "| FER1001 | 1 | 3 |\n"
    )


def test_package_override() -> None:
    assert package_name("open-ai", "openai") == "openai"
    config = synthesize(SPEC, "acme", "acme_sdk").config
    assert config["targets"] == {"python": {"package_name": "acme_sdk", "publish": {"pypi": False}}}


def test_spec_format_and_schemas() -> None:
    assert spec_format({"swagger": "2.0"}) == "swagger 2.0"
    assert spec_format({"openapi": "3.1.0"}) == "openapi 3.1.0"
    assert spec_format({}) == "unknown"
    assert count_schemas({"components": {"schemas": {"A": {}, "B": {}}}}) == 2
    assert count_schemas({"definitions": {"A": {}}}) == 1


REFUSED = (
    "info FER1603 stainless.yml#/paths/~1a/get/parameters/0/style: dropped\n"
    "ferrule-gen: FER201 at #/paths/~1v1~1files~1{id}/delete: recursive schema file without "
    "x-ferrule-models is not generated yet. Fix: leave the operation out, or name the schema\n"
)


def test_refusal() -> None:
    assert refusal(REFUSED) == Refusal(
        "FER201",
        "#/paths/~1v1~1files~1{id}/delete",
        "delete /v1/files/{id}",
        "recursive schema file without x-ferrule-models is not generated yet",
    )
    assert refusal("ferrule-gen: FER201 at #/components/schemas/A: nope. Fix: x") is None
    assert refusal("forge: document refused") is None
    assert recursive_schema("recursive schema file without x-ferrule-models") == "file"
    assert recursive_schema("an allOf member of kind union is not generated yet") is None


def config_of() -> dict[str, object]:
    return {
        "resources": {
            "files": {"methods": {"delete": "delete /v1/files/{id}", "list": "get /v1/files"}},
            "solo": {"methods": {"get": "get /solo"}},
        }
    }


def test_drop_endpoint() -> None:
    config = config_of()
    assert drop_endpoint(config, "get /solo")
    assert drop_endpoint(config, "delete /v1/files/{id}")
    assert config == {"resources": {"files": {"methods": {"list": "get /v1/files"}}}}
    assert not drop_endpoint(config, "get /missing")


def test_add_model_once() -> None:
    config = config_of()
    assert add_model(config, "delete /v1/files/{id}", "file")
    assert not add_model(config, "get /solo", "file")
    assert not add_model(config, "get /missing", "other")
    resources = config["resources"]
    assert isinstance(resources, dict)
    assert resources["files"]["models"] == {"file": "file"}


def test_api_endpoints() -> None:
    md = (
        '- <code title="get /v1/files/{file_id}">client.files.<a href="x">retrieve</a>(id)</code>\n'
        '- <code title="post /v1/files">client.files.create()</code>\n'
    )
    assert api_endpoints(md) == {"get /v1/files/{}", "post /v1/files"}
    assert endpoint_key("get /v1/files/{id}") == "get /v1/files/{}"


def test_top_causes() -> None:
    pyright = (
        '  /x/a.py:1:7 - error: Class declaration "A" is obscured (reportRedeclaration)\n'
        '  /x/b.py:2:7 - error: Class declaration "B" is obscured (reportRedeclaration)\n'
        "  /x/c.py:3:1 - error: Import cycle\n"
        "  /x/d.py:4:1 - warning: unused (reportUnusedImport)\n"
    )
    assert top_causes(pyright, "pyright") == {"reportRedeclaration": 2, "error": 1}
    mypy = 'src/a.py:1: error: Name "A" already defined on line 2  [no-redef]\n'
    assert top_causes(mypy, "mypy") == {"no-redef": 1}
    ruff = 'src/a.py:102:9: invalid-syntax: Duplicate parameter "timeout"\nFound 1 error.\n'
    assert top_causes(ruff, "ruff") == {'invalid-syntax: Duplicate parameter "timeout"': 1}
    dup = "E   SyntaxError: duplicate argument 'timeout'\n"
    pytest_out = dup + dup + "E   assert 1 == 2\nE   ValueError: line 42\n"
    assert top_causes(pytest_out, "tests", top=1) == {
        "SyntaxError: duplicate argument 'timeout'": 2
    }
    assert top_causes(pytest_out, "tests")["ValueError: line N"] == 1


def test_count_external_refs() -> None:
    spec = {
        "paths": {"/a": {"get": {"$ref": "resources/a_get.yml"}}},
        "components": {
            "schemas": {"A": {"$ref": "#/components/schemas/B"}, "C": [{"$ref": "x.json#/D"}]}
        },
    }
    assert count_external_refs(spec) == 2


def test_endpoints_of_reads_split_api_md(tmp_path: Path) -> None:
    (tmp_path / "src/pkg/resources/a").mkdir(parents=True)
    (tmp_path / "api.md").write_text('<code title="get /x">x</code>\n')
    (tmp_path / "src/pkg/resources/a/api.md").write_text('<code title="post /a/{id}">a</code>\n')
    assert endpoints_of(tmp_path) == {"get /x", "post /a/{}"}
    assert endpoints_of(tmp_path / "missing") == set()
