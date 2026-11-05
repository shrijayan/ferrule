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
from ferrule.adopt import AdoptRequest, adopt
from ferrule.errors import MISSING_INPUT, FerruleError, fail
from ferrule.generate import GenerateRequest, generate, load_renderer, resolve_tools
from ferrule.generate_csharp import FORMATTER_BIN as CSHARP_FORMATTER_BIN
from ferrule.generate_csharp import CSharpRequest, generate_csharp, load_csharp_renderer
from ferrule.generate_go import GoRequest, generate_go, load_go_renderer
from ferrule.generate_jvm import JvmRequest, generate_jvm, load_java_renderer, resolve_ktfmt
from ferrule.generate_php import FORMATTER_BIN as PHP_FORMATTER_BIN
from ferrule.generate_php import PhpRequest, generate_php, load_php_renderer
from ferrule.generate_ruby import (
    RubyRequest,
    generate_ruby,
    load_ruby_renderer,
    resolve_formatter,
)
from ferrule.regenerate import RegenerateRequest, regenerate
from ferrule.rescue import rescue, write_spec
from ferrule.verify import export_ref, verify

if TYPE_CHECKING:
    from collections.abc import Callable, Sequence

SUBCOMMANDS: dict[str, str] = {
    "generate": "Generate an SDK from an OpenAPI spec.",
    "adopt": "Adopt an existing Stainless-generated SDK.",
    "rescue": "Save the OpenAPI spec .stats.yml points to (md5-checked).",
    "verify": "Verify an SDK against its oracle.",
    "merge": "Merge regenerated code with custom changes.",
    "regenerate": "Regenerate an adopted SDK and merge it into the current branch.",
}
UNIMPLEMENTED: tuple[str, ...] = ()

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
    gen.add_argument("--gen", type=Path, help="ferrule-gen binary (default: $FERRULE_GEN)")
    gen.add_argument(
        "--target",
        choices=("python", "go", "java", "kotlin", "ruby", "php", "csharp"),
        default="python",
        help="SDK language (default: python)",
    )

    ver = subs["verify"]
    ver.add_argument("--against", required=True, help="old SDK: a directory or a git ref")
    ver.add_argument("--new", type=Path, required=True, help="new SDK directory")
    ver.add_argument("--repo", type=Path, default=Path(), help="git repo for --against REF")
    ver.add_argument("--package", help="import name (default: detected)")
    ver.add_argument("--json", type=Path, help="also write the full report here")

    res = subs["rescue"]
    res.add_argument("repo", nargs="?", type=Path, default=Path(), help="SDK checkout (default: .)")
    res.add_argument("--out", type=Path, help="where to write the spec (default: openapi.<ext>)")

    ado = subs["adopt"]
    ado.add_argument("repo", nargs="?", type=Path, default=Path(), help="SDK checkout (default: .)")
    ado.add_argument("--config", type=Path, help="stainless.yml (default: REPO/stainless.yml)")
    ado.add_argument("--spec", type=Path, help="OpenAPI spec (default: rescue it)")
    ado.add_argument("--dry-run", action="store_true", help="report the plan, change nothing")
    ado.add_argument("--gen", type=Path, help="ferrule-gen binary (default: $FERRULE_GEN)")

    mrg = subs["merge"]
    mrg.add_argument("--base", required=True, help="previous generation: a directory or ref")
    mrg.add_argument("--ours", required=True, help="custom branch: a directory or ref")
    mrg.add_argument("--theirs", required=True, help="new generation: a directory or ref")
    mrg.add_argument("--repo", type=Path, default=Path(), help="repository for REF arguments")
    mrg.add_argument("--out", type=Path, help="write the merged tree to this new directory")
    mrg.add_argument("--report", type=Path, help="write the merge report (JSON) here")
    mrg.add_argument("--keep", action="append", default=[], help="keep_files glob (repeatable)")
    reg = subs["regenerate"]
    reg.add_argument("--repo", type=Path, default=Path(), help="SDK checkout (default: .)")
    reg.add_argument("--spec", type=Path, required=True, help="OpenAPI spec (.yml or .json)")
    reg.add_argument("--config", type=Path, required=True, help="stainless.yml")
    reg.add_argument("--gen", type=Path, help="ferrule-gen binary (default: $FERRULE_GEN)")
    return parser


