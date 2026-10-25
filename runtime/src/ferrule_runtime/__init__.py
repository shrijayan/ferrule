# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Runtime support code copied into Ferrule-generated SDKs."""

from ferrule_runtime._go import GoOptions, render_go_files
from ferrule_runtime._render import (
    Language,
    RenderError,
    RuntimeOptions,
    default_client_name,
    render,
    render_files,
    render_options,
    render_tree,
    template_root,
)
from ferrule_runtime._retry import retry_delay, should_retry
from ferrule_runtime._typescript import (
    QueryArrayFormat,
    TypeScriptOptions,
    render_typescript_files,
)
from ferrule_runtime.pagination import (
    PageField,
    PageKind,
    PageModel,
    PageScheme,
    PaginationError,
    render_pagination,
)

__all__ = [
    "GoOptions",
    "Language",
    "PageField",
    "PageKind",
    "PageModel",
    "PageScheme",
    "PaginationError",
    "QueryArrayFormat",
    "RenderError",
    "RuntimeOptions",
    "TypeScriptOptions",
    "default_client_name",
    "render",
    "render_files",
    "render_go_files",
    "render_options",
    "render_pagination",
    "render_tree",
    "render_typescript_files",
    "retry_delay",
    "should_retry",
    "template_root",
]
