# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Command-line entry point for ``ferrule``: argument parsing and I/O only."""

from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING

from ferrule import __version__
from ferrule.errors import FerruleError
from ferrule.generate import GenerateRequest, generate, load_renderer, resolve_tools
from ferrule.verify import export_ref, verify

if TYPE_CHECKING:
    from collections.abc import Sequence

SUBCOMMANDS: dict[str, str] = {
    "generate": "Generate an SDK from an OpenAPI spec.",
    "adopt": "Adopt an existing Stainless-generated SDK.",
    "rescue": "Rescue custom code from a generated SDK.",
    "verify": "Verify an SDK against its oracle.",
    "merge": "Merge regenerated code with custom changes.",
}
UNIMPLEMENTED = ("adopt", "rescue", "merge")

EXIT_OK = 0
EXIT_FAILED = 1
EXIT_NOT_IMPLEMENTED = 2


def build_parser() -> argparse.ArgumentParser:
    """Build the top-level argument parser."""
    parser = argparse.ArgumentParser(prog="ferrule", description="Ferrule SDK tooling.")
    parser.add_argument("--version", action="version", version=f"ferrule {__version__}")
    sub = parser.add_subparsers(dest="command", required=True, metavar="COMMAND")
    subs = {
        name: sub.add_parser(name, help=text, description=text)
        for name, text in SUBCOMMANDS.items()
    }

    gen = subs["generate"]
    gen.add_argument("--spec", type=Path, required=True, help="OpenAPI spec (.yml or .json)")
    gen.add_argument("--config", type=Path, required=True, help="stainless.yml")
    gen.add_argument("--out", type=Path, required=True, help="new, empty output directory")
    gen.add_argument("--runtime-from", type=Path, help="copy the runtime from this SDK checkout")
    gen.add_argument("--forge", type=Path, help="Forge CLI (default: $FERRULE_FORGE)")
    gen.add_argument("--plugin", type=Path, help="plugin.wasm (default: $FERRULE_PLUGIN)")

    ver = subs["verify"]
    ver.add_argument("--against", required=True, help="old SDK: a directory or a git ref")
    ver.add_argument("--new", type=Path, required=True, help="new SDK directory")
    ver.add_argument("--repo", type=Path, default=Path(), help="git repo for --against REF")
    ver.add_argument("--package", help="import name (default: detected)")
    ver.add_argument("--json", type=Path, help="also write the full report here")
    return parser


def _generate(args: argparse.Namespace) -> int:
    request = GenerateRequest(
        spec=args.spec,
        config=args.config,
        out=args.out,
        tools=resolve_tools(os.environ, forge=args.forge, plugin=args.plugin),
        runtime_from=args.runtime_from,
    )
    result = generate(request, renderer=load_renderer())
    sys.stdout.write(
        f"generated {result.package}: {result.files} files "
        f"({result.runtime_files} runtime) sha256 {result.digest}\n",
    )
    return EXIT_OK


def _verify(args: argparse.Namespace) -> int:
    against = Path(args.against)
    with tempfile.TemporaryDirectory(prefix="ferrule-verify-") as tmp:
        old = against if against.is_dir() else export_ref(args.repo, args.against, Path(tmp))
        report = verify(old, args.new, args.package)
    sys.stdout.write(report.summary() + "\n")
    for change in report.breaking:
        sys.stdout.write(f"  breaking {change.detail}\n")
    if args.json is not None:
        args.json.write_text(json.dumps(report.to_json(), indent=2, sort_keys=True) + "\n")
    return EXIT_FAILED if report.breaking else EXIT_OK


def main(argv: Sequence[str] | None = None) -> int:
    """Run the CLI and return a process exit code."""
    args = build_parser().parse_args(argv)
    command: str = args.command
    if command in UNIMPLEMENTED:
        sys.stderr.write(f"ferrule {command}: FER0001 not implemented in this version\n")
        return EXIT_NOT_IMPLEMENTED
    try:
        return _generate(args) if command == "generate" else _verify(args)
    except FerruleError as exc:
        sys.stderr.write(f"ferrule {command}: {exc.diagnostic.render()}\n")
        return EXIT_FAILED
