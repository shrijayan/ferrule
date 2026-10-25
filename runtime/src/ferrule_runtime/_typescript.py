# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the TypeScript runtime (``runtime/typescript/template``) for one generated SDK.

The template is the runtime of the Stainless-generated Isaacus TypeScript SDK (the
``typescript`` target's current layout: ``src/core/**``, ``src/internal/**`` and the
``src/*.ts`` re-export shims), plus ``src/internal/qs/**`` from knock-mgmt-node; see
``runtime/NOTICE``. Paths are relative to the SDK repository root.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Literal

from ferrule_runtime._render import RenderError, render_tree, template_root

__all__ = ["QueryArrayFormat", "TypeScriptOptions", "render_typescript_files"]

QueryArrayFormat = Literal["comma", "brackets", "repeat", "indices"]

CLIENT_PLACEHOLDER = "__FerruleClient__"
ERROR_PLACEHOLDER = "__FerruleError__"
MAX_RETRIES_PLACEHOLDER = "__FERRULE_MAX_RETRIES__"
QS_ARRAY_FORMAT_PLACEHOLDER = "__FERRULE_QS_ARRAY_FORMAT__"

_QS_DIR = "src/internal/qs/"
_ARRAY_FORMATS = frozenset({"comma", "brackets", "repeat", "indices"})
_JS_IDENT = re.compile(r"^[A-Za-z_$][A-Za-z0-9_$]*$")
_HEADER = re.compile(r"^[a-z0-9!#$%&*+.^_`|~-]+$")


@dataclass(frozen=True, kw_only=True)
class TypeScriptOptions:
    """Settings that vary the TypeScript runtime between generated SDKs."""

    client_name: str
    """Client class (``Isaacus``, ``ImageKit``); the runtime imports its type from ``client``."""
    error_name: str | None = None
    """Base error class; Stainless uses ``<ClientName>Error``."""
    max_retries: int = 2
    query_array_format: QueryArrayFormat | None = None
    """``None``: the basic ``stringifyQuery`` (primitives only). A format: the vendored ``qs``
    stringifier in ``src/internal/qs/`` with that ``arrayFormat`` (nested query params)."""
    redacted_headers: tuple[str, ...] = ("api-key", "x-api-key")
    """Header names ``internal/utils/log.ts`` masks besides ``authorization`` and cookies.
    Every 2026 oracle lists ``api-key`` and ``x-api-key``; papr's 2025 generation listed its
    security-scheme headers instead (``x-session-token``, ``x-api-key``)."""
    pagination: bool = False
    """Re-export ``PagePromise`` from ``src/index.ts``. ``src/core/pagination.ts`` and
    ``src/pagination.ts`` are generated from the config's schemes, not rendered here."""


def render_typescript_files(options: TypeScriptOptions) -> dict[str, str]:
    """Render the TypeScript runtime (pure): ``{path from the SDK root: exact text}``."""
    error_name = options.error_name or f"{options.client_name}Error"
    for kind, value in (("client_name", options.client_name), ("error_name", error_name)):
        if not _JS_IDENT.match(value):
            msg = f"{kind} must be a JavaScript identifier, got {value!r}"
            raise RenderError(msg)
    if options.max_retries < 0:
        msg = "max_retries must be non-negative"
        raise RenderError(msg)
    fmt = options.query_array_format
    if fmt is not None and fmt not in _ARRAY_FORMATS:
        msg = f"query_array_format must be one of {sorted(_ARRAY_FORMATS)}, got {fmt!r}"
        raise RenderError(msg)
    for header in options.redacted_headers:
        if not _HEADER.match(header):
            msg = f"redacted_headers must be lower-case header names, got {header!r}"
            raise RenderError(msg)
    redacted = "".join(
        f"            name.toLowerCase() === '{header}' ||\n" for header in options.redacted_headers
    )
    return render_tree(
        template_root("typescript"),
        comment="//",
        replacements={
            QS_ARRAY_FORMAT_PLACEHOLDER: fmt or "",
            MAX_RETRIES_PLACEHOLDER: str(options.max_retries),
            ERROR_PLACEHOLDER: error_name,
            CLIENT_PLACEHOLDER: options.client_name,
        },
        flags={"qs": fmt is not None, "pagination": options.pagination},
        slots={"redacted_headers": redacted},
        gated={_QS_DIR: fmt is not None},
    )
