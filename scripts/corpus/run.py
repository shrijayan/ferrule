# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Corpus harness: generate an SDK from each pinned corpus spec and run the compile gate.

Usage: uv run python -m scripts.corpus.run [--tier 1] [--spec ID ...]

Per spec, each stage is timed and limited, and the run stops at the first failing stage:

1. fetch: ``corpus/fetch.py`` (pinned, sha256-verified); YAML is parsed to JSON with the
   converter's ``yaml`` package (scripts/corpus/to_json.mjs);
2. a minimal stainless.yml is synthesized (scripts/corpus/harness.py documents the rules);
3. convert: the converter CLI; 4. forge: Forge with the Python plugin (a failure whose output
   points at the WASM plugin is recorded as ``plugin``);
5. runtime: ``ferrule_runtime.render`` plus the import sort and ``ruff format`` of
   ``ferrule generate``;
6. compile: ruff, pyright and mypy with the generated SDK's own configuration (ADR 0012), in a
   venv holding only the SDK's declared dependencies;
7. wheel: ``uv build --wheel``.

Specs are third-party data: they are hashed, parsed and passed to our tools, never executed.
Reports go to reports/corpus/<id>.json and reports/corpus/summary.md.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path
from typing import TYPE_CHECKING, cast

from corpus.fetch import CACHE, MANIFEST, parse_manifest
from ferrule.errors import FerruleError
from ferrule.generate import (
    GenerateRequest,
    format_step,
    load_renderer,
    pick_package,
    plan_steps,
    relative_files,
    resolve_tools,
    sort_imports_step,
    symlinks,
)
from scripts.corpus.harness import failed_at, fer_codes, reached, summarize, synthesize

if TYPE_CHECKING:
    from collections.abc import Sequence

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / ".cache" / "corpus"
REPORTS = ROOT / "reports" / "corpus"
PYTHON = "3.12"
MAX_SPEC_BYTES = 64 << 20
MAX_FILES = 100_000
MAX_TREE_BYTES = 1 << 30
TIMEOUTS = {
    "fetch": 600,
    "convert": 900,
    "forge": 1800,
    "runtime": 900,
    "deps": 600,
    "ruff": 900,
    "pyright": 2400,
    "mypy": 2400,
    "wheel": 900,
}
TAIL = 4000


def log(msg: str) -> None:
    """Progress line on stderr."""
    sys.stderr.write(f"corpus: {msg}\n")


class StageError(Exception):
    """A stage failed; ``detail`` is recorded in the report."""

    def __init__(self, detail: str) -> None:
        """Keep the detail."""
        super().__init__(detail)
        self.detail = detail


