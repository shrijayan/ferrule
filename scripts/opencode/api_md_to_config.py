# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Draft a stainless.yml for the opencode SDK from its ``api.md`` resource tables.

An adopter writes the ``resources`` tree from what the SDK already exposes. ``api.md`` lists
every method as ``| `name(args)` | `VERB /path` | ...`` under a ``## `client.a.b` `` heading,
which is all the resource tree needs. Client settings are fixed below, read off
``src/opencode/_client.py`` (basic auth, ``OPENCODE_`` environment variables).

``build`` is pure and deterministic; only ``main`` touches files.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path
from typing import cast

HEADING_RE = re.compile(r"^## `client\.([a-z0-9_.]+)`$")
ROW_RE = re.compile(r"^\| `([a-z0-9_]+)\(.*?\)` \| `([A-Z]+) (\S+)` \|")
PARAM_RE = re.compile(r":([A-Za-z0-9_]+)")

HEADER = """\
# stainless.yml for shrijayan/opencode-python-sdk v0.2.3, written as an adopter would.
# The resource tree is drafted from the SDK's api.md by scripts/opencode/api_md_to_config.py;
# client settings follow src/opencode/_client.py. The spec comes from
# scripts/opencode/contract_to_openapi.py (ADR 0013).
organization:
  name: opencode
  docs: https://opencode.ai/docs
  contact: https://github.com/shrijayan/opencode-python-sdk/issues

security:
  - basicAuth: []

security_schemes:
  basicAuth:
    type: http
    scheme: basic

client_settings:
  default_client_name: Opencode
  default_env_prefix: OPENCODE
  opts:
    username:
      type: string
      nullable: true
      default: opencode
      auth:
        security_scheme: basicAuth
        role: username
    password:
      type: string
      nullable: true
      read_env: OPENCODE_PASSWORD
      auth:
        security_scheme: basicAuth
        role: password

environments:
  production: http://127.0.0.1:4096

targets:
  python:
    package_name: opencode
    project_name: opencode-python-sdk
    publish:
      pypi: false

readme:
  example_requests:
    default:
      type: request
      endpoint: get /api/session
      params: {}
    headline:
      type: request
      endpoint: get /api/session
      params: {}

"""


class ApiMdError(ValueError):
    """Raised when ``api.md`` has a row this script cannot place."""


type Tree = dict[str, object]


def endpoint(verb: str, path: str) -> str:
    """Turn ``GET /api/a/:id`` into the stainless.yml endpoint ``get /api/a/{id}``."""
    path = PARAM_RE.sub(r"{\1}", path)
    if path.endswith("/*"):
        path = path[:-1] + "{path}"
    return f"{verb.lower()} {path}"


def parse(text: str) -> Tree:
    """Return the nested resource tree ``{name: {"methods": {...}, "subresources": {...}}}``."""
    tree: Tree = {}
    node: Tree | None = None
    for line in text.splitlines():
        heading = HEADING_RE.match(line)
        if heading:
            node = {"subresources": tree}
            for part in heading.group(1).split("."):
                children = cast("Tree", node.setdefault("subresources", {}))
                node = cast("Tree", children.setdefault(part, {}))
            continue
        row = ROW_RE.match(line)
        if row is None:
            continue
        if node is None:
            msg = f"method row before any resource heading: {line}"
            raise ApiMdError(msg)
        methods = cast("dict[str, str]", node.setdefault("methods", {}))
        name, verb, path = row.groups()
        if name in methods:
            msg = f"duplicate method {name}: {line}"
            raise ApiMdError(msg)
        methods[name] = endpoint(verb, path)
    return tree


def _emit(tree: Tree, indent: int, out: list[str]) -> None:
    pad = "  " * indent
    for name, node in tree.items():
        out.append(f"{pad}{name}:")
        body = cast("Tree", node)
        methods = cast("dict[str, str]", body.get("methods", {}))
        if methods:
            out.append(f"{pad}  methods:")
            out.extend(f"{pad}    {m}: {e}" for m, e in methods.items())
        sub = cast("Tree", body.get("subresources", {}))
        if sub:
            out.append(f"{pad}  subresources:")
            _emit(sub, indent + 2, out)


def build(api_md: str) -> str:
    """Return the full stainless.yml text for ``api_md``."""
    out = ["resources:"]
    _emit(parse(api_md), 1, out)
    return HEADER + "\n".join(out) + "\n"


def main(argv: list[str] | None = None) -> int:
    """Read ``api.md`` and write ``stainless.yml``."""
    parser = argparse.ArgumentParser(description="Draft stainless.yml from api.md.")
    parser.add_argument("api_md", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args(argv)
    source = cast("Path", args.api_md)
    cast("Path", args.output).write_text(build(source.read_text(encoding="utf-8")), "utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
