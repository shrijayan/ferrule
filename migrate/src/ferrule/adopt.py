# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule adopt``: switch an existing Stainless-generated SDK repo to Ferrule, locally.

Steps: rescue the spec, find the last Stainless commit, regenerate in adopt mode (ADR 0004,
ADR 0011), record every hand edit made since that commit to a generated file as a
``.ferrule/patches/NNNN-<slug>.patch``, re-apply them three-way on the regenerated tree, write
``ferrule.lock`` and ``.github/workflows/sdk.yml``, and commit all of it on a new local branch
``ferrule/adopt``. Nothing is pushed and no PR is opened.

Every git call runs with hooks, fsmonitor, submodule recursion, LFS smudging and remote
protocols disabled, inside the given repository only; the work happens in a temporary
``git worktree`` so the user's checkout is never touched.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING

from ferrule import __version__
from ferrule.errors import MISSING_INPUT, SYMLINK_REFUSED, fail
from ferrule.generate import (
    GenerateRequest,
    Runner,
    RuntimeRenderer,
    Tools,
    ensure_inside,
    generate,
    relative_files,
    run_subprocess,
)
from ferrule.lock import LOCK_FILE, Lock, sha256_bytes, sha256_file
from ferrule.rescue import Fetcher, https_fetch, rescue

if TYPE_CHECKING:
    from collections.abc import Callable, Iterable, Mapping, Sequence

PATCH_CONFLICT = "FER4102"
GIT_FAILED = "FER4105"
NOT_ADOPTABLE = "FER4106"

BOT = "stainless-app[bot]"
STATS_FILE = ".stats.yml"
BRANCH = "ferrule/adopt"
COMMIT_MESSAGE = "Switch SDK generation to Ferrule"
PATCH_DIR = PurePosixPath(".ferrule/patches")
WORKFLOW_FILE = PurePosixPath(".github/workflows/sdk.yml")
PROTECTED_DIRS = frozenset({"lib", ".git", ".ferrule"})

# Pinned by full commit SHA, as every workflow in this repository is.
CHECKOUT_SHA = "3d3c42e5aac5ba805825da76410c181273ba90b1"  # actions/checkout v7.0.1
# ferrule-dev/action has no release yet: replace with its first release commit (see report).
ACTION_SHA = "0000000000000000000000000000000000000000"

GIT_CONFIG = (
    "-c",
    "core.hooksPath=/dev/null",
    "-c",
    "core.fsmonitor=false",
    "-c",
    "submodule.recurse=false",
    "-c",
    "protocol.allow=never",
    "-c",
    "filter.lfs.smudge=",
    "-c",
    "filter.lfs.process=",
    "-c",
    "filter.lfs.required=false",
    "-c",
    "commit.gpgsign=false",
)
GIT_ENV = {
    "GIT_LFS_SKIP_SMUDGE": "1",
    "GIT_TERMINAL_PROMPT": "0",
    "GIT_CONFIG_NOSYSTEM": "1",
}


def workflow_text(spec: str, config: str) -> str:
    """``.github/workflows/sdk.yml`` from plan §4, actions pinned by commit SHA."""
    return f"""name: sdk

on:
  push:
    paths: [{spec}, {config}]
  schedule:
    - cron: "0 6 * * *"

permissions:
  contents: read

jobs:
  regenerate:
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
    steps:
      - uses: actions/checkout@{CHECKOUT_SHA} # v7.0.1
        with:
          persist-credentials: false
      - uses: ferrule-dev/action@{ACTION_SHA}
        with:
          spec: {spec}
          config: {config}
          verify: true # fails on an unlabeled breaking change
          open-pr: true # labels major / minor / patch
"""


def slug(path: PurePosixPath) -> str:
    """Return a file-name-safe slug for ``path``."""
    text = re.sub(r"[^a-z0-9]+", "-", str(path).lower()).strip("-")
    return text[:60].rstrip("-") or "file"


def patch_name(index: int, path: PurePosixPath) -> str:
    """``NNNN-<slug>.patch``, numbered from 1."""
    return f"{index:04d}-{slug(path)}.patch"


_KEEP_KEY = re.compile(r"^(?P<indent>\s*)keep_files\s*:\s*(?P<inline>.*)$")
_ITEM = re.compile(r"^(?P<indent>\s*)-\s*(?P<value>.+?)\s*$")


