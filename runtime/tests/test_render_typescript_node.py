# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Rendering the ``node`` TypeScript runtime: options and their effect on the template."""

from __future__ import annotations

import pytest

from ferrule_runtime import (
    RenderError,
    TypeScriptNodeOptions,
    render_typescript_files,
    render_typescript_node_files,
    template_root,
)
from ferrule_runtime._typescript import TypeScriptOptions


def _render(**kwargs: str) -> dict[str, str]:
    options = {"client_name": "Acme", "package_name": "acme", "repo": "acme/acme-node"} | kwargs
    return render_typescript_node_files(TypeScriptNodeOptions(**options))


def test_template_root_exists() -> None:
    assert template_root("typescript-node").joinpath("src", "core.ts").is_file()


def test_node_defaults() -> None:
    files = _render()
    assert "export class AcmeError extends Error {}" in files["src/error.ts"]
    assert "import type { Acme } from './index';" in files["src/resource.ts"]
    assert (
        "https://www.github.com/acme/acme-node#file-uploads" in files["src/_shims/web-runtime.ts"]
    )
    assert "require('acme/_shims/auto/runtime')" in files["src/_shims/index.js"]
    assert not any("ferrule" in text.lower() for text in files.values())


def test_lines_prettier_wraps_by_width() -> None:
    short = _render()
    assert "  if (value == null) throw new AcmeError(" in short["src/core.ts"]
    assert "    throw new Error(`can't" in short["src/_shims/registry.ts"]
    long = _render(client_name="AcmeExtraLong", package_name="@acme-extra/long-package")
    assert "  if (value == null)\n    throw new AcmeExtraLongError(" in long["src/core.ts"]
    assert "    throw new Error(\n      `can't" in long["src/_shims/registry.ts"]


def test_layouts_share_one_file() -> None:
    # Why the node layout is a template of its own: of its 36 files only the one-line
    # `src/resources.ts` re-export is also a file of the current layout's runtime.
    current = render_typescript_files(TypeScriptOptions(client_name="Acme"))
    node = _render()
    shared = sorted(p for p in set(current) & set(node) if current[p] == node[p])
    assert shared == ["src/resources.ts"]
    assert len(node) == 36


@pytest.mark.parametrize(
    ("kwargs", "match"),
    [
        ({"client_name": "1Acme"}, "client_name must be a JavaScript identifier"),
        ({"error_name": "Acme-Error"}, "error_name"),
        ({"package_name": "Acme SDK"}, "package_name must be an npm package name"),
        ({"repo": "acme"}, "repo must be a GitHub owner/name"),
    ],
)
def test_node_rejects_bad_options(kwargs: dict[str, str], match: str) -> None:
    with pytest.raises(RenderError, match=match):
        _render(**kwargs)
