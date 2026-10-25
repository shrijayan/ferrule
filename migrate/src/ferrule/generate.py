# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule generate``: spec + stainless.yml -> a formatted Python SDK tree.

Pipeline: ``ferrule-gen convert`` -> ``ferrule-gen generate`` (Forge with the Python plugin,
in one native binary, ADR 0014) -> runtime -> ruff import sort and ``ruff format``.

The planning functions here are pure; :func:`generate` is the only place that touches the
file system or runs processes, and it takes the process runner and the runtime renderer as
parameters so tests can replace both.
"""

from __future__ import annotations

import hashlib
import importlib
import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING, Protocol, cast

from ferrule.errors import (
    MISSING_INPUT,
    NO_PACKAGE,
    NO_RUNTIME,
    OUT_NOT_EMPTY,
    STEP_FAILED,
    SYMLINK_REFUSED,
    TOOL_NOT_FOUND,
    fail,
)

if TYPE_CHECKING:
    from collections.abc import Callable, Iterable, Mapping, Sequence

REPO_ROOT = Path(__file__).resolve().parents[3]

# Runtime modules of a Stainless-style package, relative to src/<package>/. In adopt mode
# (ADR 0004) these are copied from the customer's own SDK; everything else is generated.
RUNTIME_FILES = frozenset(
    {
        "_base_client.py",
        "_compat.py",
        "_constants.py",
        "_exceptions.py",
        "_files.py",
        "_models.py",
        "_qs.py",
        "_resource.py",
        "_response.py",
        "_streaming.py",
        "_types.py",
        "py.typed",
    },
)
RUNTIME_DIRS = frozenset({"_utils"})
SKIP_PARTS = frozenset({"__pycache__", ".ruff_cache", ".mypy_cache", ".pytest_cache", ".git"})


class RuntimeRenderer(Protocol):
    """What ``ferrule_runtime.render`` must look like for ``generate`` to use it.

    It writes the runtime modules for ``package`` into ``package_dir`` (``<out>/src/<package>``).
    """

    def __call__(self, package: str, package_dir: Path, /) -> object:
        """Render the runtime for ``package`` into ``package_dir``."""
        ...


type Runner = Callable[[Sequence[str]], subprocess.CompletedProcess[str]]


@dataclass(frozen=True)
class Tools:
    """Executables the pipeline runs."""

    gen: Path
    ruff: str


@dataclass(frozen=True)
class GenVersion:
    """What ``ferrule-gen --version`` reports."""

    version: str
    forge: str

    @property
    def generator(self) -> str:
        """``ferrule-gen <version>``, as ``ferrule.lock`` records it."""
        return f"ferrule-gen {self.version}"


@dataclass(frozen=True)
class Step:
    """One external command in the pipeline."""

    name: str
    argv: tuple[str, ...]


@dataclass(frozen=True)
class GenerateRequest:
    """Inputs of one ``ferrule generate`` run."""

    spec: Path
    config: Path
    out: Path
    tools: Tools
    runtime_from: Path | None = None


@dataclass(frozen=True)
class GenerateResult:
    """What a run produced."""

    package: str
    files: int
    runtime_files: int
    digest: str


def resolve_tools(
    env: Mapping[str, str],
    root: Path = REPO_ROOT,
    gen: Path | None = None,
) -> Tools:
    """Pick tool paths: explicit argument, then ``FERRULE_*`` env var, then repo default."""
    return Tools(
        gen=gen or Path(env.get("FERRULE_GEN") or root / "target/release/ferrule-gen"),
        ruff=env.get("FERRULE_RUFF", "ruff"),
    )


def plan_steps(request: GenerateRequest, work: Path) -> tuple[Step, Step]:
    """Plan ``ferrule-gen convert`` and ``generate``, the annotated spec written into ``work``."""
    annotated = work / "annotated.json"
    t = request.tools
    runtime = () if request.runtime_from is None else ("--runtime-from", str(request.runtime_from))
    convert = Step(
        "convert",
        (
            str(t.gen),
            "convert",
            str(request.spec),
            str(request.config),
            "-o",
            str(annotated),
            *runtime,
        ),
    )
    gen = Step("generate", (str(t.gen), "generate", str(annotated), "--out", str(request.out)))
    return convert, gen


def is_generate_step(argv: Sequence[str], tools: Tools) -> bool:
    """Whether ``argv`` is the ``generate`` step :func:`plan_steps` planned for ``tools``."""
    return tuple(argv[:2]) == (str(tools.gen), "generate")


def version_step(tools: Tools) -> Step:
    """``ferrule-gen --version``."""
    return Step("version", (str(tools.gen), "--version"))


_VERSION = re.compile(r"ferrule-gen (?P<version>\S+) forge (?P<forge>[0-9a-f]{40})")


def parse_gen_version(text: str) -> GenVersion | None:
    """Parse ``ferrule-gen <version> forge <40-hex rev>``; ``None`` for anything else."""
    match = _VERSION.fullmatch(text.strip())
    return None if match is None else GenVersion(match["version"], match["forge"])


def sort_imports_step(out: Path, tools: Tools) -> Step:
    """``ruff check --select I --fix`` with the SDK's own isort settings (length-sort).

    Emitters write imports in one fixed order; names differ per SDK, so the order the SDK's
    isort config wants is restored here, as Stainless's own lint-fix step does.
    """
    return Step(
        "sort-imports",
        (
            tools.ruff,
            "check",
            "--select",
            "I",
            "--fix",
            "--no-cache",
            "--quiet",
            "--exit-zero",
            str(out),
        ),
    )


def format_step(out: Path, tools: Tools) -> Step:
    """``ruff format`` over the output, without leaving a cache behind."""
    return Step("format", (tools.ruff, "format", "--no-cache", "--quiet", str(out)))


def is_runtime_path(rel: PurePosixPath) -> bool:
    """Whether ``rel`` (relative to the package dir) is a runtime module."""
    if not rel.parts or any(p in SKIP_PARTS for p in rel.parts):
        return False
    if len(rel.parts) == 1:
        return rel.name in RUNTIME_FILES
    return rel.parts[0] in RUNTIME_DIRS


def runtime_copies(
    package_files: Iterable[PurePosixPath],
    generated: Iterable[PurePosixPath],
) -> list[PurePosixPath]:
    """Runtime files to copy from an existing package; generated files always win."""
    taken = set(generated)
    return sorted(p for p in package_files if is_runtime_path(p) and p not in taken)


def kept_copies(
    package_files: Iterable[PurePosixPath],
    generated: Iterable[PurePosixPath],
) -> list[PurePosixPath]:
    """Files of an existing package to keep: everything the generator did not write.

    That is the runtime, hand-written code (``lib/``) and modules a regeneration no longer
    produces, which Stainless leaves in place (removing them would break importers). Caches
    are skipped; generated files always win.
    """
    taken = set(generated)
    return sorted(
        p
        for p in package_files
        if p not in taken and p.parts and not any(part in SKIP_PARTS for part in p.parts)
    )


def pick_package(candidates: Iterable[str], pointer: str) -> str:
    """Return the single importable package under a ``src/`` directory."""
    names = sorted(c for c in candidates if c.isidentifier())
    if len(names) != 1:
        found = ", ".join(names) or "none"
        raise fail(
            NO_PACKAGE,
            f"expected exactly one package under src/, found {found}",
            pointer,
            "point --runtime-from at a Stainless-style SDK with src/<package>/__init__.py",
        )
    return names[0]


def relative_files(root: Path) -> list[PurePosixPath]:
    """Every file under ``root`` as sorted POSIX paths, skipping caches."""
    if not root.is_dir():
        return []
    out = [
        PurePosixPath(p.relative_to(root).as_posix())
        for p in root.rglob("*")
        if p.is_file()
        and not p.is_symlink()
        and not SKIP_PARTS.intersection(p.relative_to(root).parts)
    ]
    return sorted(out)


def tree_digest(root: Path) -> str:
    """sha256 over every file's path and bytes; equal trees give equal digests."""
    h = hashlib.sha256()
    for rel in relative_files(root):
        data = (root / rel).read_bytes()
        h.update(f"{rel}\0{len(data)}\0".encode())
        h.update(data)
    return h.hexdigest()


