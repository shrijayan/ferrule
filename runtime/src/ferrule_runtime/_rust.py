# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the Rust runtime (``rust/template``) for one generated crate (ADR 0019).

Rust is a Ferrule-original target: Stainless has no Rust SDK, so this runtime is Ferrule's own
code, not a tokenised oracle runtime. Nothing in it varies between crates (each crate's names
and settings live in its generated ``client.rs``), so it has no tokens or directives and renders
to the template's files unchanged. ``ferrule-gen generate --target rust`` embeds the same files
(``crates/rust-plugin/src/runtime.rs``, checked against this directory by that crate's
``tests/runtime.rs``), and ``tests/test_render_rust.py`` checks that rendering changes nothing.
Paths are relative to the crate root.
"""

from __future__ import annotations

from ferrule_runtime._render import render_tree, template_root

__all__ = ["render_rust_files"]


def render_rust_files() -> dict[str, str]:
    """Render the Rust runtime (pure): ``{path from the crate root: exact text}``."""
    return render_tree(template_root("rust"), comment="//", replacements={}, flags={})
