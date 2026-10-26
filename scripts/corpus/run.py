# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Corpus harness: generate an SDK from each pinned corpus spec and check it end to end.

Usage: uv run python -m scripts.corpus.run [--tier 1] [--spec ID ...]

Per spec, each stage is timed and limited:

1. fetch: ``corpus/fetch.py`` (pinned, sha256-verified); YAML is parsed to JSON with the
   converter's ``yaml`` package (scripts/corpus/to_json.mjs);
2. a minimal stainless.yml is synthesized (scripts/corpus/harness.py documents the rules);
3. convert: ``ferrule-gen convert`` (Swagger 2.0 is converted to OpenAPI 3 here, FER1610);
4. generate: ``ferrule-gen generate``, Forge and the Python plugin in one process. A refused
   operation is fixed in the config as its diagnostic says (a recursive schema named as a
   model) or left out and recorded as skipped with its FER code, then generation is retried;
5. render: the whole ``ferrule generate`` (runtime, import sort, ``ruff format``) with the
   final config; its time is the generation time;
6. deps, ruff, pyright, mypy: the checks with the generated SDK's own configuration (ADR 0012),
   in a venv holding only the SDK's declared dependencies;
7. import: the package and its client import in that venv;
8. tests: the generated tests run against the pinned mock server over the converted spec, as
   the parity harness runs an oracle's tests;
9. determinism: a second ``ferrule generate`` gives the same tree digest;
10. wheel: ``uv build --wheel``;
11. compare (specs with ``compare`` in the manifest): ``ferrule verify`` against the official
    Stainless-generated SDK at its pinned commit, read statically (never imported or run),
    plus endpoint coverage from both ``api.md`` files.

Stages 6 to 11 run once render has passed, whatever an earlier one of them gave, so every
result is recorded; ``reached`` is the furthest stage passed in order.

Specs are third-party data: they are hashed, parsed and passed to our tools, never executed.
Reports go to reports/corpus/<id>.json and reports/corpus/summary.md.
"""

from __future__ import annotations

import argparse
import functools
import json
import os
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path
from typing import TYPE_CHECKING, cast

from corpus.fetch import CACHE, MANIFEST, Compare, parse_manifest
from ferrule.errors import FerruleError
from ferrule.generate import (
    GenerateRequest,
    generate,
    load_renderer,
    plan_steps,
    relative_files,
    resolve_tools,
    symlinks,
)
from ferrule.verify import verify
from scripts.corpus.harness import (
    add_model,
    api_endpoints,
    count_schemas,
    drop_endpoint,
    endpoint_key,
    failed_at,
    fer_codes,
    reached,
    recursive_schema,
    refusal,
    spec_format,
    summarize,
    synthesize,
)
from scripts.parity.harness import (
    MOCK_HOST,
    MOCK_PORT,
    lock_requirements,
    mock_command,
    parse_junit,
)
from scripts.parity.run import (
    MOCK_LOCK_TIMEOUT_S,
    MOCK_START_TIMEOUT_S,
    own_port,
    port_in_use,
    stop_mock,
    wait_for_port,
)

if TYPE_CHECKING:
    from collections.abc import Callable, Sequence

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / ".cache" / "corpus"
SDKS = ROOT / ".cache" / "corpus-sdks"
REPORTS = ROOT / "reports" / "corpus"
PYTHON = "3.12"
MAX_SPEC_BYTES = 64 << 20
MAX_FILES = 100_000
MAX_TREE_BYTES = 1 << 30
# Refusals fixed or skipped before generation gives up; each costs one convert + generate.
MAX_REFUSALS = 2000
TIMEOUTS = {
    "fetch": 600,
    "convert": 900,
    "generate": 3600,
    "render": 1800,
    "deps": 600,
    "ruff": 900,
    "pyright": 2400,
    "mypy": 2400,
    "import": 300,
    "tests": 1800,
    "wheel": 900,
    "clone": 900,
}
TAIL = 4000
GIT_SAFE = (
    "-c",
    "core.hooksPath=/dev/null",
    "-c",
    "filter.lfs.smudge=",
    "-c",
    "filter.lfs.required=false",
)


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

    def __init__(self, name: str, fmt: str, compare: Compare | None) -> None:
        """Start an empty report for ``name``."""
        self.name = name
        self.fmt = fmt
        self.compare = compare
        self.work = WORK / name
        self.out = self.work / "sdk"
        self.stages: dict[str, dict[str, object]] = {}
        self.outputs: list[str] = []
        self.counts: dict[str, object] = {}
        self.config: dict[str, object] = {}
        self.extra: dict[str, object] = {}

    def stage(self, name: str, fn: Callable[[], dict[str, object] | None]) -> bool:
        """Run ``fn`` as stage ``name``; record ok, seconds and any error."""
        log(f"{self.name}: {name}")
        start = time.monotonic()
        record: dict[str, object] = {}
        try:
            extra = fn()
            record["ok"] = True
            if extra:
                record.update(extra)
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

    @property
    def venv_python(self) -> str:
        """The check venv's interpreter."""
        return str(self.work / "venv" / "bin" / "python")

    def request(self, out: Path) -> GenerateRequest:
        """``ferrule generate`` inputs for this spec, written to ``out``."""
        tools = resolve_tools(os.environ)
        return GenerateRequest(self.work / "spec.json", self.work / "stainless.yml", out, tools)

    def write_config(self) -> None:
        """Write the synthesized stainless.yml (JSON is YAML)."""
        (self.work / "stainless.yml").write_text(json.dumps(self.config, indent=2) + "\n")


