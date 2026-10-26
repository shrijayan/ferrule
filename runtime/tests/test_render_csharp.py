# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Rendering the C# runtime: names, the credentials slot, using order and line endings."""

from __future__ import annotations

from typing import Any

import pytest

from ferrule_runtime import CSharpOptions, RenderError, render_csharp_files, template_root

ACME = CSharpOptions(namespace="Acme", client_name="Acme", env_prefix="ACME")
PARAMS_BASE = "src/Acme/Core/ParamsBase.cs"


def test_template_root_exists() -> None:
    root = template_root("csharp")
    assert root.joinpath("src", "__FerruleNs__", "Core", "ParamsBase.cs").is_file()


def test_names_paths_and_crlf() -> None:
    files = render_csharp_files(
        CSharpOptions(namespace="Acme", client_name="AcmeCloud", env_prefix="ACME_CLOUD")
    )
    assert "src/Acme/Exceptions/AcmeCloudException.cs" in files
    assert "src/Acme.Tests/Core/JsonDictionaryTest.cs" in files
    params = files[PARAMS_BASE]
    assert params.startswith("using System;\r\nusing System.Collections.Generic;\r\n")
    assert "\r\nnamespace Acme.Core;\r\n" in params
    assert "typeof(AcmeCloudClient).Name" in params
    assert '"ACME_CLOUD_CUSTOM_HEADERS"' in params
    assert all("\n" not in text.replace("\r\n", "") for text in files.values())
    assert not any("ferrule" in rel.lower() or "ferrule" in t.lower() for rel, t in files.items())


def test_auth_headers_slot() -> None:
    assert "options." not in files_between(render_csharp_files(ACME)[PARAMS_BASE])
    auth = (
        "        if (options.ApiKey != null)\n"
        "        {\n"
        '            request.Headers.Add("X-Api-Key", options.ApiKey);\n'
        "        }\n"
    )
    text = render_csharp_files(
        CSharpOptions(namespace="Acme", client_name="Acme", env_prefix="ACME", auth_headers=auth)
    )[PARAMS_BASE]
    assert (
        auth.replace("\n", "\r\n")
        + '        request.Headers.Add(\r\n            "X-Stainless-Timeout"'
        in text
    )


def test_auth_query_slot() -> None:
    plain = render_csharp_files(ACME)[PARAMS_BASE]
    auth = (
        "        if (options.ApiKey != null)\n"
        "        {\n"
        '            collection.Add("key", options.ApiKey);\n'
        "        }\n"
    )
    text = render_csharp_files(
        CSharpOptions(namespace="Acme", client_name="Acme", env_prefix="ACME", auth_query=auth)
    )[PARAMS_BASE]
    # After the request's own query parameters, before the query string is built.
    loop_end = "item.Key, item.Value);\r\n        }\r\n"
    assert loop_end + auth.replace("\n", "\r\n") + "        StringBuilder sb = new();" in text
    assert text.replace(auth.replace("\n", "\r\n"), "") == plain


def files_between(text: str) -> str:
    """The body of `AddDefaultHeaders` up to the timeout header."""
    start = text.index("static void AddDefaultHeaders")
    return text[start : text.index("X-Stainless-Timeout", start)]


@pytest.mark.parametrize(
    ("namespace", "first"),
    [("Acme", "using Acme;"), ("Stagehand", "using Moq;"), ("Mo", "using Mo;")],
)
def test_usings_sort_around_moq(namespace: str, first: str) -> None:
    text = render_csharp_files(
        CSharpOptions(namespace=namespace, client_name="Acme", env_prefix="ACME")
    )[f"src/{namespace}.Tests/RetriesTest.cs"]
    after_system = text.split("using System.Threading.Tasks;\r\n", 1)[1]
    assert after_system.startswith(first)


@pytest.mark.parametrize(
    ("kwargs", "match"),
    [
        ({"namespace": "acme"}, "namespace"),
        ({"namespace": "Acme.Sdk"}, "namespace"),
        ({"client_name": "Acme Cloud"}, "client_name"),
        ({"env_prefix": "acme"}, "env_prefix"),
        ({"auth_headers": "if (x) {}"}, "auth_headers must be empty or end with"),
        ({"auth_query": "if (x) {}"}, "auth_query must be empty or end with"),
    ],
)
def test_rejects_bad_options(kwargs: dict[str, Any], match: str) -> None:
    options: dict[str, Any] = {
        "namespace": "Acme",
        "client_name": "Acme",
        "env_prefix": "ACME",
        **kwargs,
    }
    with pytest.raises(RenderError, match=match):
        render_csharp_files(CSharpOptions(**options))