def keep_files(config_text: str) -> frozenset[str]:
    """Paths listed under any ``keep_files:`` key of ``stainless.yml`` (block or flow list)."""
    found: set[str] = set()
    lines = config_text.splitlines()
    for i, line in enumerate(lines):
        match = _KEEP_KEY.match(line)
        if match is None:
            continue
        inline = match["inline"].split(" #", 1)[0].strip()
        if inline.startswith("[") and inline.endswith("]"):
            found.update(v.strip().strip("'\"") for v in inline[1:-1].split(",") if v.strip())
            continue
        for item_line in lines[i + 1 :]:
            item = _ITEM.match(item_line)
            if item is None or len(item["indent"]) < len(match["indent"]):
                break
            found.add(item["value"].split(" #", 1)[0].strip().strip("'\""))
    return frozenset(found)


def is_protected(path: PurePosixPath, keep: frozenset[str]) -> bool:
    """Whether adopt must never write ``path``: ``lib/`` code, git/ferrule state, keep_files."""
    return bool(PROTECTED_DIRS.intersection(path.parts)) or str(path) in keep


@dataclass(frozen=True)
class AdoptRequest:
    """Inputs of one ``ferrule adopt`` run."""

    repo: Path
    tools: Tools
    config: Path | None = None
    spec: Path | None = None
    dry_run: bool = False


@dataclass
class AdoptResult:
    """What a run did."""

    repo: Path
    base: str
    against: str
    package: str
    generated: list[PurePosixPath] = field(default_factory=list[PurePosixPath])
    patches: list[str] = field(default_factory=list[str])
    branch: str | None = None


@dataclass(frozen=True)
class Git:
    """git, confined to one repository, with every side channel disabled."""

    repo: Path
    env: Mapping[str, str] = field(default_factory=lambda: dict(os.environ))

    def run(
        self, *args: str, cwd: Path | None = None, check: bool = True
    ) -> subprocess.CompletedProcess[str]:
        """Run ``git <args>`` in ``cwd`` (default: the repository)."""
        argv = ["git", "-C", str(cwd or self.repo), *GIT_CONFIG, *args]
        proc = subprocess.run(  # noqa: S603 - fixed argv, no shell
            argv,
            capture_output=True,
            text=True,
            check=False,
            env={**self.env, **GIT_ENV},
        )
        if check and proc.returncode != 0:
            last = (proc.stderr or proc.stdout).strip().splitlines()
            raise fail(
                GIT_FAILED,
                f"git {args[0]} failed: {last[-1] if last else proc.returncode}",
                str(self.repo),
                "fix the repository state git reported",
            )
        return proc

    def out(self, *args: str) -> str:
        """Return the stripped stdout of ``git <args>``."""
        return self.run(*args).stdout.strip()


def last_stainless_commit(git: Git) -> str:
    """Newest commit authored by the Stainless bot, else the last one to change .stats.yml."""
    sha = git.out("log", "-n1", "--fixed-strings", f"--author={BOT}", "--format=%H", "HEAD")
    if not sha:
        sha = git.out("log", "-n1", "--format=%H", "HEAD", "--", STATS_FILE)
    if not sha:
        raise fail(
            NOT_ADOPTABLE,
            f"no commit by {BOT} and no commit touching {STATS_FILE}",
            "REPO",
            "adopt a repository Stainless generated",
        )
    return sha


def last_release(git: Git, base: str) -> str:
    """Return the newest tag reachable from HEAD, else ``base``."""
    proc = git.run("describe", "--tags", "--abbrev=0", "HEAD", check=False)
    tag = proc.stdout.strip()
    return tag if proc.returncode == 0 and tag else base


def _check_repo(git: Git) -> None:
    top = git.run("rev-parse", "--show-toplevel", check=False)
    if top.returncode != 0 or Path(top.stdout.strip()).resolve() != git.repo.resolve():
        raise fail(MISSING_INPUT, f"{git.repo} is not a git repository root", "REPO", "pass one")
    if git.out("status", "--porcelain", "--untracked-files=no"):
        raise fail(
            NOT_ADOPTABLE,
            "the working tree has uncommitted changes",
            "REPO",
            "commit or stash them first",
        )
    links = [
        line.split("\t", 1)[1]
        for line in git.out("ls-tree", "-r", "HEAD").splitlines()
        if line.startswith("120000 ")
    ]
    if links:
        raise fail(
            SYMLINK_REFUSED,
            f"refusing tracked symlinks: {', '.join(links)}",
            "REPO",
            "replace the symlinks with regular files",
        )
    if git.run("rev-parse", "--verify", "--quiet", f"refs/heads/{BRANCH}", check=False).stdout:
        raise fail(
            NOT_ADOPTABLE,
            f"branch {BRANCH} already exists",
            "REPO",
            f"delete it (git branch -D {BRANCH}) to adopt again",
        )


