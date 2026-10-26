# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the PHP runtime (``runtime/php/template``) for one generated Composer package.

The template is the runtime of the Stainless-generated ImageKit PHP SDK: ``src/Core.php``,
``src/Core/**`` (attributes, concerns, contracts, conversion, exceptions, the HTTP
implementation and ``Util``), ``src/RequestOptions.php`` and the runtime's tests in
``tests/Core/``; see ``runtime/NOTICE``. Paths are relative to the package root.

The SDK's namespace and name are tokens, and so are the client settings
``RequestOptions`` defaults to. The files are php-cs-fixer's output, whose
``ordered_imports`` sorts each file's ``use`` lines; a namespace other than the template's
sorts differently against ``Psr`` and ``PHPUnit``, so the rendered ``use`` blocks are
sorted again by that rule.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from ferrule_runtime._render import RenderError, render_tree, template_root

__all__ = ["PhpOptions", "render_php_files"]

NAMESPACE_PLACEHOLDER = "__FerruleNs__"
NAME_PLACEHOLDER = "__FerruleName__"
TIMEOUT_PLACEHOLDER = "__FERRULE_TIMEOUT__"
MAX_RETRIES_PLACEHOLDER = "__FERRULE_MAX_RETRIES__"
INITIAL_RETRY_DELAY_PLACEHOLDER = "__FERRULE_INITIAL_RETRY_DELAY__"
MAX_RETRY_DELAY_PLACEHOLDER = "__FERRULE_MAX_RETRY_DELAY__"

_SEGMENT = r"[A-Z][A-Za-z0-9]*"
_NAMESPACE = re.compile(rf"^{_SEGMENT}(?:\\{_SEGMENT})*$")
_NAME = re.compile(rf"^{_SEGMENT}$")
_USE = re.compile(r"^use [^;]+;\n$")


@dataclass(frozen=True, kw_only=True)
class PhpOptions:
    """Settings that vary the PHP runtime between generated packages."""

    namespace: str
    """The package's root namespace (``Imagekit``), its PSR-4 prefix for ``src/``."""
    name: str | None = None
    """The SDK's name in the base exception (``ImagekitException``) and the error
    descriptions (``'Imagekit Error'``). Defaults to the namespace's last segment."""
    timeout: float = 60
    """``client_settings.timeout`` in seconds: ``RequestOptions::$timeout``."""
    max_retries: int = 2
    initial_retry_delay: float = 0.5
    """``client_settings.retries.initial_delay_seconds``."""
    max_retry_delay: float = 8.0
    """``client_settings.retries.max_delay_seconds``."""


def _check(options: PhpOptions) -> None:
    if not _NAMESPACE.match(options.namespace):
        msg = f"namespace must be a PHP namespace of PascalCase segments, got {options.namespace!r}"
        raise RenderError(msg)
    if options.name is not None and not _NAME.match(options.name):
        msg = f"name must be a PHP class-name prefix, got {options.name!r}"
        raise RenderError(msg)
    if options.timeout <= 0 or options.max_retries < 0:
        msg = "timeout must be positive and max_retries non-negative"
        raise RenderError(msg)
    if options.initial_retry_delay < 0 or options.max_retry_delay < 0:
        msg = "retry delays must be non-negative"
        raise RenderError(msg)


def _number(value: float) -> str:
    """Write ``RequestOptions::$timeout``: an integer as written (``60``), else ``0.5``."""
    return str(int(value)) if float(value).is_integer() else repr(float(value))


def _float(value: float) -> str:
    """Write a retry delay as the oracle does: a ``float`` literal (``8.0``, ``0.5``)."""
    return repr(float(value))


def _sort_key(line: str) -> str:
    """php-cs-fixer ``ordered_imports`` (``alpha``): backslashes as spaces, ``strcasecmp``."""
    return line.removeprefix("use ").removesuffix(";\n").replace("\\", " ").lower()


def sort_uses(text: str) -> str:
    """Sort each block of consecutive top-level ``use`` lines as php-cs-fixer does."""
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    block: list[str] = []
    for line in [*lines, ""]:
        if _USE.match(line):
            block.append(line)
            continue
        out.extend(sorted(block, key=_sort_key))
        block = []
        out.append(line)
    return "".join(out)


def render_php_files(options: PhpOptions) -> dict[str, str]:
    """Render the PHP runtime (pure): ``{path from the package root: exact text}``."""
    _check(options)
    name = options.name or options.namespace.rsplit("\\", 1)[-1]
    files = render_tree(
        template_root("php"),
        comment="//",
        replacements={
            INITIAL_RETRY_DELAY_PLACEHOLDER: _float(options.initial_retry_delay),
            MAX_RETRY_DELAY_PLACEHOLDER: _float(options.max_retry_delay),
            MAX_RETRIES_PLACEHOLDER: str(options.max_retries),
            TIMEOUT_PLACEHOLDER: _number(options.timeout),
            NAMESPACE_PLACEHOLDER: options.namespace,
            NAME_PLACEHOLDER: name,
        },
        flags={},
    )
    return {rel: sort_uses(text) for rel, text in files.items()}
