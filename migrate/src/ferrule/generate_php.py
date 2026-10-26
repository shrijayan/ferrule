# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule generate --target php``: spec + stainless.yml -> a PHP package (Stainless layout).

Pipeline: ``ferrule-gen convert`` -> ``ferrule-gen generate --target php`` (the PHP plugin's
files plus ``.ferrule/php-runtime.json``, the runtime's options) -> the package's
``scripts/format`` on the generated files: php-cs-fixer with the generated
``.php-cs-fixer.dist.php``, run in the package directory as that script runs it -> the PHP
runtime, rendered by ``ferrule_runtime.render_php_files`` from those options (already in the
formatted layout, so it is not formatted again).

php-cs-fixer is the version ``crates/php-plugin/tools/composer.lock`` pins, which
``mise run php:tools`` installs into ``.cache/php-tools`` and ``crates/php-plugin/tools/bin`` runs;
``FERRULE_PHP_CS_FIXER`` overrides it. As in :mod:`ferrule.generate`, planning is pure and
:func:`generate_php` is the only function that touches the file system or runs processes; it
takes the runner and the renderer as parameters.
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

PHP_RUNTIME_OPTIONS = ".ferrule/php-runtime.json"
"""Where ``ferrule-gen generate --target php`` writes the runtime options (from ``--out``)."""

FORMATTER_BIN = REPO_ROOT / "crates/php-plugin/tools/bin"
"""The wrapper that runs the php-cs-fixer ``mise run php:tools`` installs."""


class PhpRenderer(Protocol):
    """``ferrule_runtime.render_php_files`` over the options' JSON object."""

    def __call__(self, options: Mapping[str, object], /) -> Mapping[str, str]:
        """Render the runtime: ``{path from the package root: text}``."""
        ...


@dataclass(frozen=True)
class PhpRequest:
    """Inputs of one ``ferrule generate --target php`` run."""

    spec: Path
    config: Path
    out: Path
    gen: Path
    fixer: tuple[str, ...] = ("php-cs-fixer",)


def plan_php_steps(request: PhpRequest, work: Path) -> tuple[Step, Step]:
    """Plan ``ferrule-gen convert`` and ``generate --target php`` (annotated spec in ``work``)."""
    annotated = work / "annotated.json"
    gen = str(request.gen)
    convert = Step(
        "convert", (gen, "convert", str(request.spec), str(request.config), "-o", str(annotated))
    )
    generate = Step(
        "generate",
        (gen, "generate", str(annotated), "--target", "php", "--out", str(request.out)),
    )
    return convert, generate


def format_step(request: PhpRequest) -> Step:
    """Plan the package's ``scripts/format``: php-cs-fixer in the package directory.

    Its config and ``composer.json`` are read from there; no cache file is written.
    """
    script = 'cd "$0" && exec "$@"'
    return Step(
        "php-cs-fixer",
        ("sh", "-c", script, str(request.out), *request.fixer, "fix", "--using-cache=no"),
    )


def load_php_renderer() -> PhpRenderer | None:
    """``render_php_files(PhpOptions(**options))`` when the runtime package provides it."""
    try:
        module = importlib.import_module("ferrule_runtime")
    except ImportError:
        return None
    options_type: object = getattr(module, "PhpOptions", None)
    render: object = getattr(module, "render_php_files", None)
    if not callable(options_type) or not callable(render):
        return None

    def php_renderer(options: Mapping[str, object], /) -> Mapping[str, str]:
        return cast("Mapping[str, str]", render(options_type(**options)))

    return php_renderer


def _check(request: PhpRequest) -> None:
    for label, path in (("--spec", request.spec), ("--config", request.config)):
        if not path.is_file():
            raise fail(MISSING_INPUT, f"{path} does not exist", label, "pass an existing file")
    if not request.gen.is_file():
        hint = "run `cargo build --release -p ferrule-gen`, or set FERRULE_GEN"
        raise fail(TOOL_NOT_FOUND, f"{request.gen} does not exist", "FERRULE_GEN", hint)
    if not request.fixer or shutil.which(request.fixer[0]) is None:
        hint = (
            "install the package's formatter (`mise run php:tools` in the Ferrule repository) "
            "or set FERRULE_PHP_CS_FIXER, such as `vendor/bin/php-cs-fixer`"
        )
        cmd = " ".join(request.fixer)
        raise fail(TOOL_NOT_FOUND, f"{cmd} is not on PATH", "FERRULE_PHP_CS_FIXER", hint)
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
    options_path = out / PHP_RUNTIME_OPTIONS
    if not options_path.is_file():
        raise fail(
            NO_RUNTIME,
            f"{options_path} is missing: ferrule-gen did not write the PHP runtime options",
            "generate",
            "use a ferrule-gen built with the PHP plugin",
        )
    options = cast("dict[str, object]", json.loads(options_path.read_text(encoding="utf-8")))
    options_path.unlink()
    options_path.parent.rmdir()
    return options


def generate_php(
    request: PhpRequest,
    runner: Runner = run_subprocess,
    renderer: PhpRenderer | None = None,
) -> GenerateResult:
    """Run the PHP pipeline into ``request.out``."""
    _check(request)
    if renderer is None:
        raise fail(
            NO_RUNTIME,
            "no PHP runtime: ferrule_runtime.render_php_files is unavailable",
            "ferrule_runtime",
            "install the ferrule-runtime package",
        )
    request.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ferrule-") as tmp:
        for step in plan_php_steps(request, Path(tmp)):
            _run(step, runner)
    options = _read_options(request.out)
    if relative_files(request.out):
        _run(format_step(request), runner)
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
