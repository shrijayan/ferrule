# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule generate --target csharp``: spec + stainless.yml -> a .NET SDK tree (Stainless layout).

Pipeline: ``ferrule-gen convert`` -> ``ferrule-gen generate --target csharp`` (the C# plugin's
files plus ``.ferrule/csharp-runtime.json``, the runtime's options) -> ``csharpier format`` on the
generated tree, as the SDK's ``scripts/format`` runs it (the generated ``.editorconfig`` gives it
CRLF line endings) -> the C# runtime, rendered by ``ferrule_runtime.render_csharp_files`` from
those options (already in csharpier's layout, so it is not formatted again).

csharpier is the version ``crates/csharp-plugin/tools/dotnet-tools.json`` pins, which
``mise run csharp:tools`` installs into ``.cache/csharp-tools`` and
``crates/csharp-plugin/tools/bin/csharpier`` runs; ``FERRULE_CSHARPIER`` overrides it. As in
:mod:`ferrule.generate`, planning is pure and :func:`generate_csharp` is the only function that
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
    REPO_ROOT,
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

CSHARP_RUNTIME_OPTIONS = ".ferrule/csharp-runtime.json"
"""Where ``ferrule-gen generate --target csharp`` writes the runtime options (from ``--out``)."""

FORMATTER_BIN = REPO_ROOT / "crates/csharp-plugin/tools/bin"
"""The wrapper that runs the csharpier ``mise run csharp:tools`` installs."""


class CSharpRenderer(Protocol):
    """``ferrule_runtime.render_csharp_files`` over the options' JSON object."""

    def __call__(self, options: Mapping[str, object], /) -> Mapping[str, str]:
        """Render the runtime: ``{path from the SDK root: text}``."""
        ...


@dataclass(frozen=True)
class CSharpRequest:
    """Inputs of one ``ferrule generate --target csharp`` run."""

    spec: Path
    config: Path
    out: Path
    gen: Path
    csharpier: tuple[str, ...] = ("csharpier",)


def plan_csharp_steps(request: CSharpRequest, work: Path) -> tuple[Step, Step, Step]:
    """Plan ``ferrule-gen convert``, ``generate --target csharp`` and ``csharpier format``."""
    annotated = work / "annotated.json"
    gen = str(request.gen)
    out = str(request.out)
    return (
        Step(
            "convert",
            (gen, "convert", str(request.spec), str(request.config), "-o", str(annotated)),
        ),
        Step("generate", (gen, "generate", str(annotated), "--target", "csharp", "--out", out)),
        Step("csharpier", (*request.csharpier, "format", out)),
    )


def load_csharp_renderer() -> CSharpRenderer | None:
    """``render_csharp_files(CSharpOptions(**options))`` when the runtime package provides it."""
    try:
        module = importlib.import_module("ferrule_runtime")
    except ImportError:
        return None
    options_type: object = getattr(module, "CSharpOptions", None)
    render: object = getattr(module, "render_csharp_files", None)
    if not callable(options_type) or not callable(render):
        return None

    def csharp_renderer(options: Mapping[str, object], /) -> Mapping[str, str]:
        return cast("Mapping[str, str]", render(options_type(**options)))

    return csharp_renderer


def _check(request: CSharpRequest) -> None:
    for label, path in (("--spec", request.spec), ("--config", request.config)):
        if not path.is_file():
            raise fail(MISSING_INPUT, f"{path} does not exist", label, "pass an existing file")
    if not request.gen.is_file():
        hint = "run `cargo build --release -p ferrule-gen`, or set FERRULE_GEN"
        raise fail(TOOL_NOT_FOUND, f"{request.gen} does not exist", "FERRULE_GEN", hint)
    if not request.csharpier or shutil.which(request.csharpier[0]) is None:
        hint = (
            "install csharpier (`mise run csharp:tools` in the Ferrule repository) "
            "or set FERRULE_CSHARPIER, such as `dotnet csharpier`"
        )
        raise fail(
            TOOL_NOT_FOUND,
            f"{' '.join(request.csharpier)} is not on PATH",
            "FERRULE_CSHARPIER",
            hint,
        )
    if request.out.exists() and any(request.out.iterdir()):
        raise fail(OUT_NOT_EMPTY, f"{request.out} is not empty", "--out", "pass a new directory")


def _run(step: Step, runner: Runner) -> None:
    proc = runner(step.argv)
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout).strip().splitlines()
        last = detail[-1] if detail else f"exit code {proc.returncode}"
        raise fail(
            STEP_FAILED, f"{step.name} failed: {last}", step.name, "fix the error it reported"
        )


def _read_options(out: Path) -> dict[str, object]:
    """Read the runtime options ``generate`` wrote, and remove them from the tree."""
    options_path = out / CSHARP_RUNTIME_OPTIONS
    if not options_path.is_file():
        raise fail(
            NO_RUNTIME,
            f"{options_path} is missing: ferrule-gen did not write the C# runtime options",
            "generate",
            "use a ferrule-gen built with the C# plugin",
        )
    options = cast("dict[str, object]", json.loads(options_path.read_text(encoding="utf-8")))
    options_path.unlink()
    options_path.parent.rmdir()
    return options


def generate_csharp(
    request: CSharpRequest,
    runner: Runner = run_subprocess,
    renderer: CSharpRenderer | None = None,
) -> GenerateResult:
    """Run the C# pipeline into ``request.out``."""
    _check(request)
    if renderer is None:
        raise fail(
            NO_RUNTIME,
            "no C# runtime: ferrule_runtime.render_csharp_files is unavailable",
            "ferrule_runtime",
            "install the ferrule-runtime package",
        )
    request.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ferrule-") as tmp:
        convert, generate, csharpier = plan_csharp_steps(request, Path(tmp))
        _run(convert, runner)
        _run(generate, runner)
    options = _read_options(request.out)
    _run(csharpier, runner)
    files = renderer(options)
    for rel, text in files.items():
        path = ensure_inside(request.out, PurePosixPath(rel))
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(text.encode("utf-8"))
    return GenerateResult(
        package=str(options.get("namespace", "")),
        files=len(relative_files(request.out)),
        runtime_files=len(files),
        digest=tree_digest(request.out),
    )
