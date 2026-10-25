# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the Go runtime (``runtime/go/template``) for one generated SDK (Stainless Go v1).

The template is the runtime of the Stainless-generated OneBusAway Go SDK: ``internal/**``
(without ``internal/version.go``), ``option/**``, ``field.go`` and ``default_http_client.go``;
see ``runtime/NOTICE``. Paths are relative to the module root.

Two runtime files carry config-dependent code that the plugin generates: the client settings
in ``internal/requestconfig/requestconfig.go`` and the environment and client-setting options
at the end of ``option/requestoption.go``. They are template slots, filled with the caller's
gofmt-formatted text.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from ferrule_runtime._render import RenderError, render_tree, template_root

__all__ = ["GoOptions", "render_go_files"]

MODULE_PLACEHOLDER = "__ferrule_module__"
PKG_PLACEHOLDER = "__ferrule_pkg__"
CLIENT_PLACEHOLDER = "__FerruleClient__"
ORG_PLACEHOLDER = "__FERRULE_ORG__"
MAX_RETRIES_PLACEHOLDER = "__FERRULE_MAX_RETRIES__"
SENSITIVE_HEADERS_PLACEHOLDER = "__FERRULE_SENSITIVE_HEADERS__"

_MODULE = re.compile(r"^[A-Za-z0-9.~_-]+(?:/[A-Za-z0-9.~_-]+)*$")
_HEADER = re.compile(r"^[a-z0-9!#$%&*+.^_`|~-]+$")
_GO_IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
_GO_KEYWORDS = frozenset(
    {
        "break", "case", "chan", "const", "continue", "default", "defer", "else",
        "fallthrough", "for", "func", "go", "goto", "if", "import", "interface", "map",
        "package", "range", "return", "select", "struct", "switch", "type", "var",
    }
)  # fmt: skip
_SLOTS = ("config_fields", "config_clone", "client_options")


@dataclass(frozen=True, kw_only=True)
class GoOptions:
    """Settings that vary the Go runtime between generated SDKs."""

    module_path: str
    """Go module path (``github.com/OneBusAway/go-sdk``); every internal import uses it."""
    package_name: str
    """Root package (``onebusaway``) of ``field.go`` and ``default_http_client.go``."""
    client_name: str
    """User-Agent product (``OnebusawaySDK`` sends ``OnebusawaySDK/Go <version>``)."""
    org_name: str | None = None
    """``organization.name`` (``onebusaway-sdk``), named in the ``RequestOption`` doc comment.
    Defaults to ``package_name``."""
    max_retries: int = 2
    redacted_headers: tuple[str, ...] = ("api-key", "x-api-key")
    """Headers ``option.WithDebugLog`` masks besides ``authorization`` and cookies (as in the
    TypeScript runtime's ``log.ts``)."""
    config_fields: str = ""
    """``RequestConfig`` struct fields for client settings (``\\tAPIKey         *string\\n``)."""
    config_clone: str = ""
    """The same fields in ``RequestConfig.Clone`` (``\\t\\tAPIKey:         cfg.APIKey,\\n``)."""
    client_options: str = ""
    """Appended to ``option/requestoption.go``: ``WithEnvironment*`` and client-setting options,
    starting with the blank line that separates them from ``WithRequestTimeout``."""


def _check(options: GoOptions) -> None:
    if not _MODULE.match(options.module_path):
        msg = f"module_path must be a Go module path, got {options.module_path!r}"
        raise RenderError(msg)
    for kind, value in (
        ("package_name", options.package_name),
        ("client_name", options.client_name),
    ):
        if not _GO_IDENT.match(value) or value in _GO_KEYWORDS:
            msg = f"{kind} must be a Go identifier, got {value!r}"
            raise RenderError(msg)
    if options.max_retries < 0:
        msg = "max_retries must be non-negative"
        raise RenderError(msg)
    for header in options.redacted_headers:
        if not _HEADER.match(header):
            msg = f"redacted_headers must be lower-case header names, got {header!r}"
            raise RenderError(msg)
    for name in _SLOTS:
        text: str = getattr(options, name)
        if text and not text.endswith("\n"):
            msg = f"{name} must be empty or end with a newline"
            raise RenderError(msg)


def _sensitive_headers(redacted: tuple[str, ...]) -> str:
    names = ("authorization", *redacted, "cookie", "set-cookie")
    return ", ".join(f'"{name}"' for name in names)


def render_go_files(options: GoOptions) -> dict[str, str]:
    """Render the Go runtime (pure): ``{path from the module root: exact text}``."""
    _check(options)
    module = options.module_path
    return render_tree(
        template_root("go"),
        comment="//",
        replacements={
            MAX_RETRIES_PLACEHOLDER: str(options.max_retries),
            MODULE_PLACEHOLDER: module,
            CLIENT_PLACEHOLDER: options.client_name,
            SENSITIVE_HEADERS_PLACEHOLDER: _sensitive_headers(options.redacted_headers),
            ORG_PLACEHOLDER: options.org_name or options.package_name,
            PKG_PLACEHOLDER: options.package_name,
        },
        # gofmt sorts the imports of a group by path; these two groups mix the module's own
        # packages with another import.
        flags={
            "module_before_io": f"{module}/internal/param" < "io",
            "module_before_sjson": f"{module}/internal/requestconfig" < "github.com/tidwall/sjson",
        },
        slots={name: getattr(options, name) for name in _SLOTS},
    )