def _recording(runner: Runner, seen: list[PurePosixPath]) -> Runner:
    """Wrap ``runner`` to note the files Forge wrote (the generator's paths)."""

    def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        proc = runner(argv)
        if "--target" in argv and proc.returncode == 0:
            seen[:] = relative_files(Path(argv[argv.index("--out") + 1]))
        return proc

    return run


def hand_edits(git: Git, base: str, paths: Iterable[PurePosixPath]) -> list[tuple[str, str]]:
    """``(patch name, git diff)`` for each path changed between ``base`` and HEAD."""
    changed = set(
        git.out("diff", "--name-only", "--no-renames", base, "HEAD", "--").splitlines(),
    )
    patches: list[tuple[str, str]] = []
    for path in sorted(p for p in paths if str(p) in changed):
        diff = git.run(
            "diff", "--full-index", "--binary", "--no-renames", base, "HEAD", "--", str(path)
        ).stdout
        patches.append((patch_name(len(patches) + 1, path), diff))
    return patches


def forge_commit(tools: Tools, env: Mapping[str, str]) -> str:
    """Return ``$FERRULE_FORGE_COMMIT``, else the commit of the checkout Forge was built in."""
    if env.get("FERRULE_FORGE_COMMIT"):
        return env["FERRULE_FORGE_COMMIT"]
    checkout = tools.forge.resolve().parents[2]
    if (checkout / ".git").exists():
        proc = Git(checkout, env).run("rev-parse", "HEAD", check=False)
        if proc.returncode == 0:
            return proc.stdout.strip()
    return "unknown"


def _write(root: Path, rel: PurePosixPath, data: bytes) -> None:
    target = ensure_inside(root, rel)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)


def _apply(git: Git, wt: Path, patches: Sequence[tuple[str, str]], tmp: Path) -> None:
    conflicts: list[str] = []
    for name, diff in patches:
        patch = tmp / name
        patch.write_text(diff, encoding="utf-8")
        proc = git.run("apply", "--3way", "--index", str(patch), cwd=wt, check=False)
        if proc.returncode != 0:
            conflicts.append(name)
    if conflicts:
        unmerged = git.run("diff", "--name-only", "--diff-filter=U", cwd=wt).stdout.split()
        where = ", ".join(unmerged) or "see the patches"
        raise fail(
            PATCH_CONFLICT,
            f"hand edits conflict with the regenerated code: {', '.join(conflicts)} ({where})",
            str(PATCH_DIR),
            "resolve the edit by hand on the regenerated file, or move it to lib/",
        )


@dataclass(frozen=True)
class _Inputs:
    spec_name: str
    spec_data: bytes
    config_rel: str
    config_text: str


def _inputs(request: AdoptRequest, fetch: Fetcher, say: Callable[[str], None]) -> _Inputs:
    repo = request.repo
    config = request.config or ensure_inside(repo, PurePosixPath("stainless.yml"))
    if not config.is_file():
        raise fail(
            MISSING_INPUT, f"{config} does not exist", "--config", "pass --config stainless.yml"
        )
    config_rel = (
        config.resolve().relative_to(repo.resolve()).as_posix()
        if config.resolve().is_relative_to(repo.resolve())
        else "stainless.yml"
    )
    if request.spec is not None:
        if not request.spec.is_file():
            raise fail(MISSING_INPUT, f"{request.spec} does not exist", "--spec", "pass a file")
        name = f"openapi{request.spec.suffix or '.yml'}"
        data = request.spec.read_bytes()
    else:
        rescued = rescue(repo, fetch)
        say(f"✓ read {STATS_FILE} → spec URL + hash")
        name = f"openapi{rescued.stats.suffix}"
        data = rescued.data
        say(f"✓ rescued spec to ./{name} (hash ok)")
    return _Inputs(name, data, config_rel, config.read_text(encoding="utf-8"))


