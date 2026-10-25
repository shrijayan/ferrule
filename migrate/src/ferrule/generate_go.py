# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule generate --target go``: spec + stainless.yml -> a Go SDK tree (Stainless Go v1).

Pipeline: ``ferrule-gen convert`` -> ``ferrule-gen generate --target go`` (the Go plugin's files
plus ``.ferrule/go-runtime.json``, the runtime's options) -> the Go runtime, rendered by
``ferrule_runtime.render_go_files`` from those options -> ``gofmt -l`` (the plugin and the
runtime are gofmt-clean, so any file it lists is an error).

As in :mod:`ferrule.generate`, planning is pure and :func:`generate_go` is the only function that
touches the file system or runs processes; it takes the runner and the renderer as parameters.
"""

from __future__ import annotations

import importlib
import json
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
    from collections.abc import Mapping

GO_RUNTIME_OPTIONS = ".ferrule/go-runtime.json"
"""Where ``ferrule-gen generate --target go`` writes the runtime options, relative to ``--out``."""


class GoRenderer(Protocol):
    """``ferrule_runtime.render_go_files`` over the options' JSON object."""

    def __call__(self, options: Mapping[str, object], /) -> Mapping[str, str]:
        """Render the runtime: ``{path from the module root: text}``."""
        ...


@dataclass(frozen=True)
class GoRequest:
    """Inputs of one ``ferrule generate --target go`` run."""

    spec: Path
    config: Path
    out: Path
    gen: Path
    gofmt: str = "gofmt"


def plan_go_steps(request: GoRequest, work: Path) -> tuple[Step, Step]:
    """Plan ``ferrule-gen convert`` and ``generate --target go``, the annotated spec in ``work``."""
    annotated = work / "annotated.json"
    gen = str(request.gen)
    convert = Step(
        "convert", (gen, "convert", str(request.spec), str(request.config), "-o", str(annotated))
    )
    generate = Step(
        "generate", (gen, "generate", str(annotated), "--target", "go", "--out", str(request.out))
    )
    return convert, generate


def gofmt_step(request: GoRequest) -> Step:
    """``gofmt -l`` over the output: lists the files gofmt would change."""
    return Step("gofmt", (request.gofmt, "-l", str(request.out)))


def load_go_renderer() -> GoRenderer | None:
    """``render_go_files(GoOptions(**options))`` when the runtime package provides it."""
    try:
        module = importlib.import_module("ferrule_runtime")
    except ImportError:
        return None
    options_type: object = getattr(module, "GoOptions", None)
    render: object = getattr(module, "render_go_files", None)
    if not callable(options_type) or not callable(render):
        return None

    def go_renderer(options: Mapping[str, object], /) -> Mapping[str, str]:
        return cast("Mapping[str, str]", render(options_type(**options)))

    return go_renderer


def _check(request: GoRequest) -> None:
    for label, path in (("--spec", request.spec), ("--config", request.config)):
        if not path.is_file():
            raise fail(MISSING_INPUT, f"{path} does not exist", label, "pass an existing file")
    if not request.gen.is_file():
        hint = "run `cargo build --release -p ferrule-gen`, or set FERRULE_GEN"
        raise fail(TOOL_NOT_FOUND, f"{request.gen} does not exist", "FERRULE_GEN", hint)
    if shutil.which(request.gofmt) is None:
        hint = "install Go (`mise install`) or set FERRULE_GOFMT"
        raise fail(TOOL_NOT_FOUND, f"{request.gofmt} is not on PATH", "FERRULE_GOFMT", hint)
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


def _render_runtime(out: Path, renderer: GoRenderer) -> int:
    """Render the runtime from the options ``generate`` wrote, then remove them."""
    options_path = out / GO_RUNTIME_OPTIONS
    if not options_path.is_file():
        raise fail(
            NO_RUNTIME,
            f"{options_path} is missing: ferrule-gen did not write the Go runtime options",
            "generate",
            "use a ferrule-gen built with the Go plugin",
        )
    options = json.loads(options_path.read_text(encoding="utf-8"))
    options_path.unlink()
    options_path.parent.rmdir()
    files = renderer(options)
    for rel, text in files.items():
        path = ensure_inside(out, PurePosixPath(rel))
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(text.encode("utf-8"))
    return len(files)


def generate_go(
    request: GoRequest,
    runner: Runner = run_subprocess,
    renderer: GoRenderer | None = None,
) -> GenerateResult:
    """Run the Go pipeline into ``request.out``."""
    _check(request)
    if renderer is None:
        raise fail(
            NO_RUNTIME,
            "no Go runtime: ferrule_runtime.render_go_files is unavailable",
            "ferrule_runtime",
            "install the ferrule-runtime package",
        )
    request.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ferrule-") as tmp:
        for step in plan_go_steps(request, Path(tmp)):
            _run(step, runner)
    runtime = _render_runtime(request.out, renderer)
    unformatted = _run(gofmt_step(request), runner).split()
    if unformatted:
        raise fail(
            STEP_FAILED,
            f"gofmt would change {', '.join(unformatted)}",
            "gofmt",
            "report it: the Go plugin's printer must write gofmt's layout",
        )
    return GenerateResult(
        package=_go_package(request.out),
        files=len(relative_files(request.out)),
        runtime_files=runtime,
        digest=tree_digest(request.out),
    )


def _go_package(out: Path) -> str:
    """Read the module path from ``go.mod``."""
    go_mod = out / "go.mod"
    if go_mod.is_file():
        for line in go_mod.read_text(encoding="utf-8").splitlines():
            if line.startswith("module "):
                return line.removeprefix("module ").strip()
    return ""
