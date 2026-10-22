# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Unit tests for render_files() and render_pagination() that need no oracle cache."""

from __future__ import annotations

import ast
from typing import TYPE_CHECKING

import pytest

from ferrule_runtime import (
    PageField,
    PageScheme,
    PaginationError,
    RenderError,
    RuntimeOptions,
    default_client_name,
    render,
    render_files,
    render_options,
    render_pagination,
    template_root,
)
from ferrule_runtime._render import resolve_directives, timeout_comment

if TYPE_CHECKING:
    from pathlib import Path

ITEMS = "List[_T]"


def test_render_is_deterministic_and_parses() -> None:
    first = render_files("acme", "Acme")
    assert first == render_files("acme", "Acme")
    for rel, text in first.items():
        if rel.endswith(".py"):
            ast.parse(text, filename=rel)
        assert "__ferrule" not in text
        assert "__Ferrule" not in text
        assert "__FERRULE" not in text


def test_render_substitutes_names() -> None:
    files = render_files("acme_sdk", "AcmeAPI")
    assert "class AcmeAPIError(Exception):" in files["_exceptions.py"]
    assert 'os.environ.get("ACME_API_LOG")' in files["_utils/_logs.py"]
    assert 'logging.getLogger("acme_sdk")' in files["_utils/_logs.py"]
    assert "from ._client import AcmeAPI, AsyncAcmeAPI" in files["_resource.py"]
    assert "pagination.py" not in files


def test_render_options() -> None:
    files = render_files(
        "acme",
        "Acme",
        error_name="AcmeBaseError",
        log_env_var="ACME_DEBUG",
        repo_url="https://github.com/acme/acme-python/",
        default_branch="trunk",
        timeout=900,
        max_retries=5,
        min_python=(3, 10),
        webhooks=True,
    )
    assert "class AcmeBaseError(Exception):" in files["_exceptions.py"]
    assert "class APIWebhookValidationError(APIError):" in files["_exceptions.py"]
    assert 'os.environ.get("ACME_DEBUG")' in files["_utils/_logs.py"]
    assert "See https://github.com/acme/acme-python/tree/trunk#file-uploads" in files["_files.py"]
    constants = files["_constants.py"]
    assert "# default timeout is 15 minutes\n" in constants
    assert "httpx.Timeout(timeout=900, connect=5.0)" in constants
    assert "DEFAULT_MAX_RETRIES = 5\n" in constants
    assert "sys.version_info" not in files["_utils/_compat.py"]


def test_render_defaults() -> None:
    files = render_files("acme", "Acme")
    assert "APIWebhookValidationError" not in files["_exceptions.py"]
    assert "sys.version_info < (3, 10)" in files["_utils/_compat.py"]
    assert "# default timeout is 1 minute\n" in files["_constants.py"]
    assert "DEFAULT_MAX_RETRIES = 2\n" in files["_constants.py"]
    assert 'instead."\n' in files["_files.py"]


def test_long_package_name_wraps_base_model_hint() -> None:
    short = render_files("acme", "Acme")["_response.py"]
    long_ = render_files("a" * 40, "Acme")["_response.py"]
    assert 'raise TypeError("Pydantic models' in short
    assert 'raise TypeError(\n                "Pydantic models' in long_


def test_render_includes_pagination() -> None:
    scheme = PageScheme(
        name="Page", kind="fake_page", items="data", fields=(PageField("data", ITEMS),)
    )
    files = render_files("acme", "Acme", pagination=[scheme])
    assert "class SyncPage(" in files["pagination.py"]


@pytest.mark.parametrize(
    ("kwargs", "match"),
    [
        ({"package_name": "acme-sdk", "client_name": "Acme"}, "package_name"),
        ({"package_name": "class", "client_name": "Acme"}, "package_name"),
        ({"package_name": "acme", "client_name": "1Acme"}, "client_name"),
        ({"package_name": "acme", "client_name": "Acme", "error_name": "Bad-Error"}, "error_name"),
        ({"package_name": "acme", "client_name": "Acme", "timeout": 0}, "timeout"),
        ({"package_name": "acme", "client_name": "Acme", "max_retries": -1}, "max_retries"),
    ],
)
def test_render_rejects_bad_options(kwargs: dict[str, object], match: str) -> None:
    with pytest.raises(RenderError, match=match):
        render_options(RuntimeOptions(**kwargs))  # type: ignore[arg-type]


