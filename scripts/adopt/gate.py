# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Phase 4 gate: adopt public Stainless Python SDKs locally and prove nothing changed.

Usage: uv run python -m scripts.adopt.gate [name ...]   (default: TARGETS)

Generation runs ``ferrule-gen`` (``$FERRULE_GEN``, default ``target/release/ferrule-gen``);
``$FERRULE_CONVERT`` replaces its convert command, as for the parity harness.

For each config oracle with a paired SDK (oracles/manifest.yaml):

1. clone the pinned SDK from oracles/cache into .cache/adopt/<name> (local only, never pushed);
2. ``ferrule adopt`` it with the pinned config and spec (adopt runs verify; exit 1 = breaking);
3. check custom code is intact: every file ferrule.lock does not list as generated is
   byte-identical between the pinned commit and the ``ferrule/adopt`` branch;
4. run the SDK's own tests against the adopted tree with the parity harness's mock + venv.

Reports go to reports/adopt/<name>.json. Oracle code is data: it is cloned, read and tested
in a venv exactly as the parity harness does, never installed with its own scripts.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from typing import TYPE_CHECKING, cast

from oracles.fetch import parse_manifest
from scripts.parity.harness import resolve_pair
from scripts.parity.run import CACHE, ROOT, run_tests, spec_file

if TYPE_CHECKING:
    from pathlib import Path

# Picked from the Phase 4 baseline: config oracles that reach 0 breaking (docs/phases/phase-4.md).
TARGETS = ("onebusaway", "isaacus", "lumaai", "honcho", "knock-mgmt")
WORK = ROOT / ".cache" / "adopt"
REPORTS = ROOT / "reports" / "adopt"
GIT_SAFE = (
    "-c",
    "core.hooksPath=/dev/null",
    "-c",
    "protocol.allow=never",
    "-c",
    "protocol.file.allow=always",
)


def git(repo: Path, *args: str) -> str:
    """Run git in ``repo`` with hooks off; return stdout."""
    proc = subprocess.run(  # noqa: S603 - fixed argv
        ["git", "-C", str(repo), *GIT_SAFE, *args],  # noqa: S607
        capture_output=True,
        text=True,
        check=True,
    )
    return proc.stdout


def tree_blobs(repo: Path, ref: str) -> dict[str, str]:
    """Return path -> blob sha of every file at ``ref``."""
    out: dict[str, str] = {}
    for line in git(repo, "ls-tree", "-r", "--full-tree", ref).splitlines():
        meta, path = line.split("\t", 1)
        out[path] = meta.split()[2]
    return out


def custom_code_changes(repo: Path, base: str, branch: str, generated: set[str]) -> list[str]:
    """Return non-generated paths that differ between ``base`` and ``branch``.

    Files adopt adds on purpose (spec, lock, workflow, patches) are allowed new; anything else
    that existed at ``base`` and is not generated must be byte-identical.
    """
    before, after = tree_blobs(repo, base), tree_blobs(repo, branch)
    return sorted(
        path for path, blob in before.items() if path not in generated and after.get(path) != blob
    )


def prepare(name: str, url: str, sha: str) -> Path:
    """Clone the pinned SDK in full (tags and blobs, for verify) with a git identity.

    The oracle cache is a blob-less clone, so release tags there can't be archived.
    """
    dest = WORK / name
    if dest.exists():
        shutil.rmtree(dest)
    dest.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(  # noqa: S603 - local clone of our own cache
        [  # noqa: S607
            "git",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "submodule.recurse=false",
            "clone",
            "--quiet",
            "--no-recurse-submodules",
            url,
            str(dest),
        ],
        check=True,
    )
    git(dest, "checkout", "--quiet", "-B", "main", sha)
    git(dest, "config", "user.name", "Ferrule adopt gate")
    git(dest, "config", "user.email", "adopt-gate@ferrule.invalid")
    return dest