def fetch(spec: Spec) -> dict[str, object]:
    """Fetch the pinned file, make a JSON copy for the harness and synthesize the config."""
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
    try:
        doc: object = json.loads(as_json.read_text())
    except (ValueError, OSError) as exc:
        msg = f"spec is not parseable: {exc}"
        raise StageError(msg) from exc
    if not isinstance(doc, dict):
        msg = "spec root is not a mapping"
        raise StageError(msg)
    tree = cast("dict[str, object]", doc)
    result = synthesize(tree, spec.name, spec.compare.package if spec.compare else None)
    spec.config = result.config
    spec.write_config()
    spec.counts.update(
        operations=result.operations,
        schemas=count_schemas(tree),
        resources=result.resources,
        methods=result.methods,
        renamed_methods=len(result.renamed),
    )
    return {"bytes": size, "format": spec_format(tree)}


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


def convert_stage(spec: Spec) -> dict[str, object]:
    """``ferrule-gen convert`` with the synthesized config; diagnostics counted by code."""
    convert, _ = plan_steps(spec.request(spec.work / "raw"), spec.work)
    text = spec.check(run(convert.argv, TIMEOUTS["convert"]), "converter")
    codes = fer_codes([text])
    return {"codes": codes, "swagger2": "FER1610" in codes}


def generate_stage(spec: Spec) -> dict[str, object]:
    """Convert and generate until the plugin and Forge accept every configured operation.

    A refused operation is fixed as its diagnostic says (a recursive schema named as a model)
    or left out of the config, and recorded; anything that is not an operation refusal fails.
    """
    raw = spec.work / "raw"
    skipped: list[dict[str, str]] = []
    models: list[str] = []
    rounds = 0
    deadline = time.monotonic() + TIMEOUTS["generate"]
    while True:
        rounds += 1
        shutil.rmtree(raw, ignore_errors=True)
        raw.mkdir(parents=True)
        convert, gen = plan_steps(spec.request(raw), spec.work)
        proc = run(convert.argv, TIMEOUTS["convert"])
        if proc.returncode == 0:
            proc = run(gen.argv, max(1.0, deadline - time.monotonic()))
        if proc.returncode == 0:
            break
        text = proc.stdout + proc.stderr
        refused = refusal(text)
        if refused is None or len(skipped) + len(models) >= MAX_REFUSALS:
            spec.outputs.append(text)
            spec.extra.update(skipped=skipped, models_added=models)
            budget = "" if refused is None else f" (gave up after {MAX_REFUSALS} refusals)"
            by = "plugin" if "ferrule-gen: FER" in text else "forge"
            msg = f"ferrule-gen exited {proc.returncode} in {by}{budget}:\n{text.strip()[-TAIL:]}"
            raise StageError(msg)
        if time.monotonic() > deadline:
            msg = f"generation passed {TIMEOUTS['generate']}s after {rounds} rounds"
            raise StageError(msg)
        schema = recursive_schema(refused.detail)
        if schema and add_model(spec.config, refused.endpoint, schema):
            models.append(schema)
        elif drop_endpoint(spec.config, refused.endpoint):
            skipped.append(
                {"endpoint": refused.endpoint, "code": refused.code, "detail": refused.detail}
            )
        else:
            spec.outputs.append(text)
            msg = f"refusal for an endpoint not in the config: {text.strip()[-TAIL:]}"
            raise StageError(msg)
        spec.outputs.append(f"{refused.code}\n")
        spec.write_config()
    spec.extra.update(skipped=skipped, models_added=models)
    by_code: dict[str, int] = {}
    for s in skipped:
        by_code[s["code"]] = by_code.get(s["code"], 0) + 1
    spec.counts.update(skipped=len(skipped), skipped_by_code=dict(sorted(by_code.items())))
    return {"rounds": rounds, "skipped": len(skipped), "models_added": len(models)}