def _rescue(args: argparse.Namespace) -> int:
    rescued = rescue(args.repo)
    out: Path = args.out or args.repo / f"openapi{rescued.stats.suffix}"
    write_spec(rescued, out)
    sys.stdout.write(f"rescued spec to {out} (md5 {rescued.stats.spec_hash} ok)\n")
    return EXIT_OK


def _say(line: str) -> None:
    sys.stdout.write(line + "\n")
    sys.stdout.flush()


def _adopt(args: argparse.Namespace) -> int:
    request = AdoptRequest(
        repo=args.repo,
        tools=resolve_tools(os.environ, gen=args.gen),
        config=args.config,
        spec=args.spec,
        dry_run=args.dry_run,
    )
    result = adopt(request, renderer=load_renderer(), say=_say)
    if result.branch is None:
        return EXIT_OK
    sys.stdout.write(f"ferrule verify --against {result.against}\n")
    with tempfile.TemporaryDirectory(prefix="ferrule-verify-") as tmp:
        old = export_ref(result.repo, result.against, Path(tmp) / "old")
        new = export_ref(result.repo, result.branch, Path(tmp) / "new")
        report = verify(old, new)
    sys.stdout.write(report.summary() + "\n")
    for change in report.breaking:
        sys.stdout.write(f"  breaking {change.detail}\n")
    return EXIT_FAILED if report.breaking else EXIT_OK


def _generate_go(args: argparse.Namespace) -> int:
    if args.runtime_from is not None:
        raise fail(
            MISSING_INPUT,
            "--runtime-from is not supported with --target go",
            "--runtime-from",
            "drop it: the Go runtime is rendered from ferrule_runtime",
        )
    request = GoRequest(
        spec=args.spec,
        config=args.config,
        out=args.out,
        gen=resolve_tools(os.environ, gen=args.gen).gen,
        gofmt=os.environ.get("FERRULE_GOFMT", "gofmt"),
    )
    result = generate_go(request, renderer=load_go_renderer())
    sys.stdout.write(
        f"generated {result.package}: {result.files} files "
        f"({result.runtime_files} runtime) sha256 {result.digest}\n",
    )
    return EXIT_OK


def _generate_jvm(args: argparse.Namespace) -> int:
    if args.runtime_from is not None:
        raise fail(
            MISSING_INPUT,
            f"--runtime-from is not supported with --target {args.target}",
            "--runtime-from",
            "drop it: the JVM runtime is rendered from ferrule_runtime",
        )
    request = JvmRequest(
        spec=args.spec,
        config=args.config,
        out=args.out,
        gen=resolve_tools(os.environ, gen=args.gen).gen,
        target=args.target,
        ktfmt=resolve_ktfmt(os.environ),
    )
    result = generate_jvm(request, renderer=load_java_renderer())
    sys.stdout.write(
        f"generated {result.package}: {result.files} files "
        f"({result.runtime_files} runtime) sha256 {result.digest}\n",
    )
    return EXIT_OK


def _generate_ruby(args: argparse.Namespace) -> int:
    if args.runtime_from is not None:
        raise fail(
            MISSING_INPUT,
            "--runtime-from is not supported with --target ruby",
            "--runtime-from",
            "drop it: the Ruby runtime is rendered from ferrule_runtime",
        )
    request = RubyRequest(
        spec=args.spec,
        config=args.config,
        out=args.out,
        gen=resolve_tools(os.environ, gen=args.gen).gen,
        stree=resolve_formatter(os.environ, "FERRULE_STREE", "stree"),
        rubocop=resolve_formatter(os.environ, "FERRULE_RUBOCOP", "rubocop"),
    )
    result = generate_ruby(request, renderer=load_ruby_renderer())
    sys.stdout.write(
        f"generated {result.package}: {result.files} files "
        f"({result.runtime_files} runtime) sha256 {result.digest}\n",
    )
    return EXIT_OK


