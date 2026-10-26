# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule adopt``: switch an existing SDK repo (Stainless-generated, or not) to Ferrule, locally.

Steps: rescue the spec, find the last Stainless commit, regenerate in adopt mode (ADR 0004,
ADR 0011), record every hand edit a generated file still carries (made by a maintainer, before
or after that commit: Stainless carries custom code into each generation) as a
``.ferrule/patches/NNNN-<slug>.patch``, re-apply them three-way on the regenerated tree, write
``ferrule.lock`` and ``.github/workflows/sdk.yml``, and commit all of it on a new local branch
``ferrule/adopt``. The pristine generation goes on a local ``generated`` branch, named in the
adopt commit's ``Ferrule-Generated-From`` trailer: ``ferrule regenerate`` merges from it. Nothing
is pushed and no PR is opened.

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
    GenVersion,
    Runner,
    RuntimeRenderer,
    Tools,
    ensure_inside,
    gen_version,
    generate,
    is_generate_step,
    relative_files,
    run_subprocess,
)
from ferrule.hand_edit import revert
from ferrule.lock import LOCK_FILE, Lock, sha256_bytes, sha256_file
from ferrule.merge import KeepRules, merge_trees
from ferrule.merge.gitio import GENERATED_BRANCH, TRAILER, commit_tree
from ferrule.rescue import Fetcher, https_fetch, rescue

if TYPE_CHECKING:
    from collections.abc import Callable, Iterable, Mapping, Sequence

PATCH_CONFLICT = "FER4102"
GIT_FAILED = "FER4105"
NOT_ADOPTABLE = "FER4106"

BOT = "stainless-app[bot]"
# Commits by these authors are generated, never hand edits: the names Stainless's bots have
# committed under over time (seen in the histories of the oracle SDKs).
STAINLESS_AUTHORS = frozenset(
    {BOT, "stainless-sdks[bot]", "stainless-sdks-internal[bot]", "stainless-bot", "Stainless Bot"}
)
STATS_FILE = ".stats.yml"
BRANCH = "ferrule/adopt"
COMMIT_MESSAGE = "Switch SDK generation to Ferrule"
PATCH_DIR = PurePosixPath(".ferrule/patches")
WORKFLOW_FILE = PurePosixPath(".github/workflows/sdk.yml")
PROTECTED_DIRS = frozenset({"lib", ".git", ".ferrule"})

# Pinned by full commit SHA, as every workflow in this repository is.
CHECKOUT_SHA = "3d3c42e5aac5ba805825da76410c181273ba90b1"  # actions/checkout v7.0.1
# The action has no release yet: pin a commit of shrijayan/ferrule that has action/ (see report).
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
    branches: [main]
    paths: [{spec}, {config}]
  schedule:
    - cron: "0 6 * * *"
  workflow_dispatch:

permissions:
  contents: read

jobs:
  regenerate:
    runs-on: ubuntu-latest
    permissions:
      contents: write # push ferrule/regenerate and the generated branch
      pull-requests: write # open the PR and label it
    steps:
      - uses: actions/checkout@{CHECKOUT_SHA} # v7.0.1
        with:
          fetch-depth: 0 # the previous release and the generated branch
          persist-credentials: false
      - uses: shrijayan/ferrule/action@{ACTION_SHA}
        with:
          spec: {spec}
          config: {config}
          verify: true # fails on an unlabeled breaking change
          open-pr: true # labels major / minor / patch
          github-token: ${{{{ secrets.GITHUB_TOKEN }}}}
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
            if not item_line.strip() or item_line.lstrip().startswith("#"):
                continue
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

    def raw(
        self, *args: str, cwd: Path | None = None, check: bool = True
    ) -> subprocess.CompletedProcess[bytes]:
        """Run ``git <args>`` in ``cwd`` (default: the repository); output stays bytes."""
        argv = ["git", "-C", str(cwd or self.repo), *GIT_CONFIG, *args]
        proc = subprocess.run(  # noqa: S603 - fixed argv, no shell
            argv,
            capture_output=True,
            check=False,
            env={**self.env, **GIT_ENV},
        )
        if check and proc.returncode != 0:
            text = (proc.stderr or proc.stdout).decode(errors="replace")
            last = text.strip().splitlines()
            raise fail(
                GIT_FAILED,
                f"git {args[0]} failed: {last[-1] if last else proc.returncode}",
                str(self.repo),
                "fix the repository state git reported",
            )
        return proc

    def run(
        self, *args: str, cwd: Path | None = None, check: bool = True
    ) -> subprocess.CompletedProcess[str]:
        """Run ``git <args>`` in ``cwd`` (default: the repository)."""
        proc = self.raw(*args, cwd=cwd, check=check)
        return subprocess.CompletedProcess(
            proc.args,
            proc.returncode,
            proc.stdout.decode(errors="surrogateescape"),
            proc.stderr.decode(errors="replace"),
        )

    def out(self, *args: str) -> str:
        """Return the stripped stdout of ``git <args>``."""
        return self.run(*args).stdout.strip()


