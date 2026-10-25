# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the Ruby runtime (``runtime/ruby/template``) for one generated gem.

The template is the runtime of the Stainless-generated OneBusAway Ruby SDK: ``errors``,
``file_part``, ``internal`` (``internal/transport/**``, ``internal/type/**``, ``util``) and
``request_options`` in each of the three views (``lib/**.rb``, ``rbi/**.rbi`` for Sorbet and
``sig/**.rbs`` for RBS), plus their tests; see ``runtime/NOTICE``. Paths are relative to the
gem root.

The ``@example`` blocks of ``BaseModel``, ``Enum`` and ``Union`` show one of the SDK's own
models, so they are slots the plugin fills. Lines whose wrapping depends on the module-name
length (rubocop's 110 columns in ``.rb``, syntax_tree's 80 in ``.rbi`` and ``.rbs``) are kept
in both forms in ``#ferrule:if fits`` blocks.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Literal

from ferrule_runtime._render import RenderError, render_tree, template_root

__all__ = ["RubyArrayFormat", "RubyAuthIn", "RubyOptions", "render_ruby_files"]

RubyArrayFormat = Literal["comma", "repeat"]
RubyAuthIn = Literal["headers", "query"]

GEM_PLACEHOLDER = "__ferrule_gem__"
MODULE_PLACEHOLDER = "__FerruleModule__"

_GEM = re.compile(r"^[a-z][a-z0-9_]*$")
_MODULE = re.compile(r"^[A-Z][A-Za-z0-9]*$")
_RB_LINE = 110  # .rubocop.yml Layout/LineLength
_STREE_LINE = 80  # syntax_tree's default print width (.rbi, .rbs)
_SLOTS = ("base_model_example", "enum_example", "union_example")


@dataclass(frozen=True, kw_only=True)
class RubyOptions:
    """Settings that vary the Ruby runtime between generated gems."""

    gem_name: str
    """``lib/<gem>`` (``onebusaway_sdk``)."""
    module_name: str
    """Top-level module (``OnebusawaySDK``)."""
    array_format: RubyArrayFormat = "comma"
    """``query_settings.array_format``: query arrays comma-joined (the default) or repeated."""
    auth_in: RubyAuthIn = "headers"
    """Where the client's credentials go: ``BaseClient#auth_headers`` or ``#auth_query``
    (an API key sent as a query parameter)."""
    base_model_example: str = ""
    """``# @example`` lines of ``BaseModel`` (``lib`` view), from the first model."""
    enum_example: str = ""
    """``# @example`` lines of ``Enum``; empty when the SDK has no enum."""
    union_example: str = ""
    """``# @example`` lines of ``Union``; empty when the SDK has no union."""


def _check(options: RubyOptions) -> None:
    if not _GEM.match(options.gem_name):
        msg = f"gem_name must be a lower-case Ruby file name, got {options.gem_name!r}"
        raise RenderError(msg)
    if not _MODULE.match(options.module_name):
        msg = f"module_name must be a Ruby constant name, got {options.module_name!r}"
        raise RenderError(msg)
    for name in _SLOTS:
        text: str = getattr(options, name)
        if text and not text.endswith("\n"):
            msg = f"{name} must be empty or end with a newline"
            raise RenderError(msg)


def _flags(options: RubyOptions) -> dict[str, bool]:
    return {
        "comma": options.array_format == "comma",
        "auth_query": options.auth_in == "query",
    }


def _line_length(rel: str) -> int | None:
    if rel.endswith(".rb"):
        return _RB_LINE
    return _STREE_LINE if rel.endswith((".rbi", ".rbs")) else None


def render_ruby_files(options: RubyOptions) -> dict[str, str]:
    """Render the Ruby runtime (pure): ``{path from the gem root: exact text}``."""
    _check(options)
    return render_tree(
        template_root("ruby"),
        comment="#",
        replacements={
            MODULE_PLACEHOLDER: options.module_name,
            GEM_PLACEHOLDER: options.gem_name,
        },
        flags=_flags(options),
        line_length=_line_length,
        slots={name: getattr(options, name) for name in _SLOTS},
    )
