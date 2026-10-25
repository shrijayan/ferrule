# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule generate --target ruby``: spec + stainless.yml -> a Ruby gem tree (Stainless layout).

Pipeline: ``ferrule-gen convert`` -> ``ferrule-gen generate --target ruby`` (the Ruby plugin's
files plus ``.ferrule/ruby-runtime.json``, the runtime's options) -> the gem's ``rake format`` on
the generated files: ``rubocop -a`` on ``.rb`` (with the generated ``.rubocop.yml``),
``stree write`` on ``.rbi`` and, with the RBS plugin, on ``.rbs`` (the Rakefile's class-alias
workaround around it) -> the Ruby runtime, rendered by ``ferrule_runtime.render_ruby_files`` from
those options (already in the formatted layout, so it is not formatted again).

The formatters are the versions ``crates/ruby-plugin/tools/Gemfile`` pins; ``FERRULE_STREE`` and
``FERRULE_RUBOCOP`` name them (such as ``bundle exec stree``). As in :mod:`ferrule.generate`,
planning is pure and :func:`generate_ruby` is the only function that touches the file system or
runs processes; it takes the runner and the renderer as parameters.
"""

from __future__ import annotations

import importlib
import json
import re
import shutil
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING, Protocol, cast

from ferrule.errors import (
    MISSING_INPUT,
    NO_RUNTIME,
    OUT_NOT_EMPTY,
    STEP_FAILED,
    TOOL_NOT_FOUND,
    fail,
)
from ferrule.generate import (
    GenerateResult,
    Runner,
    Step,
    ensure_inside,
    relative_files,
    run_subprocess,
    tree_digest,
)

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence

RUBY_RUNTIME_OPTIONS = ".ferrule/ruby-runtime.json"
"""Where ``ferrule-gen generate --target ruby`` writes the runtime options (from ``--out``)."""

ALIAS_MARK = "ferrule-rbs-alias"
"""The marker comment that hides ``class X = Y`` from syntax_tree while it formats ``.rbs``."""

_ALIAS = re.compile(r"^(?P<ws>\s*)(?P<kind>class|module) (?P<name>\S+) = (?P<value>.+)$")


class RubyRenderer(Protocol):
    """``ferrule_runtime.render_ruby_files`` over the options' JSON object."""

    def __call__(self, options: Mapping[str, object], /) -> Mapping[str, str]:
        """Render the runtime: ``{path from the gem root: text}``."""
        ...


@dataclass(frozen=True)
class RubyRequest:
    """Inputs of one ``ferrule generate --target ruby`` run."""

    spec: Path
    config: Path
    out: Path
    gen: Path
    stree: tuple[str, ...] = ("stree",)
    rubocop: tuple[str, ...] = ("rubocop",)


def plan_ruby_steps(request: RubyRequest, work: Path) -> tuple[Step, Step]:
    """Plan ``ferrule-gen convert`` and ``generate --target ruby`` (annotated spec in ``work``)."""
    annotated = work / "annotated.json"
    gen = str(request.gen)
    convert = Step(
        "convert", (gen, "convert", str(request.spec), str(request.config), "-o", str(annotated))
    )
    generate = Step(
        "generate",
        (gen, "generate", str(annotated), "--target", "ruby", "--out", str(request.out)),
    )
    return convert, generate


def format_steps(request: RubyRequest, files: Sequence[PurePosixPath]) -> list[Step]:
    """Plan the gem's ``rake format`` on ``files`` (from ``--out``): rubocop, then syntax_tree."""
    out = request.out
    by_ext: dict[str, list[str]] = {".rb": [], ".rbi": [], ".rbs": []}
    for rel in files:
        if rel.suffix in by_ext:
            by_ext[rel.suffix].append(str(out / rel))
    # As the Rakefile's tasks run them (`RUBYOPT=--encoding=UTF-8`).
    env = ("env", "RUBYOPT=--encoding=UTF-8")
    rubocop = (
        *env,
        *request.rubocop,
        "--config",
        str(out / ".rubocop.yml"),
        "--fail-level",
        "F",
        "--autocorrect",
        "--format",
        "quiet",
        "--",
    )
    return [
        Step(name, (*argv, *paths))
        for name, argv, paths in (
            ("rubocop", rubocop, by_ext[".rb"]),
            ("stree", (*env, *request.stree, "write", "--"), by_ext[".rbi"]),
            ("stree", (*env, *request.stree, "write", "--plugin=rbs", "--"), by_ext[".rbs"]),
        )
        if paths
    ]


def hide_aliases(text: str) -> str:
    """``class X = Y`` lines as a marker comment and the constant ``X: Y``, which stree parses."""
    out: list[str] = []
    for line in text.split("\n"):
        m = _ALIAS.match(line)
        if m:
            ws = m["ws"]
            out.extend((f"{ws}# {m['kind']} {ALIAS_MARK}", f"{ws}{m['name']}: {m['value']}"))
        else:
            out.append(line)
    return "\n".join(out)


def restore_aliases(text: str) -> str:
    """Undo :func:`hide_aliases` on stree's output."""
    lines = text.split("\n")
    out: list[str] = []
    i = 0
    while i < len(lines):
        stripped = lines[i].lstrip()
        ws = lines[i][: len(lines[i]) - len(stripped)]
        kind = stripped.removeprefix("# ").removesuffix(f" {ALIAS_MARK}")
        if (
            kind in {"class", "module"}
            and stripped == f"# {kind} {ALIAS_MARK}"
            and i + 1 < len(lines)
        ):
            name, sep, value = lines[i + 1].lstrip().partition(": ")
            if sep:
                out.append(f"{ws}{kind} {name} = {value}")
                i += 2
                continue
        out.append(lines[i])
        i += 1
    return "\n".join(out)


def load_ruby_renderer() -> RubyRenderer | None:
    """``render_ruby_files(RubyOptions(**options))`` when the runtime package provides it."""
    try:
        module = importlib.import_module("ferrule_runtime")
    except ImportError:
        return None
    options_type: object = getattr(module, "RubyOptions", None)
    render: object = getattr(module, "render_ruby_files", None)
    if not callable(options_type) or not callable(render):
        return None

    def ruby_renderer(options: Mapping[str, object], /) -> Mapping[str, str]:
        return cast("Mapping[str, str]", render(options_type(**options)))

    return ruby_renderer


def _check(request: RubyRequest) -> None:
    for label, path in (("--spec", request.spec), ("--config", request.config)):
        if not path.is_file():
            raise fail(MISSING_INPUT, f"{path} does not exist", label, "pass an existing file")
    if not request.gen.is_file():
        hint = "run `cargo build --release -p ferrule-gen`, or set FERRULE_GEN"
        raise fail(TOOL_NOT_FOUND, f"{request.gen} does not exist", "FERRULE_GEN", hint)
    for name, cmd in (("FERRULE_STREE", request.stree), ("FERRULE_RUBOCOP", request.rubocop)):
        if not cmd or shutil.which(cmd[0]) is None:
            hint = (
                "install the gem's formatters (BUNDLE_GEMFILE=crates/ruby-plugin/tools/Gemfile "
                f"bundle install) and set {name}, such as `bundle exec stree`"
            )
            raise fail(TOOL_NOT_FOUND, f"{' '.join(cmd)} is not on PATH", name, hint)
    if request.out.exists() and any(request.out.iterdir()):
        raise fail(OUT_NOT_EMPTY, f"{request.out} is not empty", "--out", "pass a new directory")


def _run(step: Step, runner: Runner) -> str:
    proc = runner(step.argv)
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout).strip().splitlines()
        last = detail[-1] if detail else f"exit code {proc.returncode}"
        raise fail(
            STEP_FAILED, f"{step.name} failed: {last}", step.name, "fix the error it reported"
        )
    return proc.stdout


