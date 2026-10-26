# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the legacy ``node`` TypeScript runtime (``runtime/typescript-node/template``).

The template is the runtime of the Stainless-generated OneBusAway JS SDK, the ``node`` target's
layout (``src/core.ts``, ``src/_shims/**``, ``src/error.ts``, ``src/uploads.ts``, the
``src/resource.ts`` / ``src/resources.ts`` / ``src/shims/*.ts`` entry points and
``src/internal/utils/query.ts``); see ``runtime/NOTICE``. Paths are relative to the SDK
repository root. The ``typescript`` target's current layout is a different runtime
(:mod:`ferrule_runtime._typescript`): no file of one is a file of the other.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from ferrule_runtime._render import RenderError, render_tree, template_root

__all__ = ["TypeScriptNodeOptions", "render_typescript_node_files"]

CLIENT_PLACEHOLDER = "__FerruleClient__"
ERROR_PLACEHOLDER = "__FerruleError__"
PACKAGE_PLACEHOLDER = "__ferrule_package__"
REPO_PLACEHOLDER = "__FERRULE_REPO__"

# prettier's printWidth in every node-layout oracle (`.prettierrc.json`).
PRINT_WIDTH = 110

_JS_IDENT = re.compile(r"^[A-Za-z_$][A-Za-z0-9_$]*$")
_NPM_NAME = re.compile(r"^(?:@[a-z0-9][a-z0-9._-]*/)?[a-z0-9][a-z0-9._-]*$")
_REPO = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")

# The lines prettier keeps on one line only while they fit in PRINT_WIDTH; the template has both
# layouts behind a flag.
_ENSURE_PRESENT = (
    "  if (value == null) throw new {error}("
    "`Expected a value to be given but received ${{value}} instead.`);"
)
_REGISTRY = (
    "    throw new Error(`can't \\`import '{package}/shims/${{shims.kind}}'\\` after "
    "\\`import '{package}/shims/${{kind}}'\\``);"
)


@dataclass(frozen=True, kw_only=True)
class TypeScriptNodeOptions:
    """Settings that vary the ``node`` runtime between generated SDKs."""

    client_name: str
    """Client class (``OnebusawaySDK``, ``LumaAI``); ``src/resource.ts`` imports its type."""
    package_name: str
    """The npm package (``onebusaway-sdk``, ``@honcho-ai/core``), named in shim import paths."""
    repo: str
    """GitHub ``owner/name`` of the SDK; the shims link its README's file-upload section."""
    error_name: str | None = None
    """Base error class; Stainless uses ``<ClientName>Error``."""


def render_typescript_node_files(options: TypeScriptNodeOptions) -> dict[str, str]:
    """Render the ``node`` runtime (pure): ``{path from the SDK root: exact text}``."""
    error_name = options.error_name or f"{options.client_name}Error"
    for kind, value in (("client_name", options.client_name), ("error_name", error_name)):
        if not _JS_IDENT.match(value):
            msg = f"{kind} must be a JavaScript identifier, got {value!r}"
            raise RenderError(msg)
    if not _NPM_NAME.match(options.package_name):
        msg = f"package_name must be an npm package name, got {options.package_name!r}"
        raise RenderError(msg)
    if not _REPO.match(options.repo):
        msg = f"repo must be a GitHub owner/name, got {options.repo!r}"
        raise RenderError(msg)
    flags = {
        "ensure_present_fits": len(_ENSURE_PRESENT.format(error=error_name)) <= PRINT_WIDTH,
        "registry_fits": len(_REGISTRY.format(package=options.package_name)) <= PRINT_WIDTH,
    }
    return render_tree(
        template_root("typescript-node"),
        comment="//",
        replacements={
            ERROR_PLACEHOLDER: error_name,
            CLIENT_PLACEHOLDER: options.client_name,
            PACKAGE_PLACEHOLDER: options.package_name,
            REPO_PLACEHOLDER: options.repo,
        },
        flags=flags,
    )
