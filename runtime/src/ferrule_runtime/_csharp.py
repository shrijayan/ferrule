# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Render the C# runtime (``runtime/csharp/template``) for one generated .NET SDK.

The template is the runtime of the Stainless-generated imagekit-dotnet SDK (Apache-2.0, see
``runtime/NOTICE``): ``src/<Ns>/Core/**`` (but the generated ``ClientOptions``,
``EnvironmentUrl`` and ``ModelBase``), ``src/<Ns>/Exceptions/**``, ``IPageExtensions.cs``,
``Shims.cs``, and the tests ``src/<Ns>.Tests/Core/**`` and ``RetriesTest.cs``. Paths are
relative to the SDK root.

``ParamsBase.AddDefaultHeaders`` sends the client's credentials, which depend on the config's
client options and security schemes: that block is the ``auth_headers`` slot the plugin fills.
The SDK's ``.editorconfig`` asks for CRLF line endings in ``.cs`` files, so they render with
CRLF, byte for byte what Stainless ships. Lines are laid out for the oracle's names; ``ferrule
generate`` runs csharpier over the whole SDK afterwards, as the SDK's ``scripts/format`` does,
so a longer name wraps as csharpier wraps it.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from ferrule_runtime._render import RenderError, render_tree, template_root

__all__ = ["CSharpOptions", "render_csharp_files"]

NS_PLACEHOLDER = "__FerruleNs__"
CLIENT_PLACEHOLDER = "__FerruleClient__"
ENV_PLACEHOLDER = "__FERRULE_ENV__"

_IDENT = re.compile(r"^[A-Z][A-Za-z0-9]*$")
_ENV = re.compile(r"^[A-Z][A-Z0-9_]*$")


@dataclass(frozen=True, kw_only=True)
class CSharpOptions:
    """Settings that vary the C# runtime between generated SDKs."""

    namespace: str
    """Root namespace and project directory (``targets.csharp.package_name``: ``Imagekit``)."""
    client_name: str
    """Prefix of the client and exception classes (``ImageKit``: ``ImageKitClient``,
    ``ImageKitException``)."""
    env_prefix: str
    """``<PREFIX>_CUSTOM_HEADERS`` (``IMAGE_KIT``)."""
    auth_headers: str = ""
    """C# statements of ``AddDefaultHeaders`` that add the credentials; empty without auth."""


def _check(options: CSharpOptions) -> None:
    for kind, value in (("namespace", options.namespace), ("client_name", options.client_name)):
        if not _IDENT.match(value):
            msg = f"{kind} must be a PascalCase C# identifier, got {value!r}"
            raise RenderError(msg)
    if not _ENV.match(options.env_prefix):
        msg = f"env_prefix must be SCREAMING_SNAKE_CASE, got {options.env_prefix!r}"
        raise RenderError(msg)
    if options.auth_headers and not options.auth_headers.endswith("\n"):
        msg = "auth_headers must be empty or end with a newline"
        raise RenderError(msg)


def _flags(options: CSharpOptions) -> dict[str, bool]:
    # csharpier sorts the usings after `System*` alphabetically, so the SDK's own namespace
    # comes before or after Moq's.
    return {"ns_before_moq": options.namespace.lower() < "moq"}


def render_csharp_files(options: CSharpOptions) -> dict[str, str]:
    """Render the C# runtime (pure): ``{path from the SDK root: exact text}``."""
    _check(options)
    files = render_tree(
        template_root("csharp"),
        comment="//",
        replacements={
            NS_PLACEHOLDER: options.namespace,
            CLIENT_PLACEHOLDER: options.client_name,
            ENV_PLACEHOLDER: options.env_prefix,
        },
        flags=_flags(options),
        slots={"auth_headers": options.auth_headers},
    )
    return {
        rel: text.replace("\n", "\r\n") if rel.endswith(".cs") else text
        for rel, text in files.items()
    }