def test_render_function_is_exported() -> None:
    assert default_client_name("acme_sdk") == "AcmeSdk"
    assert template_root().joinpath("_base_client.py").is_file()


def testtimeout_comment() -> None:
    assert timeout_comment(60) == "1 minute"
    assert timeout_comment(600) == "10 minutes"
    assert timeout_comment(90) == "90 seconds"


def test_directives_nest_and_else() -> None:
    text = (
        "a\n#ferrule:if x\nb\n#ferrule:if y\nc\n#ferrule:else\nd\n#ferrule:endif\n"
        "#ferrule:else\ne\n#ferrule:endif\nf\n"
    )
    assert resolve_directives(text, {"x": True, "y": False}, "t") == "a\nb\nd\nf\n"
    assert resolve_directives(text, {"x": False, "y": True}, "t") == "a\ne\nf\n"


@pytest.mark.parametrize(
    ("text", "match"),
    [
        ("#ferrule:if nope\n#ferrule:endif\n", "unknown template flag"),
        ("#ferrule:else\n", "without #ferrule:if"),
        ("#ferrule:endif\n", "without #ferrule:if"),
        ("#ferrule:if x\n", "unterminated"),
    ],
)
def test_directive_errors(text: str, match: str) -> None:
    with pytest.raises(RenderError, match=match):
        resolve_directives(text, {"x": True}, "t")


def test_unknown_token_is_an_error(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    (tmp_path / "x.py").write_text("__FERRULE_NOPE__\n")
    (tmp_path / "__pycache__").mkdir()
    monkeypatch.setattr("ferrule_runtime._render.template_root", lambda: tmp_path)
    with pytest.raises(RenderError, match="__FERRULE_NOPE__"):
        render_files("acme", "Acme")


def test_pagination_requires_schemes() -> None:
    with pytest.raises(PaginationError, match="at least one"):
        render_pagination([])


@pytest.mark.parametrize(
    ("scheme", "match"),
    [
        (
            PageScheme(name="P", kind="cursor", items="data", fields=(PageField("data", ITEMS),)),
            "param, path",
        ),
        (PageScheme(name="P", kind="fake_page", items="data", fields=()), "items field"),
        (
            PageScheme(
                name="P",
                kind="fake_page",
                items="data",
                fields=(PageField("data", ITEMS),),
                has_more="more",
            ),
            "has_more field",
        ),
    ],
)
def test_pagination_validation(scheme: PageScheme, match: str) -> None:
    with pytest.raises(PaginationError, match=match):
        render_pagination([scheme])


def test_pagination_wraps_long_lines() -> None:
    schemes = [
        PageScheme(
            name=f"VeryLongPaginationName{i}",
            kind="fake_page",
            items="data",
            fields=(PageField("data", ITEMS),),
        )
        for i in range(4)
    ]
    text = render_pagination(schemes, line_length=60)
    assert '__all__ = [\n    "SyncVeryLongPaginationName0",\n' in text
    assert "from typing import (\n    List,\n" in render_pagination(schemes, line_length=30)
    ast.parse(text)


def test_pagination_page_number_without_total() -> None:
    scheme = PageScheme(
        name="Page",
        kind="page_number",
        items="items",
        fields=(PageField("items", ITEMS), PageField("page", "Optional[int]", optional=True)),
        param="page",
        path=("page",),
    )
    text = render_pagination([scheme])
    assert "total_pages" not in text
    assert 'return PageInfo(params={"page": current_page + 1})' in text


def test_render_writes_package_dir(tmp_path: Path) -> None:
    written = render("acme", tmp_path / "src" / "acme")
    assert (tmp_path / "src/acme/_utils/_logs.py").is_file()
    assert len(written) == len(render_files("acme", "Acme"))
    assert "class AcmeError(Exception)" in (tmp_path / "src/acme/_exceptions.py").read_text()
    render("acme", tmp_path / "b", client_name="Zed", timeout=900)
    assert "class ZedError" in (tmp_path / "b/_exceptions.py").read_text()
