# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the vendored runtime template into the files of one generated SDK.

The template in ``runtime/template`` is the runtime of a Stainless-generated SDK (Apache-2.0,
see ``runtime/NOTICE``) with names replaced by tokens and a few config-dependent spots wrapped
in ``#ferrule:if <flag>`` / ``#ferrule:else`` / ``#ferrule:endif`` directive lines. Rendering
substitutes the tokens and resolves the directives; the output is byte-identical to the files a
Stainless-generated SDK with the same settings ships.

Other languages (``runtime/<language>/template``) go through the same engine,
:func:`render_tree`, with their own comment syntax for directives (``//ferrule:if``).
"""

from __future__ import annotations

import keyword
import re
from dataclasses import dataclass
from importlib import resources
from pathlib import Path
from typing import TYPE_CHECKING, Any, Literal

from ferrule_runtime.pagination import PageScheme, render_pagination

if TYPE_CHECKING:
    from collections.abc import Callable, Iterable, Mapping, Sequence
    from importlib.resources.abc import Traversable

__all__ = [
    "Language",
    "RenderError",
    "RuntimeOptions",
    "default_client_name",
    "render",
    "render_files",
    "render_options",
    "render_tree",
    "template_root",
]

Language = Literal["python", "typescript", "typescript-node", "go", "java", "ruby"]

PKG_PLACEHOLDER = "__ferrule_pkg__"
CLIENT_PLACEHOLDER = "__FerruleClient__"
ERROR_PLACEHOLDER = "__FerruleError__"
LOG_ENV_PLACEHOLDER = "__FERRULE_LOG_ENV__"
TIMEOUT_PLACEHOLDER = "__FERRULE_TIMEOUT__"
TIMEOUT_COMMENT_PLACEHOLDER = "__FERRULE_TIMEOUT_COMMENT__"
MAX_RETRIES_PLACEHOLDER = "__FERRULE_MAX_RETRIES__"
FILE_UPLOADS_HINT_PLACEHOLDER = "__FERRULE_FILE_UPLOADS_HINT__"

_DIRECTIVE_OPS = r"ferrule:(if|else|endif|slot)(?: ([a-z0-9_]+))?$"
_FITS = "fits"  # ``if fits``: computed per block from the line length, never a caller flag
_LEFTOVER = re.compile(r"__(?:ferrule_[a-z_]+|Ferrule[A-Za-z]+|FERRULE_[A-Z_]+)__")
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
    """Adds the ``testing/`` fake server (``<pkg>.testing``); not in Stainless output (ADR 0016)."""


def template_root(language: Language = "python") -> Traversable:
    """Locate a runtime template (package data in a wheel, ``runtime/...`` in-repo).

    Python's is ``runtime/template``; every other language's is ``runtime/<language>/template``.
    """
    parts = ("template",) if language == "python" else (language, "template")
    packaged = resources.files("ferrule_runtime").joinpath(*parts)
    if packaged.is_dir():
        return packaged
    return Path(__file__).resolve().parents[2].joinpath(*parts)


def _walk(root: Traversable, prefix: str = "") -> Iterable[tuple[str, Traversable]]:
    for entry in sorted(root.iterdir(), key=lambda e: e.name):
        if entry.name in _NOT_TEMPLATE or entry.name.startswith("."):
            continue
        rel = f"{prefix}{entry.name}"
        if entry.is_dir():
            yield from _walk(entry, f"{rel}/")
        else:
            yield rel, entry


def _branch(  # noqa: PLR0913 - one directive line and the state it updates
    op: str,
    name: str | None,
    stack: list[tuple[bool, bool]],
    flags: Mapping[str, bool],
    *,
    path: str,
    marker: str,
) -> None:
    """Apply an ``if``/``else``/``endif`` directive to the ``(condition, in_else)`` stack."""
    if op == "if":
        if name is None or name not in flags:
            msg = f"{path}: unknown template flag {name!r}"
            raise RenderError(msg)
        stack.append((flags[name], False))
    elif not stack:
        msg = f"{path}: {marker}{op} without {marker}if"
        raise RenderError(msg)
    elif op == "else":
        stack[-1] = (stack[-1][0], True)
    else:
        stack.pop()


def _fits(
    lines: Sequence[str],
    index: int,
    fits: Callable[[str], bool] | None,
    *,
    path: str,
    marker: str,
) -> bool:
    """Evaluate ``if fits`` at ``lines[index]``: its branch must be one line, then else/endif."""
    candidate, closing = [*lines[index + 1 : index + 3], "", ""][:2]
    if (
        fits is None
        or closing.strip() not in (f"{marker}else", f"{marker}endif")
        or candidate.strip().startswith(marker)
    ):
        msg = f"{path}: {marker}if fits needs a line-length rule and a one-line branch"
        raise RenderError(msg)
    return fits(candidate.rstrip("\n"))


def _within(limit: int, substitute: Callable[[str], str]) -> Callable[[str], bool]:
    """``fits`` for one file: the line, with the tokens replaced, has at most ``limit`` columns."""
    return lambda line: len(substitute(line)) <= limit


def resolve_directives(  # noqa: PLR0913 - keyword-only settings of one file
    text: str,
    flags: Mapping[str, bool],
    path: str,
    *,
    comment: str = "#",
    slots: Mapping[str, str] | None = None,
    fits: Callable[[str], bool] | None = None,
) -> str:
    """Resolve whole-line ``<comment>ferrule:`` directives.

    ``if <flag>`` / ``else`` / ``endif`` keep or drop lines; ``slot <name>`` is replaced by
    ``slots[name]`` (text the caller generates, such as config-dependent Go client options).
    ``if fits`` holds when the one line after it fits the formatter's line length (``fits``,
    given that line); its ``else`` branch holds the formatter's wrapped form.
    """
    marker = f"{comment}ferrule:"
    directive = re.compile(re.escape(comment) + _DIRECTIVE_OPS)
    out: list[str] = []
    stack: list[tuple[bool, bool]] = []
    lines = text.splitlines(keepends=True)
    for index, line in enumerate(lines):
        stripped = line.strip()
        match = directive.match(stripped)
        active = all(cond != in_else for cond, in_else in stack)
        if match is None:
            if stripped.startswith(marker):
                msg = f"{path}: malformed directive {stripped!r}"
                raise RenderError(msg)
            if active:
                out.append(line)
            continue
        op, name = match.groups()
        if op == "if" and name == _FITS:
            stack.append((_fits(lines, index, fits, path=path, marker=marker), False))
            continue
        if op != "slot":
            _branch(op, name, stack, flags, path=path, marker=marker)
            continue
        if slots is None or name not in slots:
            msg = f"{path}: unknown template slot {name!r}"
            raise RenderError(msg)
        if active:
            out.append(slots[name])
    if stack:
        msg = f"{path}: unterminated {marker}if"
        raise RenderError(msg)
    return "".join(out)


def render_tree(  # noqa: PLR0913 - keyword-only settings of one render
    root: Traversable,
    *,
    replacements: Mapping[str, str],
    flags: Mapping[str, bool],
    comment: str = "#",
    slots: Mapping[str, str] | None = None,
    gated: Mapping[str, bool] | None = None,
    line_length: Callable[[str], int | None] | None = None,
) -> dict[str, str]:
    """Render every file under ``root``: the one code path all languages share (pure).

    Directives are resolved first, then tokens are replaced in ``replacements`` order (callers
    list the longest token first), in the text and in the path (a directory named
    ``__ferrule_pkg_path__`` becomes ``org/acme``). ``gated`` maps a template path prefix
    (``testing/``, before replacement) to whether files under it are rendered. A token left over
    after replacement is an error, and so are two templates whose paths render to one file
    (client ``BadRequest``: ``__FerruleClient__Exception.kt`` and ``BadRequestException.kt``).
    ``line_length`` gives a file's formatter line length (or ``None``) for ``if fits`` blocks,
    measured on the line with the tokens replaced.
    """

    def substitute(text: str) -> str:
        for token, value in replacements.items():
            text = text.replace(token, value)
        return text

    files: dict[str, str] = {}
    sources: dict[str, str] = {}
    for rel, entry in _walk(root):
        if any(rel.startswith(prefix) and not keep for prefix, keep in (gated or {}).items()):
            continue
        limit = None if line_length is None else line_length(rel)
        text = resolve_directives(
            entry.read_bytes().decode("utf-8"),
            flags,
            rel,
            comment=comment,
            slots=slots,
            fits=None if limit is None else _within(limit, substitute),
        )
        text = substitute(text)
        out = substitute(rel)
        for where in (out, text):
            leftover = _LEFTOVER.search(where)
            if leftover is not None:
                msg = f"{rel}: unknown template token {leftover.group(0)}"
                raise RenderError(msg)
        if out in sources:
            msg = (
                f"{rel} and {sources[out]} both render to {out}; choose names that keep "
                "the runtime's files apart"
            )
            raise RenderError(msg)
        sources[out] = rel
        files[out] = text
    return files


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

    files = render_tree(
        template_root(),
        replacements=replacements,
        flags=flags,
        gated={_TESTING_DIR: options.testing},
    )
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
