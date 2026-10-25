# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the JVM runtime (``runtime/java/template``) for one Java or Kotlin SDK.

Stainless writes both its ``java`` and ``kotlin`` targets in Kotlin with the same Gradle layout:
the runtime is ``<artifact>-core/src/{main,test}/kotlin/<package>/{core,errors}/**``, the
ProGuard rules, and ``OkHttpClient.kt`` in ``<artifact>-client-okhttp``; see ``runtime/NOTICE``.
One template serves both targets: the Java target adds ``Optional`` overloads and ``@Jvm*``
annotations, kept in ``//ferrule:if java`` blocks, and the Kotlin target's own text sits in
``//ferrule:if kotlin`` blocks. Paths are relative to the Gradle project root.

ktfmt sorts every import block by import path, so the order depends on the package name
(``io.acme`` sorts before ``java.util``, ``org.acme`` after): the renderer re-sorts each block
after substituting the names, the same rule ktfmt applies. Lines whose wrapping depends on the
client-name length are kept in both forms in ``//ferrule:if fits`` blocks (ktfmt's 100 columns).
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Literal

from ferrule_runtime._render import RenderError, render_tree, template_root

__all__ = ["JavaArrayFormat", "JavaOptions", "JavaTarget", "render_java_files"]

JavaTarget = Literal["java", "kotlin"]
JavaArrayFormat = Literal["comma", "repeat"]

ARTIFACT_PLACEHOLDER = "__ferrule_artifact__"
PKG_PATH_PLACEHOLDER = "__ferrule_pkg_path__"
PKG_PLACEHOLDER = "__ferrule_pkg__"
CLIENT_PLACEHOLDER = "__FerruleClient__"
LOG_ENV_PLACEHOLDER = "__FERRULE_LOG_ENV__"
REPO_PLACEHOLDER = "__FERRULE_REPO__"

_PACKAGE = re.compile(r"^[a-z][a-z0-9_]*(?:\.[a-z][a-z0-9_]*)*$")
_ARTIFACT = re.compile(r"^[a-z0-9][a-z0-9.-]*$")
_REPO = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
_IDENT = re.compile(r"^[A-Z][A-Za-z0-9]*$")
_ENV = re.compile(r"^[A-Z][A-Z0-9_]*$")
_CAMEL_BOUNDARY = re.compile(r"(?<=[a-z0-9])(?=[A-Z])")
_LINE_LENGTH = 100  # ktfmt --kotlinlang-style
_CORE = f"{ARTIFACT_PLACEHOLDER}-core/src"
_MAIN = f"{_CORE}/main/kotlin/{PKG_PATH_PLACEHOLDER}"
_TEST = f"{_CORE}/test/kotlin/{PKG_PATH_PLACEHOLDER}"
_JAVA_ONLY = (
    f"{ARTIFACT_PLACEHOLDER}-client-okhttp/src/test/",
    f"{_MAIN}/core/http/AsyncStreamResponse.kt",
    f"{_MAIN}/core/http/PhantomReachableClosingAsyncStreamResponse.kt",
    f"{_TEST}/core/http/AsyncStreamResponseTest.kt",
)


@dataclass(frozen=True, kw_only=True)
class JavaOptions:
    """Settings that vary the JVM runtime between generated SDKs."""

    package_name: str
    """``reverse_domain`` (``org.onebusaway``): every ``package`` line and directory."""
    client_name: str
    """Client class prefix (``OnebusawaySdk``): ``<client>Exception`` and the other errors."""
    artifact_name: str
    """Gradle project prefix (``onebusaway-sdk-java``): ``<artifact>-core``,
    ``<artifact>-client-okhttp`` and the ProGuard rules file."""
    repo: str
    """GitHub ``owner/name`` (``OneBusAway/java-sdk``), linked from the Jackson version check."""
    target: JavaTarget = "java"
    log_env_var: str | None = None
    """Read by ``LogLevel.fromEnv``; default ``<CLIENT_SCREAMING_SNAKE>_LOG``."""
    array_format: JavaArrayFormat = "comma"
    """``query_settings.array_format``: arrays in query strings and multipart bodies are
    comma-joined (the default) or repeated."""
    webhooks: bool = False
    """Adds ``UnwrapWebhookParams`` and ``<client>WebhookException`` (webhook unwrapping)."""
    empty_responses: bool = False
    """Adds ``handlers/EmptyHandler.kt``, used by methods whose response has no body."""
    mock_tests: bool = True
    """Adds the test server extension (``TestServerExtension.kt``); off with
    ``disable_mock_tests``."""


def _screaming_snake(name: str) -> str:
    return _CAMEL_BOUNDARY.sub("_", name).upper()


def _check(options: JavaOptions) -> None:
    for kind, value, pattern in (
        ("package_name", options.package_name, _PACKAGE),
        ("client_name", options.client_name, _IDENT),
        ("artifact_name", options.artifact_name, _ARTIFACT),
        ("repo", options.repo, _REPO),
    ):
        if not pattern.match(value):
            msg = f"{kind} is not valid: {value!r}"
            raise RenderError(msg)
    if options.log_env_var is not None and not _ENV.match(options.log_env_var):
        msg = f"log_env_var must be an environment variable name, got {options.log_env_var!r}"
        raise RenderError(msg)


def _flags(options: JavaOptions) -> dict[str, bool]:
    return {
        "java": options.target == "java",
        "kotlin": options.target == "kotlin",
        "comma": options.array_format == "comma",
    }


def _line_length(rel: str) -> int | None:
    return _LINE_LENGTH if rel.endswith(".kt") else None


def _sort_imports(text: str) -> str:
    """Sort each block of consecutive ``import`` lines by path, as ktfmt does."""
    out: list[str] = []
    block: list[str] = []
    for line in text.splitlines(keepends=True):
        if line.startswith("import "):
            block.append(line)
            continue
        out += sorted(block)
        block = []
        out.append(line)
    return "".join(out + sorted(block))


def render_java_files(options: JavaOptions) -> dict[str, str]:
    """Render the JVM runtime (pure): ``{path from the Gradle root: exact text}``."""
    _check(options)
    files = render_tree(
        template_root("java"),
        comment="//",
        replacements={
            ARTIFACT_PLACEHOLDER: options.artifact_name,
            PKG_PATH_PLACEHOLDER: options.package_name.replace(".", "/"),
            PKG_PLACEHOLDER: options.package_name,
            LOG_ENV_PLACEHOLDER: options.log_env_var
            or f"{_screaming_snake(options.client_name)}_LOG",
            REPO_PLACEHOLDER: options.repo,
            CLIENT_PLACEHOLDER: options.client_name,
        },
        flags=_flags(options),
        line_length=_line_length,
        gated={
            **dict.fromkeys(_JAVA_ONLY, options.target == "java"),
            f"{_MAIN}/core/UnwrapWebhookParams.kt": options.webhooks,
            f"{_MAIN}/errors/{CLIENT_PLACEHOLDER}WebhookException.kt": options.webhooks,
            f"{_MAIN}/core/handlers/EmptyHandler.kt": options.empty_responses,
            f"{_TEST}/TestServerExtension.kt": options.mock_tests,
        },
    )
    return {
        rel: _sort_imports(text) if rel.endswith(".kt") else text for rel, text in files.items()
    }