def adopt(  # noqa: PLR0913 - every collaborator is injectable for tests
    request: AdoptRequest,
    *,
    runner: Runner = run_subprocess,
    renderer: RuntimeRenderer | None = None,
    fetch: Fetcher = https_fetch,
    env: Mapping[str, str] | None = None,
    say: Callable[[str], None] = lambda _line: None,
) -> AdoptResult:
    """Run the adoption; returns what was done. Raises FerruleError on any problem."""
    env = dict(os.environ) if env is None else env
    repo = request.repo.resolve()
    git = Git(repo, env)
    _check_repo(git)
    inputs = _inputs(AdoptRequest(repo, request.tools, request.config, request.spec), fetch, say)
    keep = keep_files(inputs.config_text)
    base = last_stainless_commit(git)
    with tempfile.TemporaryDirectory(prefix="ferrule-adopt-") as tmp_name:
        tmp = Path(tmp_name)
        spec_file = tmp / inputs.spec_name
        spec_file.write_bytes(inputs.spec_data)
        config_file = tmp / "stainless.yml"
        config_file.write_text(inputs.config_text, encoding="utf-8")
        say("✓ converted stainless.yml → overlays")
        seen: list[PurePosixPath] = []
        out = tmp / "out"
        result = generate(
            GenerateRequest(spec_file, config_file, out, request.tools, runtime_from=repo),
            runner=_recording(runner, seen),
            renderer=renderer,
        )
        say("✓ copied runtime from your repo (you own it)")
        generated = [p for p in seen if not is_protected(p, keep)]
        patches = hand_edits(git, base, generated)
        say(f"✓ found {len(patches)} hand edits → {PATCH_DIR}/")
        say("✓ generated via forge + ferrule-python")
        adopted = AdoptResult(
            repo=repo,
            base=base,
            against=last_release(git, base),
            package=result.package,
            generated=generated,
            patches=[name for name, _ in patches],
        )
        if request.dry_run:
            return adopted
        lock = Lock(
            ferrule_version=__version__,
            forge_commit=forge_commit(request.tools, env),
            plugin_sha256=sha256_file(request.tools.plugin),
            spec_sha256=sha256_bytes(inputs.spec_data),
            config_sha256=sha256_bytes(inputs.config_text.encode()),
            files={str(p): sha256_file(out / p) for p in generated},
        )
        _commit(git, tmp=tmp, out=out, inputs=inputs, lock=lock, patches=patches)
    adopted.branch = BRANCH
    say(f"✓ committed {COMMIT_MESSAGE!r} on local branch {BRANCH} (not pushed)")
    return adopted


def _identity(git: Git) -> tuple[str, str]:
    name = git.run("config", "user.name", check=False).stdout.strip()
    email = git.run("config", "user.email", check=False).stdout.strip()
    if not name or not email:
        raise fail(
            NOT_ADOPTABLE,
            "git user.name or user.email is not set",
            "REPO",
            "set them with git config in the repository",
        )
    return name, email


def _commit(  # noqa: PLR0913 - one call site
    git: Git,
    *,
    tmp: Path,
    out: Path,
    inputs: _Inputs,
    lock: Lock,
    patches: Sequence[tuple[str, str]],
) -> None:
    name, email = _identity(git)
    wt = tmp / "worktree"
    git.run("worktree", "add", "--quiet", "-b", BRANCH, str(wt), "HEAD")
    committed = False
    try:
        for rel in lock.files:
            _write(wt, PurePosixPath(rel), (out / rel).read_bytes())
        git.run("add", "--all", "--", ".", cwd=wt)
        _apply(git, wt, patches, tmp)
        for patch_file, diff in patches:
            _write(wt, PATCH_DIR / patch_file, diff.encode())
        _write(wt, PurePosixPath(inputs.spec_name), inputs.spec_data)
        _write(wt, PurePosixPath(inputs.config_rel), inputs.config_text.encode())
        _write(wt, PurePosixPath(LOCK_FILE), lock.render().encode())
        workflow = workflow_text(inputs.spec_name, inputs.config_rel)
        _write(wt, WORKFLOW_FILE, workflow.encode())
        git.run("add", "--all", "--", ".", cwd=wt)
        git.run(
            "-c", f"user.name={name}", "-c", f"user.email={email}",
            "commit", "--quiet", "--no-verify", "-s", "-m", COMMIT_MESSAGE,
            cwd=wt,
        )  # fmt: skip
        committed = True
    finally:
        git.run("worktree", "remove", "--force", str(wt), check=False)
        shutil.rmtree(wt, ignore_errors=True)
        git.run("worktree", "prune", check=False)
        if not committed:
            git.run("branch", "-D", BRANCH, check=False)
