# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Rendering the TypeScript runtime: options and their effect on the template."""

from __future__ import annotations

from typing import Any

import pytest

from ferrule_runtime import (
    RenderError,
    TypeScriptOptions,
    render_typescript_files,
    template_root,
)


def test_template_root_exists() -> None:
    assert template_root("typescript").joinpath("src", "internal", "shims.ts").is_file()


def test_typescript_defaults() -> None:
    files = render_typescript_files(TypeScriptOptions(client_name="Acme"))
    assert "src/internal/qs/stringify.ts" not in files
    assert "throw new AcmeError(" in files["src/internal/utils/query.ts"]
    assert "PagePromise" not in files["src/index.ts"]
    assert "   * @default 2\n" in files["src/internal/request-options.ts"]
    assert "=== 'x-api-key' ||" in files["src/internal/utils/log.ts"]
    assert not any("ferrule" in text.lower() for text in files.values())


def test_typescript_options() -> None:
    files = render_typescript_files(
        TypeScriptOptions(
            client_name="Acme",
            error_name="AcmeAPIError",
            max_retries=5,
            query_array_format="repeat",
            pagination=True,
            redacted_headers=("x-acme-key",),
        )
    )
    assert "src/internal/qs/stringify.ts" in files
    assert "{ arrayFormat: 'repeat' }" in files["src/internal/utils/query.ts"]
    assert "export { PagePromise } from './core/pagination';\n" in files["src/index.ts"]
    assert "AcmeAPIError" in files["src/core/error.ts"]
    assert "   * @default 5\n" in files["src/internal/request-options.ts"]
    log = files["src/internal/utils/log.ts"]
    assert "=== 'x-acme-key' ||" in log
    assert "x-api-key" not in log


@pytest.mark.parametrize(
    ("kwargs", "match"),
    [
        ({"client_name": "1Acme"}, "client_name must be a JavaScript identifier"),
        ({"client_name": "Acme", "error_name": "Acme-Error"}, "error_name"),
        ({"client_name": "Acme", "max_retries": -1}, "max_retries"),
        ({"client_name": "Acme", "query_array_format": "dots"}, "query_array_format"),
        ({"client_name": "Acme", "redacted_headers": ("X-Key",)}, "lower-case header names"),
    ],
)
def test_typescript_rejects_bad_options(kwargs: dict[str, Any], match: str) -> None:
    with pytest.raises(RenderError, match=match):
        render_typescript_files(TypeScriptOptions(**kwargs))
