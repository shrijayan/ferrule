# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Runtime support code copied into Ferrule-generated SDKs."""

from ferrule_runtime._csharp import CSharpOptions, render_csharp_files
from ferrule_runtime._go import GoOptions, render_go_files
from ferrule_runtime._java import JavaArrayFormat, JavaOptions, JavaTarget, render_java_files
from ferrule_runtime._php import PhpOptions, render_php_files
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
from ferrule_runtime._ruby import RubyArrayFormat, RubyAuthIn, RubyOptions, render_ruby_files
from ferrule_runtime._typescript import (
    QueryArrayFormat,
    TypeScriptOptions,
    render_typescript_files,
)
from ferrule_runtime._typescript_node import TypeScriptNodeOptions, render_typescript_node_files
from ferrule_runtime.pagination import (
    PageField,
    PageKind,
    PageModel,
    PageScheme,
    PaginationError,
    page_schemes,
    render_pagination,
)

__all__ = [
    "CSharpOptions",
    "GoOptions",
    "JavaArrayFormat",
    "JavaOptions",
    "JavaTarget",
    "Language",
    "PageField",
    "PageKind",
    "PageModel",
    "PageScheme",
    "PaginationError",
    "PhpOptions",
    "QueryArrayFormat",
    "RenderError",
    "RubyArrayFormat",
    "RubyAuthIn",
    "RubyOptions",
    "RuntimeOptions",
    "TypeScriptNodeOptions",
    "TypeScriptOptions",
    "default_client_name",
    "page_schemes",
    "render",
    "render_csharp_files",
    "render_files",
    "render_go_files",
    "render_java_files",
    "render_options",
    "render_pagination",
    "render_php_files",
    "render_ruby_files",
    "render_tree",
    "render_typescript_files",
    "render_typescript_node_files",
    "retry_delay",
    "should_retry",
    "template_root",
]
