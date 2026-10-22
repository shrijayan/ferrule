# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Command-line entry point for ``ferrule``."""

from __future__ import annotations

import argparse
import sys
from typing import TYPE_CHECKING

from ferrule import __version__

if TYPE_CHECKING:
    from collections.abc import Sequence

SUBCOMMANDS: dict[str, str] = {
    "generate": "Generate an SDK from an OpenAPI spec.",
    "adopt": "Adopt an existing Stainless-generated SDK.",
    "rescue": "Rescue custom code from a generated SDK.",
    "verify": "Verify an SDK against its oracle.",
    "merge": "Merge regenerated code with custom changes.",
}

EXIT_NOT_IMPLEMENTED = 2


def build_parser() -> argparse.ArgumentParser:
    """Build the top-level argument parser."""
    parser = argparse.ArgumentParser(prog="ferrule", description="Ferrule SDK tooling.")
    parser.add_argument("--version", action="version", version=f"ferrule {__version__}")
    sub = parser.add_subparsers(dest="command", required=True, metavar="COMMAND")
    for name, help_text in SUBCOMMANDS.items():
        sub.add_parser(name, help=help_text, description=help_text)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    """Run the CLI and return a process exit code."""
    args = build_parser().parse_args(argv)
    command: str = args.command
    sys.stderr.write(f"ferrule {command}: FER0001 not implemented in this version\n")
    return EXIT_NOT_IMPLEMENTED
