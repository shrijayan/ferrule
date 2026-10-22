# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Runtime support code copied into Ferrule-generated SDKs."""

from ferrule_runtime._render import (
    RenderError,
    RuntimeOptions,
    default_client_name,
    render,
    render_files,
    render_options,
    template_root,
)
from ferrule_runtime._retry import retry_delay, should_retry
from ferrule_runtime.pagination import (
    PageField,
    PageKind,
    PageModel,
    PageScheme,
    PaginationError,
    render_pagination,
)

__all__ = [
    "PageField",
    "PageKind",
    "PageModel",
    "PageScheme",
    "PaginationError",
    "RenderError",
    "RuntimeOptions",
    "default_client_name",
    "render",
    "render_files",
    "render_options",
    "render_pagination",
    "retry_delay",
    "should_retry",
    "template_root",
]