def render_stage(spec: Spec, out: Path) -> dict[str, object]:
    """Run the whole ``ferrule generate`` with the final config (the generation time)."""
    renderer = load_renderer()
    if renderer is None:
        msg = "ferrule_runtime.render is unavailable"
        raise StageError(msg)
    shutil.rmtree(out, ignore_errors=True)
    result = generate(spec.request(out), renderer=renderer)
    found = tree_limits(out)
    return {"package": result.package, "digest": result.digest, **found}


def scale(spec: Spec) -> None:
    """Methods the generated ``api.md`` lists, and generation seconds."""
    spec.counts["generated_methods"] = len(endpoints_of(spec.out))
    spec.counts["files"] = spec.stages["render"].get("files")


def deps_stage(spec: Spec) -> dict[str, object]:
    """Make a venv with the SDK's own dependencies and dev lock, and the SDK itself.

    The dev lock (pytest, its plugins) is what the SDK's own lint runs with: pyright checks
    ``tests/`` too. Pins are installed by name, without re-resolving and without hooks.
    """
    out = spec.out
    pyproject = out / "pyproject.toml"
    if not pyproject.is_file():
        # Checking without it would pick up Ferrule's own root config instead.
        msg = "generated SDK has no pyproject.toml, so it has no ruff/pyright/mypy config"
        raise StageError(msg)
    venv = spec.work / "venv"
    shutil.rmtree(venv, ignore_errors=True)
    py = spec.venv_python
    spec.check(
        run(["uv", "venv", "--quiet", "--python", PYTHON, str(venv)], TIMEOUTS["deps"]), "uv venv"
    )
    project = cast("dict[str, object]", tomllib.loads(pyproject.read_text()).get("project", {}))
    deps = [str(d) for d in cast("list[object]", project.get("dependencies", []))]
    lock = out / "requirements-dev.lock"
    dist = str(project.get("name", ""))
    reqs = lock_requirements(lock.read_text(), exclude=[dist]) if lock.is_file() else []
    req_file = spec.work / "requirements-dev.txt"
    req_file.write_text("\n".join(reqs or [*deps, "pytest"]) + "\n")
    no_deps = ["--no-deps"] if reqs else []
    spec.check(
        run(
            ["uv", "pip", "install", "--quiet", *no_deps, "--python", py, "-r", str(req_file)],
            TIMEOUTS["deps"],
        ),
        "uv pip install",
    )
    spec.check(
        run(
            ["uv", "pip", "install", "--quiet", "--no-deps", "--python", py, str(out)],
            TIMEOUTS["deps"],
        ),
        "uv pip install (sdk)",
    )
    return {"dependencies": len(deps), "dev_lock": len(reqs)}


def count_errors(text: str, tool: str) -> int:
    """Error count from a checker's output (best effort)."""
    lines = text.splitlines()
    if tool == "pyright":
        return sum(1 for line in lines if " - error:" in line)
    if tool == "mypy":
        return sum(1 for line in lines if ": error:" in line)
    return sum(1 for line in lines if line[:1] not in {"", " "} and ":" in line and "[" not in line)


def check_stage(spec: Spec, tool: str) -> dict[str, object]:
    """One checker with the SDK's own config (strict as the SDK sets it)."""
    bindir = Path(sys.executable).parent
    argv = {
        "ruff": [str(bindir / "ruff"), "check", "--no-cache", "--output-format", "concise", "."],
        "pyright": [str(bindir / "pyright"), "--pythonpath", spec.venv_python],
        "mypy": [
            str(bindir / "mypy"),
            "--python-executable",
            spec.venv_python,
            "--no-incremental",
            ".",
        ],
    }[tool]
    env = {**os.environ, "PYTHONPATH": str(spec.out / "src")}
    proc = run(argv, TIMEOUTS[tool], cwd=spec.out, env=env)
    text = proc.stdout + proc.stderr
    (spec.work / f"{tool}.log").write_text(text)
    errors = count_errors(text, tool)
    if proc.returncode != 0:
        msg = f"{tool} exited {proc.returncode} with {errors} errors:\n{text.strip()[-1500:]}"
        raise StageError(msg)
    return {"errors": errors}