def load_renderer() -> RuntimeRenderer | None:
    """``ferrule_runtime.render`` when the runtime package provides it."""
    try:
        module = importlib.import_module("ferrule_runtime")
    except ImportError:
        return None
    render: object = getattr(module, "render", None)
    return cast("RuntimeRenderer", render) if callable(render) else None


def run_subprocess(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
    """Run one pipeline command, capturing its output."""
    return subprocess.run(argv, capture_output=True, text=True, check=False)  # noqa: S603


def _check_inputs(request: GenerateRequest) -> None:
    for label, path in (("--spec", request.spec), ("--config", request.config)):
        if not path.is_file():
            raise fail(MISSING_INPUT, f"{path} does not exist", label, "pass an existing file")
    _check_gen(request.tools)
    ruff = request.tools.ruff
    if shutil.which(ruff) is None:
        hint = "install it or set FERRULE_RUFF"
        raise fail(TOOL_NOT_FOUND, f"{ruff} is not on PATH", "FERRULE_RUFF", hint)
    if request.out.exists() and any(request.out.iterdir()):
        raise fail(OUT_NOT_EMPTY, f"{request.out} is not empty", "--out", "pass a new directory")
    if request.runtime_from is not None and not (request.runtime_from / "src").is_dir():
        raise fail(
            MISSING_INPUT,
            f"{request.runtime_from}/src does not exist",
            "--runtime-from",
            "point it at the root of an existing SDK checkout",
        )


_BUILD_GEN = "run `cargo build --release -p ferrule-gen`, or set FERRULE_GEN"


def _check_gen(tools: Tools) -> None:
    if not tools.gen.is_file():
        raise fail(TOOL_NOT_FOUND, f"{tools.gen} does not exist", "FERRULE_GEN", _BUILD_GEN)


def _run(step: Step, runner: Runner) -> str:
    proc = runner(step.argv)
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout).strip().splitlines()
        last = detail[-1] if detail else f"exit code {proc.returncode}"
        raise fail(
            STEP_FAILED,
            f"{step.name} failed: {last}",
            step.name,
            f"fix the error {step.name} reported",
        )
    return proc.stdout