def last_stainless_commit(git: Git) -> str | None:
    """Newest commit authored by the Stainless bot, else the last one to change .stats.yml.

    ``None`` when there is neither: the SDK was not generated by Stainless.
    """
    sha = git.out("log", "-n1", "--fixed-strings", f"--author={BOT}", "--format=%H", "HEAD")
    if not sha:
        sha = git.out("log", "-n1", "--format=%H", "HEAD", "--", STATS_FILE)
    return sha or None


def adoption_base(git: Git, say: Callable[[str], None]) -> str:
    """Return the commit hand edits are counted from: the last Stainless commit, else HEAD.

    An SDK another generator wrote (opencode-python-sdk) has no Stainless history (and no
    .stats.yml, so its spec is given with --spec). It is adopted from HEAD, so no hand edit is
    recorded: the files Ferrule generates replace that generator's, and every other file is
    kept (ADR 0011).
    """
    sha = last_stainless_commit(git)
    if sha is not None:
        return sha
    say("✓ no Stainless history: adopting from HEAD, no hand edits to record")
    return git.out("rev-parse", "HEAD")


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
    for branch in (BRANCH, GENERATED_BRANCH):
        if git.run("rev-parse", "--verify", "--quiet", f"refs/heads/{branch}", check=False).stdout:
            raise fail(
                NOT_ADOPTABLE,
                f"branch {branch} already exists",
                "REPO",
                f"delete it (git branch -D {branch}) to adopt again",
            )


def recording(runner: Runner, tools: Tools, seen: list[PurePosixPath]) -> Runner:
    """Wrap ``runner`` to note the files ``ferrule-gen generate`` wrote (the generator's paths)."""

    def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        proc = runner(argv)
        if is_generate_step(argv, tools) and proc.returncode == 0:
            seen[:] = relative_files(Path(argv[argv.index("--out") + 1]))
        return proc

    return run


@dataclass(frozen=True)
class HandEdit:
    """One recorded edit: its patch, the file without the edit and the file at HEAD."""

    name: str
    path: PurePosixPath
    diff: bytes
    pristine: bytes
    head: bytes


_DIFF = ("--no-color", "--no-ext-diff", "--no-textconv", "--no-renames")


def edit_commits(git: Git, base: str) -> dict[str, list[str]]:
    """Path -> the commits not made by Stainless that changed it, newest first.

    When Stainless authored ``base``, the whole history counts: the bot merges custom code
    into every later generation, so an edit older than ``base`` can still be live. Otherwise
    (``base`` is only the last commit to change .stats.yml) just the commits after it count.
    Merges and root commits are skipped: they carry no edit of their own.
    """
    author = git.out("log", "-n1", "--format=%an", base)
    history = "HEAD" if author in STAINLESS_AUTHORS else f"{base}..HEAD"
    log = git.out(
        "-c", "core.quotePath=false", "log", "--no-merges", "--no-renames", "--name-only",
        "--format=%x00%H%x09%P%x09%an", history,
    )  # fmt: skip
    commits: dict[str, list[str]] = {}
    for record in log.split("\x00")[1:]:
        head, *files = record.split("\n")
        sha, parents, name = head.split("\t", 2)
        if not parents or name in STAINLESS_AUTHORS:
            continue
        for path in filter(None, files):
            commits.setdefault(path, []).append(sha)
    return commits


def hand_edits(
    git: Git, base: str, out: Path, paths: Iterable[PurePosixPath], tmp: Path
) -> list[HandEdit]:
    """Every hand edit the generated ``paths`` (written under ``out``) still carry at HEAD.

    Each edit is undone on the file as it is at HEAD (:func:`ferrule.hand_edit.revert`),
    unless the regenerated file already has its text; the patch, one per path, goes from that
    pristine file to HEAD, so it applies three-way on a regeneration.
    """
    commits = edit_commits(git, base)
    edits: list[HandEdit] = []
    for path in sorted(p for p in paths if str(p) in commits):
        shown = git.raw("show", f"HEAD:{path}", check=False)
        if shown.returncode != 0:
            continue  # the edit was to a file HEAD no longer has
        head = pristine = shown.stdout
        regenerated = ensure_inside(out, path).read_bytes()
        for sha in commits[str(path)]:
            diff = git.raw("diff", *_DIFF, f"{sha}^", sha, "--", str(path)).stdout
            pristine = revert(pristine, diff, regenerated)
        if pristine == head:
            continue
        work = tmp / "edits" / str(len(edits))
        _write(work, "a" / path, pristine)
        _write(work, "b" / path, head)
        patch = git.raw(
            "diff", "--no-index", *_DIFF, "--full-index", "--binary", "--no-prefix",
            f"a/{path}", f"b/{path}", cwd=work, check=False,
        ).stdout  # fmt: skip
        edits.append(HandEdit(patch_name(len(edits) + 1, path), path, patch, pristine, head))
    return edits


