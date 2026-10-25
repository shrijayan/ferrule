# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Rendering the JVM runtime: Java and Kotlin targets, paths, import order and line wrapping."""

from __future__ import annotations

from typing import Any

import pytest

from ferrule_runtime import JavaOptions, RenderError, render_java_files, template_root

ACME = JavaOptions(
    package_name="com.acme.api",
    client_name="Acme",
    artifact_name="acme-java",
    repo="acme/acme-java",
)
CORE = "acme-java-core/src/main/kotlin/com/acme/api"


def _with(**kwargs: Any) -> JavaOptions:  # noqa: ANN401 - JavaOptions fields
    options: dict[str, Any] = {
        "package_name": ACME.package_name,
        "client_name": ACME.client_name,
        "artifact_name": ACME.artifact_name,
        "repo": ACME.repo,
        **kwargs,
    }
    return JavaOptions(**options)


def test_template_root_exists() -> None:
    assert template_root("java").joinpath("__ferrule_artifact__-core").is_dir()


def test_java_defaults() -> None:
    files = render_java_files(ACME)
    assert f"{CORE}/errors/AcmeException.kt" in files
    assert f"{CORE}/core/http/AsyncStreamResponse.kt" in files
    assert "acme-java-core/src/main/resources/META-INF/proguard/acme-java-core.pro" in files
    assert "acme-java-core/src/test/kotlin/com/acme/api/TestServerExtension.kt" in files
    assert f"{CORE}/core/UnwrapWebhookParams.kt" not in files
    assert f"{CORE}/core/handlers/EmptyHandler.kt" not in files
    log = files[f"{CORE}/core/LogLevel.kt"]
    assert 'System.getenv("ACME_LOG")' in log
    assert "https://www.github.com/acme/acme-java#jackson" in files[f"{CORE}/core/Check.kt"]
    assert '.joinToString(",")' in files[f"{CORE}/core/http/QueryParams.kt"]
    assert "@JvmStatic fun builder()" in files[f"{CORE}/core/http/Headers.kt"]
    assert not any("ferrule" in rel.lower() or "ferrule" in t.lower() for rel, t in files.items())


def test_kotlin_target_drops_the_java_api() -> None:
    files = render_java_files(_with(target="kotlin", artifact_name="acme-kotlin"))
    core = "acme-kotlin-core/src/main/kotlin/com/acme/api"
    assert f"{core}/core/http/AsyncStreamResponse.kt" not in files
    assert not any(rel.startswith("acme-kotlin-client-okhttp/src/test/") for rel in files)
    headers = files[f"{core}/core/http/Headers.kt"]
    assert "@JvmStatic" not in headers
    assert "        fun builder() = Builder()\n" in headers


def test_repeat_array_format() -> None:
    files = render_java_files(_with(array_format="repeat"))
    assert (
        "is JsonArray -> value.values.forEach { put(key, it) }"
        in files[f"{CORE}/core/http/QueryParams.kt"]
    )


@pytest.mark.parametrize(
    ("package", "before", "after"),
    [
        # ktfmt sorts imports by path: the package moves around java.* and kotlin.*.
        ("io.acme", "import io.acme.core.http.Headers\n", "import java.util.Objects\n"),
        ("org.acme", "import java.util.Objects\n", "import org.acme.core.http.Headers\n"),
    ],
)
def test_imports_follow_ktfmt_order(package: str, before: str, after: str) -> None:
    files = render_java_files(_with(package_name=package, webhooks=True))
    path = package.replace(".", "/")
    text = files[f"acme-java-core/src/main/kotlin/{path}/core/UnwrapWebhookParams.kt"]
    assert text.index(before) < text.index(after)


def test_wrapping_follows_the_client_name_length() -> None:
    io = "constructor(message: String? = null, cause: Throwable? = null) :"
    short = render_java_files(ACME)[f"{CORE}/errors/AcmeIoException.kt"]
    assert f"{io} AcmeException(message, cause)\n" in short
    name = "AcmeLongerClientName"
    long = render_java_files(_with(client_name=name))[f"{CORE}/errors/{name}IoException.kt"]
    assert f"{io}\n    {name}Exception(message, cause)\n" in long


def test_optional_files() -> None:
    files = render_java_files(_with(webhooks=True, empty_responses=True, mock_tests=False))
    assert f"{CORE}/core/UnwrapWebhookParams.kt" in files
    assert f"{CORE}/errors/AcmeWebhookException.kt" in files
    assert f"{CORE}/core/handlers/EmptyHandler.kt" in files
    assert "acme-java-core/src/test/kotlin/com/acme/api/TestServerExtension.kt" not in files


@pytest.mark.parametrize(
    ("kwargs", "match"),
    [
        ({"package_name": "Com.acme"}, "package_name"),
        ({"package_name": "com..acme"}, "package_name"),
        ({"client_name": "acme"}, "client_name"),
        ({"artifact_name": "Acme Java"}, "artifact_name"),
        ({"repo": "acme"}, "repo"),
        ({"log_env_var": "acme-log"}, "log_env_var"),
    ],
)
def test_rejects_bad_options(kwargs: dict[str, Any], match: str) -> None:
    with pytest.raises(RenderError, match=match):
        render_java_files(_with(**kwargs))