def adopt_one(name: str) -> dict[str, object]:
    """Adopt one target and return its report."""
    subprocess.run([sys.executable, str(ROOT / "oracles" / "fetch.py"), name], check=True)  # noqa: S603
    pair = resolve_pair(parse_manifest((ROOT / "oracles" / "manifest.yaml").read_text()), name)
    cfg_dir, sdk_cache = CACHE / pair.config.name, CACHE / pair.sdk.name
    spec = spec_file(pair.config, cfg_dir)
    config = cfg_dir / (pair.config.config_path or "stainless.yml")
    if not pair.sdk.url.startswith("https://"):
        msg = f"{pair.sdk.name}: only https clone URLs are allowed"
        raise ValueError(msg)
    repo = prepare(pair.config.group, pair.sdk.url, pair.sdk.sha)

    # FERRULE_CONVERT, if set, passes through (the TS converter until the Rust port lands).
    env = {
        **os.environ,
        "FERRULE_GEN": os.environ.get("FERRULE_GEN") or str(ROOT / "target/release/ferrule-gen"),
    }
    proc = subprocess.run(  # noqa: S603 - our own CLI
        ["uv", "run", "ferrule", "adopt", str(repo), "--config", str(config), "--spec", str(spec)],  # noqa: S607
        capture_output=True,
        text=True,
        check=False,
        env=env,
        cwd=ROOT,
    )
    report: dict[str, object] = {
        "target": pair.config.group,
        "sdk": {"name": pair.sdk.name, "sha": pair.sdk.sha},
        "adopt": {"exit": proc.returncode, "output": proc.stdout[-4000:] + proc.stderr[-4000:]},
    }
    branches = git(repo, "branch", "--list", "ferrule/adopt").strip()
    if not branches:
        report["status"] = "adopt failed"
        return report

    lock = json.loads(git(repo, "show", "ferrule/adopt:ferrule.lock"))
    generated = set(lock.get("files", {}))
    changed = custom_code_changes(repo, pair.sdk.sha, "ferrule/adopt", generated)
    report["custom_code_changed"] = changed

    tree = WORK / f"{pair.config.group}-tree"
    if tree.exists():
        shutil.rmtree(tree)
    tree.mkdir(parents=True)
    archive = subprocess.run(  # noqa: S603
        ["git", "-C", str(repo), *GIT_SAFE, "archive", "ferrule/adopt"],  # noqa: S607
        capture_output=True,
        check=True,
    )
    subprocess.run(["tar", "-x", "-C", str(tree)], input=archive.stdout, check=True)  # noqa: S603, S607
    dist = pair.sdk.name.removesuffix("-python")
    tests = run_tests(tree, sdk_cache, spec, WORK / f"{pair.config.group}-work", dist)
    report["tests"] = tests

    breaking_ok = proc.returncode == 0
    report["status"] = (
        "passed" if breaking_ok and not changed and tests.get("status") == "passed" else "failed"
    )
    return report


def main(argv: list[str] | None = None) -> int:
    """Adopt every target; exit 0 only if all pass."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0] if __doc__ else None)
    parser.add_argument("names", nargs="*", default=list(TARGETS))
    args = parser.parse_args(argv)
    REPORTS.mkdir(parents=True, exist_ok=True)
    failed: list[str] = []
    for name in args.names:
        report = adopt_one(name)
        (REPORTS / f"{report['target']}.json").write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n"
        )
        tests = report.get("tests")
        summary = cast("dict[str, object]", tests) if isinstance(tests, dict) else {}
        sys.stdout.write(
            f"{report['target']}: {report['status']} · adopt exit {report['adopt']['exit']}"  # type: ignore[index]
            f" · custom code changed {len(report.get('custom_code_changed', []))}"  # type: ignore[arg-type]
            f" · tests {summary.get('passed', 0)} passed, {summary.get('failed', 0)} failed\n"
        )
        if report["status"] != "passed":
            failed.append(str(report["target"]))
    sys.stdout.write(f"adopt:gate {'failed: ' + ', '.join(failed) if failed else 'OK'}\n")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