def forge_commit(version: GenVersion, env: Mapping[str, str]) -> str:
    """Return ``$FERRULE_FORGE_COMMIT``, else the Forge rev ``ferrule-gen --version`` names."""
    return env.get("FERRULE_FORGE_COMMIT") or version.forge


def _write(root: Path, rel: PurePosixPath, data: bytes) -> None:
    target = ensure_inside(root, rel)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)


def reapply(edit: HandEdit, regenerated: bytes) -> bytes | None:
    """``regenerated`` with ``edit`` merged in three-way, or None when they conflict.

    The base is the file without the edit, ours the file at HEAD and theirs the regenerated
    one; a line merge that conflicts falls back to the libcst merge ``ferrule merge`` uses,
    so a method added to a class the generator reorders still lands.
    """
    path = str(edit.path)
    result = merge_trees(
        {path: edit.pristine},
        {path: edit.head},
        {path: regenerated},
        keep=KeepRules(protect_lib=False),
    )
    return None if result.blocking else result.files[path]


def _apply(wt: Path, patches: Sequence[HandEdit]) -> None:
    conflicts: list[HandEdit] = []
    for edit in patches:
        target = ensure_inside(wt, edit.path)
        merged = reapply(edit, target.read_bytes())
        if merged is None:
            conflicts.append(edit)
        else:
            target.write_bytes(merged)
    if conflicts:
        names = ", ".join(edit.name for edit in conflicts)
        where = ", ".join(str(edit.path) for edit in conflicts)
        raise fail(
            PATCH_CONFLICT,
            f"hand edits conflict with the regenerated code: {names} ({where})",
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
    base = adoption_base(git, say)
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
            runner=recording(runner, request.tools, seen),
            renderer=renderer,
        )
        if result.rendered:
            say("✓ rendered Ferrule's runtime (your SDK's runtime is not Stainless-style)")
        else:
            say("✓ copied runtime from your repo (you own it)")
        written = sorted({*seen, *result.rendered})
        generated = [p for p in written if not is_protected(p, keep)]
        patches = hand_edits(git, base, out, generated, tmp)
        say(f"✓ found {len(patches)} hand edits → {PATCH_DIR}/")
        say("✓ generated via ferrule-gen (forge + ferrule-python)")
        adopted = AdoptResult(
            repo=repo,
            base=base,
            against=last_release(git, base),
            package=result.package,
            generated=generated,
            patches=[edit.name for edit in patches],
        )
        if request.dry_run:
            return adopted
        version = gen_version(request.tools, runner)
        lock = Lock(
            ferrule_version=__version__,
            forge_commit=forge_commit(version, env),
            generator=version.generator,
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
    patches: Sequence[HandEdit],
) -> None:
    name, email = _identity(git)
    # The pristine generation, base of the next `ferrule regenerate` (its three-way merge).
    pristine = {rel: (out / rel).read_bytes() for rel in lock.files}
    pristine[LOCK_FILE] = lock.render().encode()
    generation = commit_tree(git.repo, pristine, None, "Ferrule generation (adopt)")
    wt = tmp / "worktree"
    git.run("worktree", "add", "--quiet", "-b", BRANCH, str(wt), "HEAD")
    committed = False
    try:
        for rel in lock.files:
            _write(wt, PurePosixPath(rel), (out / rel).read_bytes())
        git.run("add", "--all", "--", ".", cwd=wt)
        _apply(wt, patches)
        for edit in patches:
            _write(wt, PATCH_DIR / edit.name, edit.diff)
        _write(wt, PurePosixPath(inputs.spec_name), inputs.spec_data)
        _write(wt, PurePosixPath(inputs.config_rel), inputs.config_text.encode())
        _write(wt, PurePosixPath(LOCK_FILE), lock.render().encode())
        workflow = workflow_text(inputs.spec_name, inputs.config_rel)
        _write(wt, WORKFLOW_FILE, workflow.encode())
        git.run("add", "--all", "--", ".", cwd=wt)
        git.run(
            "-c", f"user.name={name}", "-c", f"user.email={email}",
            "commit", "--quiet", "--no-verify", "-s", "-m", COMMIT_MESSAGE,
            "-m", f"{TRAILER}: {generation}",
            cwd=wt,
        )  # fmt: skip
        git.run("update-ref", f"refs/heads/{GENERATED_BRANCH}", generation)
        committed = True
    finally:
        git.run("worktree", "remove", "--force", str(wt), check=False)
        shutil.rmtree(wt, ignore_errors=True)
        git.run("worktree", "prune", check=False)
        if not committed:
            git.run("branch", "-D", BRANCH, check=False)
