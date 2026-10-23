# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the vendored runtime template into the files of one generated SDK.

The template in ``runtime/template`` is the runtime of a Stainless-generated SDK (Apache-2.0,
see ``runtime/NOTICE``) with names replaced by tokens and a few config-dependent spots wrapped
in ``#ferrule:if <flag>`` / ``#ferrule:else`` / ``#ferrule:endif`` directive lines. Rendering
substitutes the tokens and resolves the directives; the output is byte-identical to the files a
Stainless-generated SDK with the same settings ships.
"""

from __future__ import annotations

import keyword
import re
from dataclasses import dataclass
from importlib import resources
from pathlib import Path
from typing import TYPE_CHECKING, Any

from ferrule_runtime.pagination import PageScheme, render_pagination

if TYPE_CHECKING:
    from collections.abc import Iterable, Mapping, Sequence
    from importlib.resources.abc import Traversable

__all__ = [
    "RenderError",
    "RuntimeOptions",
    "default_client_name",
    "render",
    "render_files",
    "render_options",
    "template_root",
]

PKG_PLACEHOLDER = "__ferrule_pkg__"
CLIENT_PLACEHOLDER = "__FerruleClient__"
ERROR_PLACEHOLDER = "__FerruleError__"
LOG_ENV_PLACEHOLDER = "__FERRULE_LOG_ENV__"
TIMEOUT_PLACEHOLDER = "__FERRULE_TIMEOUT__"
TIMEOUT_COMMENT_PLACEHOLDER = "__FERRULE_TIMEOUT_COMMENT__"
MAX_RETRIES_PLACEHOLDER = "__FERRULE_MAX_RETRIES__"
FILE_UPLOADS_HINT_PLACEHOLDER = "__FERRULE_FILE_UPLOADS_HINT__"

_DIRECTIVE = re.compile(r"^#ferrule:(if|else|endif)(?: ([a-z0-9_]+))?$")
_LEFTOVER = re.compile(r"__(?:ferrule_pkg|Ferrule[A-Za-z]+|FERRULE_[A-Z_]+)__")
_CAMEL_BOUNDARY = re.compile(r"(?<=[a-z0-9])(?=[A-Z])")
_IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
_BASE_MODEL_HINT = (
    '            raise TypeError("Pydantic models must subclass our base model type, '
    'e.g. `from {pkg} import BaseModel`")'
)
_TESTING_DIR = "testing/"
_NOT_TEMPLATE = frozenset({"__pycache__", "ruff.toml"})  # ruff.toml: lint opt-out, not shipped
_SECONDS_PER_MINUTE = 60
_PY310 = (3, 10)


class RenderError(ValueError):
    """Raised for invalid render options or a malformed template."""


@dataclass(frozen=True, kw_only=True)
class RuntimeOptions:
    """Settings that vary the runtime between generated SDKs.

    Defaults match a Stainless-generated SDK with no client-settings overrides.
    """

    package_name: str
    client_name: str
    error_name: str | None = None
    """Base exception class; Stainless uses ``<ClientName>Error``."""
    log_env_var: str | None = None
    """Environment variable read by ``_utils/_logs.py``: ``<ENV_PREFIX>_LOG``.

    Defaults to the client name in SCREAMING_SNAKE_CASE
    (``OnebusawaySDK`` -> ``ONEBUSAWAY_SDK_LOG``);
    pass it explicitly when the config's env prefix differs (``LumaAI`` uses ``LUMAAI_LOG``).
    """
    repo_url: str | None = None
    """GitHub URL of the SDK; when set, ``_files.py`` links to its README file-upload section."""
    default_branch: str = "main"
    timeout: int = 60
    max_retries: int = 2
    min_python: tuple[int, int] = (3, 9)
    webhooks: bool = False
    """Adds ``APIWebhookValidationError`` (SDKs that configure webhook unwrapping)."""
    line_length: int = 120
    testing: bool = False
    """Adds the ``testing/`` fake server (``<pkg>.testing``); not in Stainless output (ADR 0013)."""


def template_root() -> Traversable:
    """Locate the runtime template (package data in a wheel, ``runtime/template`` in-repo)."""
    packaged = resources.files("ferrule_runtime").joinpath("template")
    if packaged.is_dir():
        return packaged
    return Path(__file__).resolve().parents[2] / "template"


def _walk(root: Traversable, prefix: str = "") -> Iterable[tuple[str, Traversable]]:
    for entry in sorted(root.iterdir(), key=lambda e: e.name):
        if entry.name in _NOT_TEMPLATE or entry.name.startswith("."):
            continue
        rel = f"{prefix}{entry.name}"
        if entry.is_dir():
            yield from _walk(entry, f"{rel}/")
        else:
            yield rel, entry


def resolve_directives(text: str, flags: Mapping[str, bool], path: str) -> str:
    out: list[str] = []
    stack: list[tuple[bool, bool]] = []  # (condition, in_else)
    for line in text.splitlines(keepends=True):
        match = _DIRECTIVE.match(line.strip())
        if match is None:
            if all(cond != in_else for cond, in_else in stack):
                out.append(line)
            continue
        op, flag = match.groups()
        if op == "if":
            if flag not in flags:
                msg = f"{path}: unknown template flag {flag!r}"
                raise RenderError(msg)
            stack.append((flags[flag], False))
        elif not stack:
            msg = f"{path}: #ferrule:{op} without #ferrule:if"
            raise RenderError(msg)
        elif op == "else":
            stack[-1] = (stack[-1][0], True)
        else:
            stack.pop()
    if stack:
        msg = f"{path}: unterminated #ferrule:if"
        raise RenderError(msg)
    return "".join(out)


def timeout_comment(seconds: int) -> str:
    if seconds % _SECONDS_PER_MINUTE == 0:
        minutes = seconds // _SECONDS_PER_MINUTE
        return "1 minute" if minutes == 1 else f"{minutes} minutes"
    return f"{seconds} seconds"


def _screaming_snake(name: str) -> str:
    """``OnebusawaySDK`` -> ``ONEBUSAWAY_SDK``, ``ImageKit`` -> ``IMAGE_KIT``."""
    return _CAMEL_BOUNDARY.sub("_", name).upper()


def _check_identifier(kind: str, value: str) -> None:
    if not _IDENT.match(value) or keyword.iskeyword(value):
        msg = f"{kind} must be a Python identifier, got {value!r}"
        raise RenderError(msg)


def render_files(  # noqa: PLR0913 - mirrors RuntimeOptions; keyword-only
    package_name: str,
    client_name: str,
    *,
    error_name: str | None = None,
    log_env_var: str | None = None,
    repo_url: str | None = None,
    default_branch: str = "main",
    timeout: int = 60,
    max_retries: int = 2,
    min_python: tuple[int, int] = (3, 9),
    webhooks: bool = False,
    line_length: int = 120,
    testing: bool = False,
    pagination: Sequence[PageScheme] = (),
) -> dict[str, str]:
    """Render the runtime for one SDK (pure).

    Returns a mapping from a path relative to the SDK's package directory (for example
    ``_utils/_logs.py``) to the exact file text. ``pagination.py`` is included when
    ``pagination`` schemes are given, and ``testing/`` (the fake server) when ``testing`` is set.
    :func:`render` writes these files to disk.
    """
    return render_options(
        RuntimeOptions(
            package_name=package_name,
            client_name=client_name,
            error_name=error_name,
            log_env_var=log_env_var,
            repo_url=repo_url,
            default_branch=default_branch,
            timeout=timeout,
            max_retries=max_retries,
            min_python=min_python,
            webhooks=webhooks,
            line_length=line_length,
            testing=testing,
        ),
        pagination=pagination,
    )


def render_options(
    options: RuntimeOptions, *, pagination: Sequence[PageScheme] = ()
) -> dict[str, str]:
    """Render the runtime from a :class:`RuntimeOptions`; see :func:`render_files`."""
    pkg = options.package_name
    error_name = options.error_name or f"{options.client_name}Error"
    for kind, value in (
        ("package_name", pkg),
        ("client_name", options.client_name),
        ("error_name", error_name),
    ):
        _check_identifier(kind, value)
    if options.timeout <= 0 or options.max_retries < 0:
        msg = "timeout must be positive and max_retries non-negative"
        raise RenderError(msg)

    hint = (
        f" See {options.repo_url.rstrip('/')}/tree/{options.default_branch}#file-uploads"
        if options.repo_url
        else ""
    )
    replacements = {
        # Longest first so no token is a prefix of a replaced value.
        FILE_UPLOADS_HINT_PLACEHOLDER: hint,
        TIMEOUT_COMMENT_PLACEHOLDER: timeout_comment(options.timeout),
        MAX_RETRIES_PLACEHOLDER: str(options.max_retries),
        TIMEOUT_PLACEHOLDER: str(options.timeout),
        LOG_ENV_PLACEHOLDER: options.log_env_var or f"{_screaming_snake(options.client_name)}_LOG",
        CLIENT_PLACEHOLDER: options.client_name,
        ERROR_PLACEHOLDER: error_name,
        PKG_PLACEHOLDER: pkg,
    }
    flags = {
        "py39": options.min_python < _PY310,
        "webhooks": options.webhooks,
        "base_model_hint_fits": len(_BASE_MODEL_HINT.format(pkg=pkg)) <= options.line_length,
    }

    files: dict[str, str] = {}
    for rel, entry in _walk(template_root()):
        if rel.startswith(_TESTING_DIR) and not options.testing:
            continue
        text = resolve_directives(entry.read_bytes().decode("utf-8"), flags, rel)
        for token, value in replacements.items():
            text = text.replace(token, value)
        leftover = _LEFTOVER.search(text)
        if leftover is not None:
            msg = f"{rel}: unknown template token {leftover.group(0)}"
            raise RenderError(msg)
        files[rel] = text
    if pagination:
        files["pagination.py"] = render_pagination(pagination, line_length=options.line_length)
    return files


def default_client_name(package: str) -> str:
    """``acme_sdk`` -> ``AcmeSDK``-style fallback: PascalCase of the package name."""
    return "".join(part[:1].upper() + part[1:] for part in package.split("_") if part)


def render(
    package: str,
    package_dir: Path,
    /,
    *,
    client_name: str | None = None,
    pagination: Sequence[PageScheme] = (),
    **options: Any,  # noqa: ANN401 - forwarded to render_files keyword options
) -> list[Path]:
    """Write the runtime for ``package`` into ``package_dir`` (``<out>/src/<package>``).

    This is the entry point ``ferrule generate`` calls. ``client_name`` defaults to
    :func:`default_client_name`; other keyword options are those of :func:`render_files`.
    Returns the written paths in render order.
    """
    files = render_files(
        package, client_name or default_client_name(package), pagination=pagination, **options
    )
    written: list[Path] = []
    for rel, text in files.items():
        path = package_dir / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(text.encode("utf-8"))
        written.append(path)
    return written