def gen_version(tools: Tools, runner: Runner = run_subprocess) -> GenVersion:
    """Run ``ferrule-gen --version``; refuse a binary that does not name its Forge rev."""
    _check_gen(tools)
    text = _run(version_step(tools), runner)
    version = parse_gen_version(text)
    if version is None:
        printed = text.strip().splitlines()[0] if text.strip() else "nothing"
        raise fail(
            TOOL_NOT_FOUND,
            f"{tools.gen} --version printed {printed!r}, "
            "expected 'ferrule-gen <version> forge <40-hex rev>'",
            "FERRULE_GEN",
            _BUILD_GEN,
        )
    return version


def _src_packages(root: Path) -> list[str]:
    src = root / "src"
    return [p.name for p in src.iterdir() if (p / "__init__.py").is_file()] if src.is_dir() else []


def symlinks(root: Path) -> list[PurePosixPath]:
    """Every symlink (file or directory) under ``root``, without following any."""
    if root.is_symlink():
        return [PurePosixPath(".")]
    found: list[PurePosixPath] = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        base = Path(dirpath)
        found.extend(
            PurePosixPath((base / name).relative_to(root).as_posix())
            for name in (*dirnames, *filenames)
            if (base / name).is_symlink()
        )
    return sorted(found)


def ensure_inside(root: Path, rel: PurePosixPath) -> Path:
    """``root / rel``, refusing any path that is or passes through a symlink or escapes ``root``."""
    path = root
    for part in rel.parts:
        path /= part
        if path.is_symlink():
            raise fail(
                SYMLINK_REFUSED,
                f"{path} is a symlink",
                "--runtime-from",
                "replace it with a regular file",
            )
    if not path.resolve().is_relative_to(
        root.resolve()
    ):  # pragma: no cover - guarded by the loop above
        raise fail(SYMLINK_REFUSED, f"{path} escapes {root}", "--runtime-from", "remove it")
    return path


def _copy_runtime(source_root: Path, out: Path) -> tuple[str, int]:
    package = pick_package(_src_packages(source_root), "--runtime-from")
    src_pkg = source_root / "src" / package
    dst_pkg = out / "src" / package
    linked = symlinks(source_root / "src")
    if linked:
        raise fail(
            SYMLINK_REFUSED,
            f"refusing symlinks in the runtime source: {', '.join(map(str, linked))}",
            "--runtime-from",
            "replace the symlinks with regular files",
        )
    copies = kept_copies(relative_files(src_pkg), relative_files(dst_pkg))
    for rel in copies:
        source = ensure_inside(src_pkg, rel)
        target = dst_pkg / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target, follow_symlinks=False)
    return package, len(copies)


def mark_scripts_executable(out: Path) -> None:
    """``chmod +x`` the files in ``scripts/``, as Stainless SDKs ship them (mode 100755)."""
    scripts = out / "scripts"
    if not scripts.is_dir():
        return
    for path in sorted(scripts.iterdir()):
        if path.is_file() and not path.is_symlink():
            path.chmod(path.stat().st_mode | 0o111)


def generate(
    request: GenerateRequest,
    runner: Runner = run_subprocess,
    renderer: RuntimeRenderer | None = None,
) -> GenerateResult:
    """Run the whole pipeline into ``request.out``."""
    _check_inputs(request)
    request.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ferrule-") as tmp:
        for step in plan_steps(request, Path(tmp)):
            _run(step, runner)
    if request.runtime_from is not None:
        package, runtime_count = _copy_runtime(request.runtime_from, request.out)
    elif renderer is not None:
        package = pick_package(_src_packages(request.out), "--out")
        package_dir = request.out / "src" / package
        before = len(relative_files(package_dir))
        renderer(package, package_dir)
        runtime_count = len(relative_files(package_dir)) - before
    else:
        raise fail(
            NO_RUNTIME,
            "no runtime source: ferrule_runtime.render is unavailable",
            "--runtime-from",
            "pass --runtime-from <existing SDK checkout> (adopt mode)",
        )
    _run(sort_imports_step(request.out, request.tools), runner)
    _run(format_step(request.out, request.tools), runner)
    mark_scripts_executable(request.out)
    return GenerateResult(
        package=package,
        files=len(relative_files(request.out)),
        runtime_files=runtime_count,
        digest=tree_digest(request.out),
    )
