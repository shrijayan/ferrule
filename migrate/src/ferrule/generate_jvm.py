# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule generate --target java|kotlin``: spec + stainless.yml -> a JVM SDK tree.

Pipeline: ``ferrule-gen convert`` -> ``ferrule-gen generate --target java|kotlin`` (the JVM
plugin's files, their Kotlin not yet laid out, plus ``.ferrule/jvm-runtime.json``, the runtime's
options) -> ``ktfmt --kotlinlang-style`` over the plugin's Kotlin files (the formatter and version
the Stainless JVM SDKs pin) -> the JVM runtime, rendered by ``ferrule_runtime.render_java_files``
from those options (already in ktfmt's layout) -> the scripts made executable.

As in :mod:`ferrule.generate`, planning is pure and :func:`generate_jvm` is the only function that
touches the file system or runs processes; it takes the runner and the renderer as parameters.
"""

from __future__ import annotations

import importlib
import json
import shutil
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING, Literal, Protocol, cast

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

JVM_RUNTIME_OPTIONS = ".ferrule/jvm-runtime.json"
"""Where ``ferrule-gen generate --target java|kotlin`` writes the runtime options."""

EXECUTABLES = (
    "bin/check-release-environment",
    "gradlew",
    "scripts/build",
    "scripts/fast-format",
    "scripts/format",
    "scripts/java-format",
    "scripts/kotlin-format",
    "scripts/lint",
    "scripts/mock",
    "scripts/test",
)
"""Generated files that are executable scripts, where present."""

KTFMT_BATCH = 200
"""Kotlin files per ktfmt run, to stay under the argument-length limit."""

type JvmTarget = Literal["java", "kotlin"]


class JavaRenderer(Protocol):
    """``ferrule_runtime.render_java_files`` over the options' JSON object."""

    def __call__(self, options: Mapping[str, object], /) -> Mapping[str, str]:
        """Render the runtime: ``{path from the Gradle root: text}``."""
        ...


@dataclass(frozen=True)
class JvmRequest:
    """Inputs of one ``ferrule generate --target java|kotlin`` run."""

    spec: Path
    config: Path
    out: Path
    gen: Path
    target: JvmTarget = "java"
    ktfmt: tuple[str, ...] = ("ktfmt",)
    """The ktfmt command: ``ktfmt``, or ``java -jar ktfmt-0.61-with-dependencies.jar``."""


def resolve_ktfmt(env: Mapping[str, str]) -> tuple[str, ...]:
    """``$FERRULE_KTFMT_JAR`` run with ``java``, else ``$FERRULE_KTFMT``, else ``ktfmt``."""
    jar = env.get("FERRULE_KTFMT_JAR")
    if jar:
        return (env.get("FERRULE_JAVA", "java"), "-jar", jar)
    return (env.get("FERRULE_KTFMT", "ktfmt"),)


def plan_jvm_steps(request: JvmRequest, work: Path) -> tuple[Step, Step]:
    """Plan ``ferrule-gen convert`` and ``generate --target``, the annotated spec in ``work``."""
    annotated = work / "annotated.json"
    gen = str(request.gen)
    convert = Step(
        "convert", (gen, "convert", str(request.spec), str(request.config), "-o", str(annotated))
    )
    generate = Step(
        "generate",
        (gen, "generate", str(annotated), "--target", request.target, "--out", str(request.out)),
    )
    return convert, generate


def ktfmt_steps(request: JvmRequest, files: Sequence[PurePosixPath]) -> list[Step]:
    """``ktfmt --kotlinlang-style`` over ``files`` (relative to ``--out``), in batches."""
    paths = [str(request.out / f) for f in files]
    return [
        Step("ktfmt", (*request.ktfmt, "--kotlinlang-style", *paths[i : i + KTFMT_BATCH]))
        for i in range(0, len(paths), KTFMT_BATCH)
    ]


def load_java_renderer() -> JavaRenderer | None:
    """``render_java_files(JavaOptions(**options))`` when the runtime package provides it."""
    try:
        module = importlib.import_module("ferrule_runtime")
    except ImportError:
        return None
    options_type: object = getattr(module, "JavaOptions", None)
    render: object = getattr(module, "render_java_files", None)
    if not callable(options_type) or not callable(render):
        return None

    def java_renderer(options: Mapping[str, object], /) -> Mapping[str, str]:
        return cast("Mapping[str, str]", render(options_type(**options)))

    return java_renderer


def _check(request: JvmRequest) -> None:
    for label, path in (("--spec", request.spec), ("--config", request.config)):
        if not path.is_file():
            raise fail(MISSING_INPUT, f"{path} does not exist", label, "pass an existing file")
    if not request.gen.is_file():
        hint = "run `cargo build --release -p ferrule-gen`, or set FERRULE_GEN"
        raise fail(TOOL_NOT_FOUND, f"{request.gen} does not exist", "FERRULE_GEN", hint)
    if shutil.which(request.ktfmt[0]) is None:
        hint = "install ktfmt 0.61 (`mise install`) and set FERRULE_KTFMT_JAR or FERRULE_KTFMT"
        raise fail(TOOL_NOT_FOUND, f"{request.ktfmt[0]} is not on PATH", "FERRULE_KTFMT", hint)
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


def _render_runtime(out: Path, renderer: JavaRenderer) -> int:
    """Render the runtime from the options ``generate`` wrote, then remove them."""
    options_path = out / JVM_RUNTIME_OPTIONS
    if not options_path.is_file():
        raise fail(
            NO_RUNTIME,
            f"{options_path} is missing: ferrule-gen did not write the JVM runtime options",
            "generate",
            "use a ferrule-gen built with the JVM plugin",
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


def _make_executable(out: Path) -> None:
    for rel in EXECUTABLES:
        path = out / rel
        if path.is_file():
            path.chmod(0o755)


def generate_jvm(
    request: JvmRequest,
    runner: Runner = run_subprocess,
    renderer: JavaRenderer | None = None,
) -> GenerateResult:
    """Run the JVM pipeline into ``request.out``."""
    _check(request)
    if renderer is None:
        raise fail(
            NO_RUNTIME,
            "no JVM runtime: ferrule_runtime.render_java_files is unavailable",
            "ferrule_runtime",
            "install the ferrule-runtime package",
        )
    request.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ferrule-") as tmp:
        for step in plan_jvm_steps(request, Path(tmp)):
            _run(step, runner)
    kotlin = [f for f in relative_files(request.out) if f.suffix == ".kt"]
    for step in ktfmt_steps(request, kotlin):
        _run(step, runner)
    runtime = _render_runtime(request.out, renderer)
    _make_executable(request.out)
    return GenerateResult(
        package=_group(request.out),
        files=len(relative_files(request.out)),
        runtime_files=runtime,
        digest=tree_digest(request.out),
    )


def _group(out: Path) -> str:
    """Read the Maven group from the root ``build.gradle.kts``."""
    build = out / "build.gradle.kts"
    if build.is_file():
        for line in build.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if stripped.startswith("group = "):
                return stripped.removeprefix("group = ").strip('"')
    return ""
