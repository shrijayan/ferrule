# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
from __future__ import annotations

import pytest

from scripts.corpus.harness import (
    Operation,
    base_url,
    failed_at,
    fer_codes,
    identifier,
    method_of,
    operations,
    package_name,
    reached,
    resource_of,
    snake,
    summarize,
    synthesize,
)

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
    assert fer_codes(["error FER1001 x", "FER1001 FER0002 FER12345 XFER1003"]) == {
        "FER0002": 1,
        "FER1001": 2,
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
    ("fetch", "convert", "forge", "plugin", "runtime", "compile", "wheel"), True
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
        report("mid", {**PASSED, "wheel": False}, {"FER1111": 1}),
    ]
    assert summarize(reports) == (
        "| spec | reached | failed at | ops | files | FER codes | seconds |\n"
        "|------|---------|-----------|-----|-------|-----------|---------|\n"
        "| alpha | wheel | - | 3 | - | - | 1.5 |\n"
        "| mid | compile | wheel | 3 | - | FER1111x1 | 1.5 |\n"
        "| zeta | fetch | convert | 3 | - | FER1001x3, FER1111x1 | 1.5 |\n"
        "\n"
        "Compile gate: 2 of 3 specs pass.\n"
        "\n"
        "| FER code | specs | occurrences |\n"
        "|----------|-------|-------------|\n"
        "| FER1111 | 2 | 2 |\n"
        "| FER1001 | 1 | 3 |\n"
    )