def _generate_php(args: argparse.Namespace) -> int:
    if args.runtime_from is not None:
        raise fail(
            MISSING_INPUT,
            "--runtime-from is not supported with --target php",
            "--runtime-from",
            "drop it: the PHP runtime is rendered from ferrule_runtime",
        )
    request = PhpRequest(
        spec=args.spec,
        config=args.config,
        out=args.out,
        gen=resolve_tools(os.environ, gen=args.gen).gen,
        fixer=resolve_formatter(
            os.environ, "FERRULE_PHP_CS_FIXER", "php-cs-fixer", bin_dir=PHP_FORMATTER_BIN
        ),
    )
    result = generate_php(request, renderer=load_php_renderer())
    sys.stdout.write(
        f"generated {result.package}: {result.files} files "
        f"({result.runtime_files} runtime) sha256 {result.digest}\n",
    )
    return EXIT_OK


def _generate_csharp(args: argparse.Namespace) -> int:
    if args.runtime_from is not None:
        raise fail(
            MISSING_INPUT,
            "--runtime-from is not supported with --target csharp",
            "--runtime-from",
            "drop it: the C# runtime is rendered from ferrule_runtime",
        )
    request = CSharpRequest(
        spec=args.spec,
        config=args.config,
        out=args.out,
        gen=resolve_tools(os.environ, gen=args.gen).gen,
        csharpier=resolve_formatter(
            os.environ, "FERRULE_CSHARPIER", "csharpier", CSHARP_FORMATTER_BIN
        ),
    )
    result = generate_csharp(request, renderer=load_csharp_renderer())
    sys.stdout.write(
        f"generated {result.package}: {result.files} files "
        f"({result.runtime_files} runtime) sha256 {result.digest}\n",
    )
    return EXIT_OK


def _generate(args: argparse.Namespace) -> int:
    if args.target == "go":
        return _generate_go(args)
    if args.target in ("java", "kotlin"):
        return _generate_jvm(args)
    if args.target == "ruby":
        return _generate_ruby(args)
    if args.target == "php":
        return _generate_php(args)
    if args.target == "csharp":
        return _generate_csharp(args)
    request = GenerateRequest(
        spec=args.spec,
        config=args.config,
        out=args.out,
        tools=resolve_tools(os.environ, gen=args.gen),
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


def _merge(args: argparse.Namespace) -> int:
    from ferrule.merge import KeepRules, merge_trees  # noqa: PLC0415 - keeps startup light
    from ferrule.merge.gitio import read_tree, write_tree  # noqa: PLC0415

    trees = [read_tree(src, args.repo) for src in (args.base, args.ours, args.theirs)]
    result = merge_trees(*trees, keep=KeepRules(tuple(args.keep)))
    if args.out is not None:
        write_tree(args.out, result.files)
    for conflict in result.conflicts:
        sys.stdout.write(conflict.render() + "\n")
    sys.stdout.write(f"merged {len(result.files)} files, {len(result.blocking)} conflict(s)\n")
    if args.report is not None:
        args.report.write_text(json.dumps(result.to_json(), indent=2, sort_keys=True) + "\n")
    return EXIT_FAILED if result.blocking else EXIT_OK


def _regenerate(args: argparse.Namespace) -> int:
    request = RegenerateRequest(
        repo=args.repo,
        spec=args.spec,
        config=args.config,
        tools=resolve_tools(os.environ, gen=args.gen),
    )
    result = regenerate(request, renderer=load_renderer())
    if result.conflict_branch is not None:
        for conflict in result.result.conflicts:
            sys.stdout.write(conflict.render() + "\n")
        sys.stdout.write(f"conflict: the merge is on {result.conflict_branch}\n")
        return EXIT_FAILED
    done = "merged" if result.commit else "nothing to merge"
    sys.stdout.write(f"{done} into {result.branch} (generation {result.generated[:12]})\n")
    return EXIT_OK


COMMANDS: dict[str, Callable[[argparse.Namespace], int]] = {
    "adopt": _adopt,
    "merge": _merge,
    "regenerate": _regenerate,
    "rescue": _rescue,
}


def main(argv: Sequence[str] | None = None) -> int:
    """Run the CLI and return a process exit code."""
    args = build_parser().parse_args(argv)
    command: str = args.command
    if command in UNIMPLEMENTED:
        sys.stderr.write(f"ferrule {command}: FER0001 not implemented in this version\n")
        return EXIT_NOT_IMPLEMENTED
    try:
        if command in COMMANDS:
            return COMMANDS[command](args)
        return _generate(args) if command == "generate" else _verify(args)
    except FerruleError as exc:
        sys.stderr.write(f"ferrule {command}: {exc.diagnostic.render()}\n")
        return EXIT_FAILED
