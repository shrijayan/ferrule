# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Parity harness: regenerate one oracle SDK and measure it against its last Stainless release.

Usage: uv run python -m scripts.parity.run [--skip-tests] <oracle>

Generation runs ``ferrule-gen`` (``$FERRULE_GEN``, default ``target/release/ferrule-gen``; build
it with ``cargo build --release -p ferrule-gen``).

<oracle> is a group (``onebusaway``) or a config/sdk name from oracles/manifest.yaml. Steps:

1. fetch both oracle repos (oracles/fetch.py, pinned and md5-checked);
2. ``ferrule generate`` from the config oracle's spec and stainless.yml, twice, runtime copied
   from the pinned SDK (adopt mode), and compare tree digests (determinism);
3. ``ferrule verify`` against the pinned SDK (griffe API diff, text closeness);
4. copy the oracle's tests/ into the generated tree, build a uv venv with the generated
   package plus the oracle's dev dependencies by name (no oracle scripts or hooks run),
   start the pinned mock server against the oracle spec and run pytest;
5. write reports/parity/<oracle>.json.

The report is written even when a step fails; the step's error is recorded in it.
"""

from __future__ import annotations

import argparse
import contextlib
import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import time
from pathlib import Path

from ferrule.errors import FerruleError
from ferrule.generate import GenerateRequest, generate, resolve_tools, symlinks, tree_digest
from ferrule.verify import verify
from oracles.fetch import CACHE, parse_manifest, spec_file
from scripts.parity.harness import (
    MOCK_HOST,
    MOCK_PORT,
    Outcomes,
    breakage_kinds,
    lock_requirements,
    mock_command,
    parse_junit,
    resolve_pair,
)

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / ".cache" / "parity"
REPORTS = ROOT / "reports" / "parity"
PYTHON = "3.12"
TEST_TIMEOUT_S = 1800
MOCK_START_TIMEOUT_S = 180


def log(msg: str) -> None:
    """Progress line on stderr."""
    sys.stderr.write(f"parity: {msg}\n")


def run(
    argv: list[str],
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
    timeout: float | None = None,
) -> subprocess.CompletedProcess[str]:
    """Run a command, capturing text output."""
    return subprocess.run(  # noqa: S603
        argv, capture_output=True, text=True, check=False, cwd=cwd, env=env, timeout=timeout
    )


def generate_once(spec: Path, config: Path, out: Path, sdk_dir: Path) -> dict[str, object]:
    """One ``ferrule generate`` run; the error is returned, not raised."""
    shutil.rmtree(out, ignore_errors=True)
    request = GenerateRequest(spec, config, out, resolve_tools(os.environ), runtime_from=sdk_dir)
    try:
        result = generate(request)
    except FerruleError as exc:
        out.mkdir(parents=True, exist_ok=True)
        return {"ok": False, "error": exc.diagnostic.render(), "digest": tree_digest(out)}
    return {
        "ok": True,
        "files": result.files,
        "runtime_files": result.runtime_files,
        "digest": result.digest,
    }


def wait_for_port(port: int, proc: subprocess.Popen[bytes], timeout: float) -> bool:
    """Poll until something listens on ``port`` or ``proc`` exits."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            return False
        with socket.socket() as s:
            s.settimeout(0.5)
            if s.connect_ex((MOCK_HOST, port)) == 0:
                return True
        time.sleep(0.5)
    return False


def port_in_use(port: int) -> bool:
    """Whether something already listens on ``port`` (then it is not our mock)."""
    with socket.socket() as s:
        s.settimeout(0.5)
        return s.connect_ex((MOCK_HOST, port)) == 0


def stop_mock(proc: subprocess.Popen[bytes]) -> None:
    """Stop the mock's process group; a mock that already exited needs nothing."""
    if proc.poll() is None:
        with contextlib.suppress(ProcessLookupError, PermissionError):
            os.killpg(proc.pid, signal.SIGTERM)
    proc.wait(timeout=30)


def refusal(sdk_dir: Path) -> dict[str, object] | None:
    """Why step 4 must not start: oracle symlinks, or a port the mock can't own."""
    linked = symlinks(sdk_dir / "tests")
    if linked:
        return {"status": "refused", "error": f"FER4007 symlinks in oracle tests/: {linked}"}
    if port_in_use(MOCK_PORT):
        return {
            "status": "mock-port-busy",
            "error": f"FER4021 something already listens on {MOCK_HOST}:{MOCK_PORT}; "
            "the oracle's tests would run against it, not the mock. Stop it and rerun.",
        }
    return None


