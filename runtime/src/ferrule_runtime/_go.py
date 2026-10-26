# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the Go runtime for one generated SDK, in Stainless's Go v1 or v2 layout.

v1 (``runtime/go/template``) is the runtime of the Stainless-generated OneBusAway Go SDK; v2
(``runtime/go-v2/template``, ``targets.go.options.enable_v2``) is imagekit-go's. Each is
``internal/**`` (without ``internal/version.go``), ``option/**``, ``field.go`` and
``default_http_client.go``, and v2 adds ``packages/param`` and ``packages/respjson``; see
``runtime/NOTICE``. Paths are relative to the module root.

Two runtime files carry config-dependent code that the plugin generates: the client settings
in ``internal/requestconfig/requestconfig.go`` and the environment and client-setting options
at the end of ``option/requestoption.go``. They are template slots, filled with the caller's
gofmt-formatted text.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Literal

from ferrule_runtime._render import RenderError, render_tree, template_root

__all__ = ["GoOptions", "render_go_files"]

MODULE_PLACEHOLDER = "__ferrule_module__"
DOCS_MODULE_PLACEHOLDER = "__ferrule_docs_module__"
PKG_PLACEHOLDER = "__ferrule_pkg__"
CLIENT_PLACEHOLDER = "__FerruleClient__"
ORG_PLACEHOLDER = "__FERRULE_ORG__"
MAX_RETRIES_PLACEHOLDER = "__FERRULE_MAX_RETRIES__"
SENSITIVE_HEADERS_PLACEHOLDER = "__FERRULE_SENSITIVE_HEADERS__"

_MODULE = re.compile(r"^[A-Za-z0-9.~_-]+(?:/[A-Za-z0-9.~_-]+)*$")
_HEADER = re.compile(r"^[a-z0-9!#$%&*+.^_`|~-]+$")
_GO_IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
_GO_KEYWORDS = frozenset(
    {
        "break", "case", "chan", "const", "continue", "default", "defer", "else",
        "fallthrough", "for", "func", "go", "goto", "if", "import", "interface", "map",
        "package", "range", "return", "select", "struct", "switch", "type", "var",
    }
)  # fmt: skip
_IMPORT_SPEC = re.compile(r'^\t(?:[A-Za-z_.][A-Za-z0-9_]* )?"([^"]+)"(?: //.*)?$')
_SLOTS = ("config_fields", "config_clone", "client_options")
_REQUEST_CONFIG = "internal/requestconfig/requestconfig.go"
# The runtime lines ending the runs `config_fields` and `config_clone` extend.
_REALIGNED = ("\tMiddlewares ", "\t\tMiddlewares: ")
_ALIGNED = re.compile(r"^(\t+)([A-Za-z_][A-Za-z0-9_]*:?) +(\S.*)$")
_MAJOR_SUFFIX = re.compile(r"/v[0-9]+$")
GoLayout = Literal["v1", "v2"]
_LAYOUTS: tuple[GoLayout, ...] = ("v1", "v2")


@dataclass(frozen=True, kw_only=True)
class GoOptions:
    """Settings that vary the Go runtime between generated SDKs."""

    module_path: str
    """Go module path (``github.com/OneBusAway/go-sdk``); every internal import uses it."""
    package_name: str
    """Root package (``onebusaway``) of ``field.go`` and ``default_http_client.go``."""
    client_name: str
    """User-Agent product (``OnebusawaySDK`` sends ``OnebusawaySDK/Go <version>``)."""
    org_name: str | None = None
    """``organization.name`` (``onebusaway-sdk``), named in the ``RequestOption`` doc comment.
    Defaults to ``package_name``."""
    max_retries: int = 2
    redacted_headers: tuple[str, ...] = ("api-key", "x-api-key")
    """Headers ``option.WithDebugLog`` masks besides ``authorization`` and cookies (as in the
    TypeScript runtime's ``log.ts``)."""
    config_fields: str = ""
    """``RequestConfig`` struct fields for client settings (``\\tAPIKey         *string\\n``)."""
    config_clone: str = ""
    """The same fields in ``RequestConfig.Clone`` (``\\t\\tAPIKey:         cfg.APIKey,\\n``)."""
    client_options: str = ""
    """Appended to ``option/requestoption.go``: ``WithEnvironment*`` and client-setting options,
    starting with the blank line that separates them from ``WithRequestTimeout``. In v2, a
    ``base64.`` in it imports ``encoding/base64`` (HTTP Basic auth)."""
    layout: GoLayout = "v1"
    """``v2`` for ``targets.go.options.enable_v2`` (``packages/param``, ``packages/respjson``)."""
    new_file: bool = False
    """v2 only: name ``field.go``'s file helper ``NewFile`` instead of ``File``, as Stainless does
    when the root package declares a type ``File`` (imagekit-go, ogx-client-go)."""


def _check_layout(options: GoOptions) -> None:
    if options.layout not in _LAYOUTS:
        msg = f"layout must be one of {_LAYOUTS}, got {options.layout!r}"
        raise RenderError(msg)
    if options.new_file and options.layout != "v2":
        msg = "new_file applies to the v2 layout only"
        raise RenderError(msg)


def _check(options: GoOptions) -> None:
    _check_layout(options)
    if not _MODULE.match(options.module_path):
        msg = f"module_path must be a Go module path, got {options.module_path!r}"
        raise RenderError(msg)
    for kind, value in (
        ("package_name", options.package_name),
        ("client_name", options.client_name),
    ):
        if not _GO_IDENT.match(value) or value in _GO_KEYWORDS:
            msg = f"{kind} must be a Go identifier, got {value!r}"
            raise RenderError(msg)
    if options.max_retries < 0:
        msg = "max_retries must be non-negative"
        raise RenderError(msg)
    for header in options.redacted_headers:
        if not _HEADER.match(header):
            msg = f"redacted_headers must be lower-case header names, got {header!r}"
            raise RenderError(msg)
    for name in _SLOTS:
        text: str = getattr(options, name)
        if text and not text.endswith("\n"):
            msg = f"{name} must be empty or end with a newline"
            raise RenderError(msg)


def _sensitive_headers(redacted: tuple[str, ...]) -> str:
    names = ("authorization", *redacted, "cookie", "set-cookie")
    return ", ".join(f'"{name}"' for name in names)


def _realign_run(text: str, anchor: str, *, path: str) -> str:
    """Re-pad the gofmt alignment run that holds the line starting with ``anchor``.

    A run is the consecutive ``<name> <value>`` (or ``<key>: <value>``) lines at one indent;
    gofmt pads every name in it to the longest plus one. A client setting slotted into a run can
    be longer than the runtime's own names, so the whole run is padded again.
    """
    lines = text.split("\n")
    at = next((n for n, line in enumerate(lines) if line.startswith(anchor)), None)
    anchored = _ALIGNED.match(lines[at]) if at is not None else None
    if at is None or anchored is None:
        msg = f"{path}: no alignment run at {anchor!r}"
        raise RenderError(msg)
    indent = anchored.group(1)

    def in_run(line: str) -> bool:
        match = _ALIGNED.match(line)
        return match is not None and match.group(1) == indent

    start, end = at, at
    while start > 0 and in_run(lines[start - 1]):
        start -= 1
    while end + 1 < len(lines) and in_run(lines[end + 1]):
        end += 1
    run = [m for m in map(_ALIGNED.match, lines[start : end + 1]) if m is not None]
    width = max(len(m.group(2)) for m in run) + 1
    lines[start : end + 1] = [f"{m.group(1)}{m.group(2):<{width}}{m.group(3)}" for m in run]
    return "\n".join(lines)


def _sort_import_groups(text: str) -> str:
    """Sort each blank-line-separated group of ``import ( ... )`` blocks by path, as gofmt does.

    The template's groups are in order for its own module path; the module's packages move
    within a group when another module path sorts differently against the group's other
    imports. A group with anything but import specs (a comment line) is left as it is.
    """
    lines = text.split("\n")
    out: list[str] = []
    group: list[str] = []

    def flush() -> None:
        paths = [m.group(1) for m in map(_IMPORT_SPEC.match, group) if m is not None]
        if len(paths) == len(group):
            out.extend(line for _, line in sorted(zip(paths, group, strict=True)))
        else:
            out.extend(group)
        group.clear()

    inside = False
    for line in lines:
        if inside and line in {"", ")"}:
            flush()
            inside = line == ""
            out.append(line)
        elif inside:
            group.append(line)
        else:
            inside = line == "import ("
            out.append(line)
    flush()
    return "\n".join(out)


def render_go_files(options: GoOptions) -> dict[str, str]:
    """Render the Go runtime (pure): ``{path from the module root: exact text}``."""
    _check(options)
    module = options.module_path
    v2 = options.layout == "v2"
    # field.go imports the module's `param` package beside `io`.
    param = f"{module}/packages/param" if v2 else f"{module}/internal/param"
    files = render_tree(
        template_root("go-v2" if v2 else "go"),
        comment="//",
        replacements={
            MAX_RETRIES_PLACEHOLDER: str(options.max_retries),
            # pkg.go.dev's README anchors name the module without its `/vN` (imagekit-go/v2).
            DOCS_MODULE_PLACEHOLDER: _MAJOR_SUFFIX.sub("", module),
            MODULE_PLACEHOLDER: module,
            CLIENT_PLACEHOLDER: options.client_name,
            SENSITIVE_HEADERS_PLACEHOLDER: _sensitive_headers(options.redacted_headers),
            ORG_PLACEHOLDER: options.org_name or options.package_name,
            PKG_PLACEHOLDER: options.package_name,
        },
        # gofmt sorts the imports of a group by path; these two groups mix the module's own
        # packages with another import (every other group is sorted after rendering).
        flags={
            "module_before_io": param < "io",
            "module_before_sjson": f"{module}/internal/requestconfig" < "github.com/tidwall/sjson",
            "base64": "base64." in options.client_options,
            "new_file": options.new_file,
        },
        slots={name: getattr(options, name) for name in _SLOTS},
    )
    # The client-setting slots sit in an alignment run of the struct and of its Clone literal.
    config = files[_REQUEST_CONFIG]
    for anchor in _REALIGNED:
        config = _realign_run(config, anchor, path=_REQUEST_CONFIG)
    files[_REQUEST_CONFIG] = config
    return {
        path: _sort_import_groups(text) if path.endswith(".go") else text
        for path, text in files.items()
    }