def client_class(spec: Spec, package: str) -> str | None:
    """Return the sync client class ``__init__.py`` imports from ``._client``."""
    init = spec.out / "src" / package / "__init__.py"
    text = init.read_text() if init.is_file() else ""
    for line in text.splitlines():
        if line.startswith("from ._client import"):
            for name in (n.strip(" ()") for n in line.split("import", 1)[1].split(",")):
                if name[:1].isupper() and not name.startswith(("Async", "Request")):
                    return name
    return None


def import_stage(spec: Spec) -> dict[str, object]:
    """Import the package and build its client in the check venv (our own generated code)."""
    package = str(spec.stages["render"]["package"])
    client = client_class(spec, package)
    code = f"import sys\nsys.path.insert(0, {str(spec.out / 'src')!r})\nimport {package}\n"
    if client:
        code += f"{package}.{client}(base_url='http://{MOCK_HOST}:1', max_retries=0)\n"
    spec.check(run([spec.venv_python, "-I", "-c", code], TIMEOUTS["import"]), "import")
    return {"client": client}


def tests_stage(spec: Spec) -> dict[str, object]:
    """The generated tests against the pinned mock server over the converted spec."""
    with own_port(MOCK_PORT, MOCK_LOCK_TIMEOUT_S) as owned:
        if not owned:
            msg = f"FER4021 another run kept {MOCK_HOST}:{MOCK_PORT} for {MOCK_LOCK_TIMEOUT_S} s"
            raise StageError(msg)
        return tests_owned(spec)


def tests_owned(spec: Spec) -> dict[str, object]:
    """:func:`tests_stage` once this run owns the mock port."""
    if port_in_use(MOCK_PORT):
        msg = f"FER4021 something already listens on {MOCK_HOST}:{MOCK_PORT}"
        raise StageError(msg)
    out = spec.out
    py = spec.venv_python
    annotated = spec.work / "annotated.json"
    mock_script = out / "scripts" / "mock"
    cmd = mock_command(mock_script.read_text() if mock_script.is_file() else "", annotated)
    env = {
        **os.environ,
        "TEST_API_BASE_URL": f"http://{MOCK_HOST}:{MOCK_PORT}",
        "DEFER_PYDANTIC_BUILD": "false",
    }
    with (spec.work / "mock.log").open("wb") as mock_log:
        mock = subprocess.Popen(  # noqa: S603 - pinned mock command, spec passed as data
            cmd, stdout=mock_log, stderr=subprocess.STDOUT, start_new_session=True
        )
        try:
            if not wait_for_port(MOCK_PORT, mock, MOCK_START_TIMEOUT_S):
                tail = (spec.work / "mock.log").read_text(errors="replace")[-1500:]
                msg = f"mock server did not start: {' '.join(cmd[-1:])}\n{tail}"
                raise StageError(msg)
            junit = spec.work / "junit.xml"
            junit.unlink(missing_ok=True)
            argv = [
                py,
                "-m",
                "pytest",
                "tests",
                "-q",
                "-p",
                "no:cacheprovider",
                f"--junitxml={junit}",
            ]
            proc = run(argv, TIMEOUTS["tests"], cwd=out, env=env)
        finally:
            stop_mock(mock)
    (spec.work / "pytest.log").write_text(proc.stdout + proc.stderr)
    counts = parse_junit(junit.read_text()).as_dict() if junit.is_file() else {}
    if proc.returncode != 0:
        tail = (proc.stdout + proc.stderr).strip()[-1500:]
        msg = f"pytest exit {proc.returncode}: {counts}\n{tail}"
        spec.extra["tests"] = counts
        raise StageError(msg)
    return {"mock": "steady" if "steady" in cmd else "prism", **counts}


def determinism_stage(spec: Spec) -> dict[str, object]:
    """Check that a second ``ferrule generate`` gives the same tree digest."""
    again = spec.work / "sdk-again"
    renderer = load_renderer()
    shutil.rmtree(again, ignore_errors=True)
    second = generate(spec.request(again), renderer=renderer).digest
    first = str(spec.stages["render"]["digest"])
    shutil.rmtree(again, ignore_errors=True)
    if first != second:
        msg = f"tree digests differ: {first} then {second}"
        raise StageError(msg)
    return {"digest": first}


def wheel_stage(spec: Spec) -> dict[str, object]:
    """``uv build --wheel`` into the work dir."""
    dist = spec.work / "dist"
    shutil.rmtree(dist, ignore_errors=True)
    spec.check(
        run(
            ["uv", "build", "--wheel", "--quiet", "--out-dir", str(dist), str(spec.out)],
            TIMEOUTS["wheel"],
        ),
        "uv build",
    )
    return {"wheels": sorted(p.name for p in dist.glob("*.whl"))}