def run_tests(
    out: Path, sdk_dir: Path, spec: Path, work: Path, package_dist: str
) -> dict[str, object]:
    """Run step 4 (venv, mock, pytest); return counts or the failing step."""
    refused = refusal(sdk_dir)
    if refused is not None:
        return refused
    shutil.rmtree(out / "tests", ignore_errors=True)
    shutil.copytree(sdk_dir / "tests", out / "tests", symlinks=True)
    venv = work / "venv"
    shutil.rmtree(venv, ignore_errors=True)
    py = str(venv / "bin" / "python")
    if run(["uv", "venv", "--quiet", "--python", PYTHON, str(venv)]).returncode:
        return {"status": "venv-failed"}
    lock = sdk_dir / "requirements-dev.lock"
    reqs = lock_requirements(lock.read_text(), exclude=[package_dist]) if lock.is_file() else []
    req_file = work / "requirements-dev.txt"
    req_file.write_text("\n".join(reqs or ["pytest"]) + "\n")
    # The lock is the full resolved set (rye locks can be internally inconsistent), so install
    # exactly those pins without re-resolving.
    no_deps = ["--no-deps"] if reqs else []
    proc = run(["uv", "pip", "install", "--quiet", *no_deps, "--python", py, "-r", str(req_file)])
    if proc.returncode:
        return {"status": "deps-failed", "error": proc.stderr.strip()[-2000:]}
    env = {
        **os.environ,
        "TEST_API_BASE_URL": f"http://{MOCK_HOST}:{MOCK_PORT}",
        "DEFER_PYDANTIC_BUILD": "false",
    }
    installed = "pyproject"
    if (out / "pyproject.toml").is_file():
        proc = run(["uv", "pip", "install", "--quiet", "--no-deps", "--python", py, str(out)])
        if proc.returncode:
            installed = f"failed: {proc.stderr.strip()[-500:]}"
    else:
        installed = "PYTHONPATH (no generated pyproject.toml)"
    if installed != "pyproject":
        env["PYTHONPATH"] = str(out / "src")
    mock_script = sdk_dir / "scripts" / "mock"
    cmd = mock_command(mock_script.read_text() if mock_script.is_file() else "", spec)
    mock_log = (work / "mock.log").open("wb")
    mock = subprocess.Popen(cmd, stdout=mock_log, stderr=subprocess.STDOUT, start_new_session=True)  # noqa: S603
    try:
        if not wait_for_port(MOCK_PORT, mock, MOCK_START_TIMEOUT_S):
            return {"status": "mock-failed", "install": installed, "mock": " ".join(cmd)}
        junit = work / "junit.xml"
        junit.unlink(missing_ok=True)
        log("running the oracle's tests against the generated package")
        try:
            proc = run(
                [
                    py,
                    "-m",
                    "pytest",
                    "tests",
                    "-q",
                    "-p",
                    "no:cacheprovider",
                    f"--junitxml={junit}",
                ],
                cwd=out,
                env=env,
                timeout=TEST_TIMEOUT_S,
            )
        except subprocess.TimeoutExpired:
            return {"status": "timeout", "install": installed}
        (work / "pytest.log").write_text(proc.stdout + proc.stderr)
        counts = parse_junit(junit.read_text()) if junit.is_file() else Outcomes()
        status = "passed" if proc.returncode == 0 else f"pytest exit {proc.returncode}"
        return {"status": status, "install": installed, "mock": " ".join(cmd), **counts.as_dict()}
    finally:
        stop_mock(mock)
        mock_log.close()


def main(argv: list[str] | None = None) -> int:
    """Run the harness for one oracle; exit 0 only on 0 breaking and passing tests."""
    parser = argparse.ArgumentParser(
        prog="parity", description=__doc__.splitlines()[0] if __doc__ else None
    )
    parser.add_argument("oracle")
    parser.add_argument("--skip-tests", action="store_true", help="stop after verify")
    args = parser.parse_args(argv)

    log(f"fetching {args.oracle}")
    fetched = run([sys.executable, str(ROOT / "oracles" / "fetch.py"), args.oracle])
    if fetched.returncode:
        sys.stderr.write(fetched.stderr)
        return 1
    pair = resolve_pair(
        parse_manifest((ROOT / "oracles" / "manifest.yaml").read_text()), args.oracle
    )
    cfg_dir, sdk_dir = CACHE / pair.config.name, CACHE / pair.sdk.name
    spec = spec_file(pair.config, cfg_dir)
    config = cfg_dir / (pair.config.config_path or "stainless.yml")
    work = WORK / pair.config.group
    work.mkdir(parents=True, exist_ok=True)
    out = work / "sdk"

    report: dict[str, object] = {
        "oracle": pair.config.group,
        "config": {"name": pair.config.name, "sha": pair.config.sha},
        "sdk": {"name": pair.sdk.name, "sha": pair.sdk.sha},
    }
    log("generating (twice, for determinism)")
    first = generate_once(spec, config, out, sdk_dir)
    second = generate_once(spec, config, work / "sdk-again", sdk_dir)
    report["generate"] = {**first, "deterministic": first["digest"] == second["digest"]}

    log("verifying the public API")
    try:
        verified = verify(sdk_dir, out)
    except FerruleError as exc:
        report["api"] = {"error": exc.diagnostic.render(), "breaking": None, "additive": None}
        report["closeness"] = None
    else:
        (work / "verify.json").write_text(json.dumps(verified.to_json(), indent=2) + "\n")
        report["api"] = {
            "breaking": len(verified.breaking),
            "additive": len(verified.additive),
            "breaking_kinds": breakage_kinds([c.kind for c in verified.breaking]),
            "additive_kinds": breakage_kinds([c.kind for c in verified.additive]),
        }
        report["closeness"] = round(verified.closeness, 2)

    if args.skip_tests:
        report["tests"] = {"status": "skipped"}
    else:
        dist = pair.sdk.name.removesuffix("-python")
        report["tests"] = run_tests(out, sdk_dir, spec, work, dist)

    REPORTS.mkdir(parents=True, exist_ok=True)
    dest = REPORTS / f"{pair.config.group}.json"
    dest.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    api = report["api"]
    tests = report["tests"]
    assert isinstance(api, dict)  # noqa: S101
    assert isinstance(tests, dict)  # noqa: S101
    sys.stdout.write(
        f"{pair.config.group}: breaking {api['breaking']} · additive {api['additive']} · "
        f"closeness {report['closeness']}% · tests {tests.get('passed', 0)} passed, "
        f"{tests.get('failed', 0)} failed, {tests.get('errors', 0)} errors ({tests['status']})\n"
        f"report: {dest.relative_to(ROOT)}\n",
    )
    ok = api["breaking"] == 0 and tests["status"] in {"passed", "skipped"}
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
