# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Rendering the Rust runtime: Ferrule's own code (ADR 0019), with nothing to replace."""

from __future__ import annotations

from pathlib import Path

from ferrule_runtime import render_rust_files, template_root


def _template_files() -> dict[str, str]:
    root = Path(str(template_root("rust")))
    return {
        p.relative_to(root).as_posix(): p.read_text(encoding="utf-8")
        for p in sorted(root.rglob("*"))
        if p.is_file()
    }


def test_renders_the_template_unchanged() -> None:
    files = render_rust_files()
    assert files == _template_files()
    assert "src/rt/http.rs" in files
    assert "src/testing/server.rs" in files


def test_runtime_names_no_sdk_and_no_vendor() -> None:
    for rel, text in render_rust_files().items():
        lowered = text.lower()
        assert "stainless" not in lowered, rel
        assert "onebusaway" not in lowered, rel
        assert "__ferrule" not in text, rel