def _git(*args: str, cwd: Path | None = None) -> str:
    proc = run(["git", *GIT_SAFE, *args], TIMEOUTS["clone"], cwd=cwd)
    if proc.returncode != 0:
        msg = f"git {' '.join(args)} exited {proc.returncode}: {proc.stderr.strip()[-1000:]}"
        raise StageError(msg)
    return proc.stdout.strip()


def checkout_sdk(compare: Compare) -> Path:
    """Check out the official SDK at its pinned commit (blobless clone; never run)."""
    dest = SDKS / compare.repo.replace("/", "__")
    if not (dest / ".git").is_dir():
        dest.parent.mkdir(parents=True, exist_ok=True)
        _git(
            "clone",
            "--filter=blob:none",
            "--no-checkout",
            f"https://github.com/{compare.repo}.git",
            str(dest),
        )
    if run(["git", "cat-file", "-e", f"{compare.sha}^{{commit}}"], 60, cwd=dest).returncode:
        _git("fetch", "--filter=blob:none", "origin", compare.sha, cwd=dest)
    _git("checkout", "--quiet", "--force", "--detach", compare.sha, cwd=dest)
    if _git("rev-parse", "HEAD", cwd=dest) != compare.sha:
        msg = f"{compare.repo}: HEAD is not {compare.sha}"
        raise StageError(msg)
    return dest


def endpoints_of(tree: Path) -> set[str]:
    """Return the endpoints the tree's ``api.md`` lists (none without one)."""
    api_md = tree / "api.md"
    return api_endpoints(api_md.read_text()) if api_md.is_file() else set[str]()


def compare_stage(spec: Spec, compare: Compare) -> dict[str, object]:
    """``ferrule verify`` against the official SDK, and endpoint coverage from both api.md."""
    sdk = checkout_sdk(compare)
    report = verify(sdk, spec.out, compare.package)
    (spec.work / "verify.json").write_text(json.dumps(report.to_json(), indent=2) + "\n")
    theirs, ours = endpoints_of(sdk), endpoints_of(spec.out)
    in_spec = {
        endpoint_key(str(e))
        for r in cast("dict[str, dict[str, object]]", spec.config["resources"]).values()
        for e in cast("dict[str, str]", r.get("methods") or {}).values()
    }
    in_spec |= {
        endpoint_key(s["endpoint"])
        for s in cast("list[dict[str, str]]", spec.extra.get("skipped", []))
    }
    return {
        "repo": compare.repo,
        "sha": compare.sha,
        "package": compare.package,
        "breaking": len(report.breaking),
        "additive": len(report.additive),
        "closeness": round(report.closeness, 4),
        "endpoints_official": len(theirs),
        "endpoints_ours": len(ours),
        "endpoints_shared": len(theirs & ours),
        "endpoints_official_in_spec": len(theirs & in_spec),
    }


def run_spec(name: str, fmt: str, compare: Compare | None) -> dict[str, object]:
    """Every stage for one spec; returns the report."""
    spec = Spec(name, fmt, compare)
    shutil.rmtree(spec.work, ignore_errors=True)
    spec.work.mkdir(parents=True)
    start = time.monotonic()
    ok = spec.stage("fetch", lambda: fetch(spec))
    ok = ok and spec.stage("convert", lambda: convert_stage(spec))
    ok = ok and spec.stage("generate", lambda: generate_stage(spec))
    ok = ok and spec.stage("render", lambda: render_stage(spec, spec.out))
    if ok:
        scale(spec)
        if spec.stage("deps", lambda: deps_stage(spec)):
            for tool in ("ruff", "pyright", "mypy"):
                spec.stage(tool, functools.partial(check_stage, spec, tool))
            if spec.stage("import", lambda: import_stage(spec)):
                spec.stage("tests", lambda: tests_stage(spec))
        spec.stage("determinism", lambda: determinism_stage(spec))
        spec.stage("wheel", lambda: wheel_stage(spec))
        if compare is not None:
            spec.stage("compare", lambda: compare_stage(spec, compare))
    report: dict[str, object] = {
        "spec": name,
        "stages": spec.stages,
        "counts": spec.counts,
        "skipped": spec.extra.get("skipped", []),
        "models_added": spec.extra.get("models_added", []),
        "fer_codes": fer_codes(spec.outputs),
        "seconds": round(time.monotonic() - start, 1),
    }
    report["reached"] = reached(report)
    report["failed_at"] = failed_at(report)
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
        report = run_spec(s.name, s.format, s.compare)
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