def run(
    argv: Sequence[str], timeout: float, cwd: Path | None = None, env: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    """Run a command with a timeout, capturing text output; a timeout is a returncode of -9."""
    try:
        return subprocess.run(  # noqa: S603
            list(argv),
            capture_output=True,
            text=True,
            check=False,
            cwd=cwd,
            env=env,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        out = exc.stdout.decode() if isinstance(exc.stdout, bytes) else (exc.stdout or "")
        return subprocess.CompletedProcess(list(argv), -9, out, f"timed out after {timeout}s")


class Spec:
    """One spec's run: stage records, captured output and counts."""

    def __init__(self, name: str, fmt: str) -> None:
        """Start an empty report for ``name``."""
        self.name = name
        self.fmt = fmt
        self.work = WORK / name
        self.stages: dict[str, dict[str, object]] = {}
        self.outputs: list[str] = []
        self.counts: dict[str, object] = {}

    def stage(self, name: str, fn: object) -> bool:
        """Run ``fn`` as stage ``name``; record ok, seconds and any error."""
        assert callable(fn)  # noqa: S101
        log(f"{self.name}: {name}")
        start = time.monotonic()
        record: dict[str, object] = {}
        try:
            extra = fn()
            record["ok"] = True
            if isinstance(extra, dict):
                record.update(cast("dict[str, object]", extra))
        except StageError as exc:
            record["ok"] = False
            record["error"] = exc.detail[-TAIL:]
        except FerruleError as exc:
            record["ok"] = False
            record["error"] = exc.diagnostic.render()
            self.outputs.append(record["error"])  # type: ignore[arg-type]
        record["seconds"] = round(time.monotonic() - start, 1)
        self.stages[name] = record
        return record["ok"] is True

    def check(self, proc: subprocess.CompletedProcess[str], what: str) -> str:
        """Record the output of ``proc``; raise unless it succeeded."""
        text = (proc.stdout or "") + (proc.stderr or "")
        self.outputs.append(text)
        if proc.returncode != 0:
            msg = f"{what} exited {proc.returncode}:\n{text.strip()[-TAIL:]}"
            raise StageError(msg)
        return text


def fetch(spec: Spec) -> dict[str, object]:
    """Fetch the pinned file and make a JSON copy for the harness."""
    spec.check(
        run([sys.executable, str(ROOT / "corpus" / "fetch.py"), spec.name], TIMEOUTS["fetch"]),
        "fetch.py",
    )
    raw = CACHE / "tier1" / f"{spec.name}.{spec.fmt}"
    size = raw.stat().st_size
    if size > MAX_SPEC_BYTES:
        msg = f"{raw} is {size} bytes, over the {MAX_SPEC_BYTES} limit"
        raise StageError(msg)
    as_json = spec.work / "spec.json"
    if spec.fmt == "json":
        shutil.copyfile(raw, as_json)
    else:
        spec.check(
            run(
                ["node", str(ROOT / "scripts" / "corpus" / "to_json.mjs"), str(raw), str(as_json)],
                TIMEOUTS["fetch"],
            ),
            "to_json.mjs",
        )
    return {"bytes": size}


def synth(spec: Spec) -> dict[str, object]:
    """Write the synthesized stainless.yml (JSON is YAML)."""
    try:
        doc: object = json.loads((spec.work / "spec.json").read_text())
    except (ValueError, OSError) as exc:
        msg = f"spec is not parseable: {exc}"
        raise StageError(msg) from exc
    if not isinstance(doc, dict):
        msg = "spec root is not a mapping"
        raise StageError(msg)
    result = synthesize(cast("dict[str, object]", doc), spec.name)
    (spec.work / "stainless.yml").write_text(json.dumps(result.config, indent=2) + "\n")
    spec.counts.update(
        operations=result.operations,
        resources=result.resources,
        methods=result.methods,
        renamed_methods=len(result.renamed),
    )
    return {}


def tree_limits(out: Path) -> dict[str, object]:
    """Refuse oversized output or symlinks in it; return file count and bytes."""
    linked = symlinks(out)
    if linked:
        msg = f"symlinks in generated output: {linked[:5]}"
        raise StageError(msg)
    files = relative_files(out)
    size = sum((out / f).stat().st_size for f in files)
    if len(files) > MAX_FILES or size > MAX_TREE_BYTES:
        msg = f"generated tree too large: {len(files)} files, {size} bytes"
        raise StageError(msg)
    return {"files": len(files), "bytes": size}


def generate_stages(spec: Spec, out: Path) -> bool:
    """Stages convert, forge/plugin and runtime: ``ferrule generate`` split per step."""
    shutil.rmtree(out, ignore_errors=True)
    out.mkdir(parents=True)
    tools = resolve_tools(os.environ)
    request = GenerateRequest(spec.work / "spec.json", spec.work / "stainless.yml", out, tools)
    convert, forge = plan_steps(request, spec.work)

    def do_convert() -> dict[str, object]:
        text = spec.check(run(convert.argv, TIMEOUTS["convert"]), "converter")
        return {"diagnostics": text.count("\n")}

    if not spec.stage("convert", do_convert):
        return False

    def do_forge() -> dict[str, object]:
        spec.check(run(forge.argv, TIMEOUTS["forge"]), "forge")
        found = tree_limits(out)
        spec.counts["files"] = found["files"]
        return found

    if not spec.stage("forge", do_forge):
        err = str(spec.stages["forge"].get("error", "")).lower()
        if "wasm" in err or "plugin" in err:
            spec.stages["plugin"] = spec.stages.pop("forge")
            spec.stages["forge"] = {"ok": True, "note": "failure is inside the plugin"}
        return False
    spec.stages["plugin"] = {"ok": True, "seconds": 0, "note": "runs inside forge"}

    def do_runtime() -> dict[str, object]:
        renderer = load_renderer()
        if renderer is None:
            msg = "ferrule_runtime.render is unavailable"
            raise StageError(msg)
        src = out / "src"
        package = pick_package(
            [p.name for p in src.iterdir() if (p / "__init__.py").is_file()]
            if src.is_dir()
            else [],
            "--out",
        )
        renderer(package, src / package)
        for step in (sort_imports_step(out, tools), format_step(out, tools)):
            spec.check(run(step.argv, TIMEOUTS["runtime"]), step.name)
        found = tree_limits(out)
        spec.counts["files"] = found["files"]
        return {"package": package, **found}

    return spec.stage("runtime", do_runtime)


def count_errors(text: str, tool: str) -> int:
    """Error count from a checker's output (best effort)."""
    lines = text.splitlines()
    if tool == "pyright":
        return sum(1 for line in lines if " - error:" in line)
    if tool == "mypy":
        return sum(1 for line in lines if ": error:" in line)
    return sum(1 for line in lines if line[:1] not in {"", " "} and ":" in line and "[" not in line)


def compile_stage(spec: Spec, out: Path) -> dict[str, object]:
    """ruff, pyright and mypy with the SDK's own config, in a venv of its declared deps."""
    venv = spec.work / "venv"
    shutil.rmtree(venv, ignore_errors=True)
    spec.check(
        run(["uv", "venv", "--quiet", "--python", PYTHON, str(venv)], TIMEOUTS["deps"]), "uv venv"
    )
    py = str(venv / "bin" / "python")
    pyproject = out / "pyproject.toml"
    deps: list[str] = []
    if pyproject.is_file():
        project = cast("dict[str, object]", tomllib.loads(pyproject.read_text()).get("project", {}))
        deps = [str(d) for d in cast("list[object]", project.get("dependencies", []))]
    if deps:
        spec.check(
            run(["uv", "pip", "install", "--quiet", "--python", py, *deps], TIMEOUTS["deps"]),
            "uv pip install",
        )
    bindir = Path(sys.executable).parent
    env = {**os.environ, "PYTHONPATH": str(out / "src")}
    checks = {
        "ruff": [str(bindir / "ruff"), "check", "--no-cache", "--output-format", "concise", "."],
        "pyright": [str(bindir / "pyright"), "--pythonpath", py],
        "mypy": [str(bindir / "mypy"), "--python-executable", py, "--no-incremental", "."],
    }
    results: dict[str, object] = {}
    failed: list[str] = []
    for tool, argv in checks.items():
        log(f"{spec.name}: compile: {tool}")
        proc = run(argv, TIMEOUTS[tool], cwd=out, env=env)
        text = proc.stdout + proc.stderr
        (spec.work / f"{tool}.log").write_text(text)
        results[tool] = {"exit": proc.returncode, "errors": count_errors(text, tool)}
        if proc.returncode != 0:
            failed.append(f"{tool} exited {proc.returncode}: {text.strip()[-1500:]}")
    if failed:
        raise StageError("\n".join(failed))
    return results


def wheel_stage(spec: Spec, out: Path) -> dict[str, object]:
    """``uv build --wheel`` into the work dir."""
    dist = spec.work / "dist"
    shutil.rmtree(dist, ignore_errors=True)
    spec.check(
        run(
            ["uv", "build", "--wheel", "--quiet", "--out-dir", str(dist), str(out)],
            TIMEOUTS["wheel"],
        ),
        "uv build",
    )
    return {"wheels": sorted(p.name for p in dist.glob("*.whl"))}


def run_spec(name: str, fmt: str) -> dict[str, object]:
    """Every stage for one spec; returns the report."""
    spec = Spec(name, fmt)
    shutil.rmtree(spec.work, ignore_errors=True)
    spec.work.mkdir(parents=True)
    start = time.monotonic()
    out = spec.work / "sdk"
    ok = spec.stage("fetch", lambda: fetch(spec)) and spec.stage("synthesize", lambda: synth(spec))
    ok = ok and generate_stages(spec, out)
    ok = ok and spec.stage("compile", lambda: compile_stage(spec, out))
    ok = ok and spec.stage("wheel", lambda: wheel_stage(spec, out))
    report: dict[str, object] = {
        "spec": name,
        "stages": spec.stages,
        "counts": spec.counts,
        "fer_codes": fer_codes(spec.outputs),
        "seconds": round(time.monotonic() - start, 1),
    }
    report["reached"] = reached(report)
    report["failed_at"] = failed_at(report)
    if spec.stages.get("synthesize", {}).get("ok") is False:
        report["failed_at"] = "synthesize"
    return report


def main(argv: list[str] | None = None) -> int:
    """Run the corpus; exit 0 when every selected spec passes every stage."""
    parser = argparse.ArgumentParser(
        prog="corpus", description=__doc__.splitlines()[0] if __doc__ else None
    )
    parser.add_argument("--tier", type=int, choices=[1], default=1)
    parser.add_argument("--spec", action="append", default=[], help="spec id (repeatable)")
    args = parser.parse_args(argv)
    manifest = parse_manifest(MANIFEST.read_text())
    wanted = cast("list[str]", args.spec)
    specs = [s for s in manifest.tier1 if not wanted or s.name in wanted]
    unknown = set(wanted) - {s.name for s in specs}
    if unknown:
        sys.stderr.write(f"corpus: unknown spec ids: {', '.join(sorted(unknown))}\n")
        return 2
    REPORTS.mkdir(parents=True, exist_ok=True)
    for s in specs:
        report = run_spec(s.name, s.format)
        (REPORTS / f"{s.name}.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    reports = [
        cast("dict[str, object]", json.loads(p.read_text()))
        for p in sorted(REPORTS.glob("*.json"))
        if p.name != "summary.json"
    ]
    table = summarize(reports)
    (REPORTS / "summary.md").write_text(table)
    sys.stdout.write(table)
    selected = [r for r in reports if r["spec"] in {s.name for s in specs}]
    return 0 if all(r["reached"] == "wheel" for r in selected) else 1


if __name__ == "__main__":
    sys.exit(main())