def _read_options(out: Path) -> dict[str, object]:
    """Read the runtime options ``generate`` wrote, and remove them from the tree."""
    options_path = out / RUBY_RUNTIME_OPTIONS
    if not options_path.is_file():
        raise fail(
            NO_RUNTIME,
            f"{options_path} is missing: ferrule-gen did not write the Ruby runtime options",
            "generate",
            "use a ferrule-gen built with the Ruby plugin",
        )
    options = cast("dict[str, object]", json.loads(options_path.read_text(encoding="utf-8")))
    options_path.unlink()
    options_path.parent.rmdir()
    return options


def _format(request: RubyRequest, generated: Sequence[PurePosixPath], runner: Runner) -> None:
    rbs = [request.out / rel for rel in generated if rel.suffix == ".rbs"]
    for path in rbs:
        path.write_text(hide_aliases(path.read_text(encoding="utf-8")), encoding="utf-8")
    try:
        for step in format_steps(request, generated):
            _run(step, runner)
    finally:
        for path in rbs:
            path.write_text(restore_aliases(path.read_text(encoding="utf-8")), encoding="utf-8")


def generate_ruby(
    request: RubyRequest,
    runner: Runner = run_subprocess,
    renderer: RubyRenderer | None = None,
) -> GenerateResult:
    """Run the Ruby pipeline into ``request.out``."""
    _check(request)
    if renderer is None:
        raise fail(
            NO_RUNTIME,
            "no Ruby runtime: ferrule_runtime.render_ruby_files is unavailable",
            "ferrule_runtime",
            "install the ferrule-runtime package",
        )
    request.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ferrule-") as tmp:
        for step in plan_ruby_steps(request, Path(tmp)):
            _run(step, runner)
    options = _read_options(request.out)
    _format(request, relative_files(request.out), runner)
    files = renderer(options)
    for rel, text in files.items():
        path = ensure_inside(request.out, PurePosixPath(rel))
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(text.encode("utf-8"))
    return GenerateResult(
        package=str(options.get("gem_name", "")),
        files=len(relative_files(request.out)),
        runtime_files=len(files),
        digest=tree_digest(request.out),
    )
