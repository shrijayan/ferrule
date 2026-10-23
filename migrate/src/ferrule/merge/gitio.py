# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""File and git access for ``ferrule merge``: read trees, write trees, regenerate branches.

Only local git plumbing is used, with hooks disabled; nothing is ever pushed.
"""

from __future__ import annotations

import os
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

from ferrule.errors import BAD_REF, OUT_NOT_EMPTY, STEP_FAILED, SYMLINK_REFUSED, fail
from ferrule.merge.core import MergeResult, merge_trees

if TYPE_CHECKING:
    from collections.abc import Mapping

    from ferrule.merge.core import Tree
    from ferrule.merge.rules import KeepRules

GENERATED_BRANCH = "generated"
TRAILER = "Ferrule-Generated-From"
_FALLBACK_IDENTITY = {
    "GIT_AUTHOR_NAME": "Ferrule",
    "GIT_AUTHOR_EMAIL": "ferrule@localhost",
    "GIT_COMMITTER_NAME": "Ferrule",
    "GIT_COMMITTER_EMAIL": "ferrule@localhost",
}


def _git(repo: Path, *args: str, stdin: bytes | None = None, env: Mapping[str, str] = {}) -> bytes:
    cmd = ["git", "-C", str(repo), "-c", f"core.hooksPath={os.devnull}", *args]
    proc = subprocess.run(  # noqa: S603 - fixed git argv, no shell
        cmd, input=stdin, capture_output=True, check=False, env={**os.environ, **env}
    )
    if proc.returncode != 0:
        detail = proc.stderr.decode(errors="replace").strip().splitlines()
        raise fail(
            STEP_FAILED,
            f"git {args[0]} failed: {detail[-1] if detail else proc.returncode}",
            str(repo),
            "check that the repository and refs exist",
        )
    return proc.stdout


def _rev(repo: Path, ref: str) -> str | None:
    proc = subprocess.run(  # noqa: S603 - fixed git argv, no shell
        ["git", "-C", str(repo), "rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}"],  # noqa: S607
        capture_output=True,
        check=False,
    )
    return proc.stdout.decode().strip() if proc.returncode == 0 else None


def read_ref(repo: Path, ref: str) -> dict[str, bytes]:
    """Read every regular file of the commit ``ref`` into memory."""
    if _rev(repo, ref) is None:
        raise fail(BAD_REF, f"{ref!r} is not a commit", str(repo), "pass a branch, tag or sha")
    listing = _git(repo, "ls-tree", "-r", "-z", "--full-tree", ref)
    entries: list[tuple[str, str]] = []
    for entry in filter(None, listing.split(b"\0")):
        meta, path = entry.split(b"\t", 1)
        mode, kind, sha = meta.split()
        if kind != b"blob":
            continue  # submodules are not part of an SDK tree
        if mode == b"120000":
            raise fail(SYMLINK_REFUSED, "symlink in tree", path.decode(), "replace with a file")
        entries.append((path.decode(), sha.decode()))
    batch = _git(repo, "cat-file", "--batch", stdin="".join(f"{s}\n" for _, s in entries).encode())
    tree: dict[str, bytes] = {}
    at = 0
    for name, _ in entries:  # each record: "<sha> blob <size>\n<data>\n"
        header_end = batch.index(b"\n", at)
        size = int(batch[at:header_end].split()[2])
        tree[name] = batch[header_end + 1 : header_end + 1 + size]
        at = header_end + 2 + size
    return tree


def read_dir(root: Path) -> dict[str, bytes]:
    """Read every file under ``root`` (skipping ``.git``) into memory."""
    tree: dict[str, bytes] = {}
    for path in sorted(root.rglob("*")):
        rel = path.relative_to(root)
        if rel.parts[0] == ".git":
            continue
        if path.is_symlink():
            raise fail(SYMLINK_REFUSED, "symlink in tree", str(path), "replace with a file")
        if path.is_file():
            tree[rel.as_posix()] = path.read_bytes()
    return tree


def read_tree(source: str, repo: Path) -> dict[str, bytes]:
    """Read a directory if ``source`` is one, else the git ref ``source`` in ``repo``."""
    path = Path(source)
    return read_dir(path) if path.is_dir() else read_ref(repo, source)


def write_tree(out: Path, files: Tree) -> None:
    """Write ``files`` into ``out``, which must be new or empty."""
    if out.exists() and any(out.iterdir()):
        raise fail(OUT_NOT_EMPTY, "output directory is not empty", str(out), "pass a new --out")
    for rel, data in files.items():
        target = out / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)


def _identity_env(repo: Path) -> dict[str, str]:
    proc = subprocess.run(  # noqa: S603 - fixed git argv, no shell
        ["git", "-C", str(repo), "var", "GIT_COMMITTER_IDENT"],  # noqa: S607
        capture_output=True,
        check=False,
    )
    return {} if proc.returncode == 0 else dict(_FALLBACK_IDENTITY)


def commit_tree(repo: Path, files: Tree, parent: str | None, message: str) -> str:
    """Create a commit holding exactly ``files`` (the work tree and index are not touched)."""
    paths = sorted(files)
    with tempfile.TemporaryDirectory(prefix="ferrule-merge-") as tmp:
        work = Path(tmp) / "work"
        write_tree(work, files)
        listing = "".join(f"{work / p}\n" for p in paths).encode()
        shas = _git(repo, "hash-object", "-w", "--stdin-paths", stdin=listing).split()
        info = b"".join(
            b"100644 %s\t%s\0" % (sha, p.encode()) for sha, p in zip(shas, paths, strict=True)
        )
        env = {"GIT_INDEX_FILE": str(Path(tmp) / "index")}
        _git(repo, "update-index", "-z", "--index-info", stdin=info, env=env)
        tree = _git(repo, "write-tree", env=env).decode().strip()
    parents = ["-p", parent] if parent else []
    sha = _git(repo, "commit-tree", tree, *parents, "-m", message, env=_identity_env(repo))
    return sha.decode().strip()


@dataclass(frozen=True)
class Regeneration:
    """What :func:`regenerate_branch` did; ``conflict_branch`` is set when a PR is needed."""

    generated: str
    result: MergeResult
    branch: str
    commit: str | None
    conflict_branch: str | None


def _current_branch(repo: Path) -> str | None:
    proc = subprocess.run(  # noqa: S603 - fixed git argv, no shell
        ["git", "-C", str(repo), "symbolic-ref", "--quiet", "--short", "HEAD"],  # noqa: S607
        capture_output=True,
        check=False,
    )
    return proc.stdout.decode().strip() if proc.returncode == 0 else None


def regenerate_branch(  # noqa: PLR0913 - keyword-only knobs with defaults
    repo: Path,
    new_tree: Tree,
    *,
    keep: KeepRules,
    into: str | None = None,
    generated: str = GENERATED_BRANCH,
    message: str = "chore: regenerate SDK",
) -> Regeneration:
    """Commit ``new_tree`` onto ``generated`` and merge it into ``into`` (default: HEAD).

    A clean merge lands as one commit on ``into`` (fast-forwarding the work tree when it is
    checked out); a blocking conflict lands, with markers, on ``ferrule/conflict-<sha>``
    instead and ``into`` is left alone, ready for a conflict PR.
    """
    branch = into or _current_branch(repo)
    if branch is None:
        raise fail(BAD_REF, "HEAD is detached", str(repo), "check out a branch or pass into=")
    ours_sha = _rev(repo, branch)
    if ours_sha is None:
        raise fail(BAD_REF, f"{branch!r} is not a branch", str(repo), "pass an existing branch")
    previous = _rev(repo, generated)
    ours = read_ref(repo, ours_sha)
    # First regeneration (adopt): the integrated branch is the baseline, nothing is forced.
    base = read_ref(repo, previous) if previous else dict(new_tree)
    gen_sha = commit_tree(repo, new_tree, previous, message)
    _git(repo, "update-ref", f"refs/heads/{generated}", gen_sha, previous or "")
    result = merge_trees(base, ours, new_tree, keep=keep)
    if result.files == ours:
        return Regeneration(gen_sha, result, branch, None, None)
    body = f"{message}\n\n{TRAILER}: {gen_sha}\n"
    sha = commit_tree(repo, result.files, ours_sha, body)
    if result.blocking:
        conflict = f"ferrule/conflict-{gen_sha[:12]}"
        _git(repo, "update-ref", f"refs/heads/{conflict}", sha)
        return Regeneration(gen_sha, result, branch, sha, conflict)
    if _current_branch(repo) == branch:
        _git(repo, "merge", "--ff-only", "--quiet", sha)
    else:
        _git(repo, "update-ref", f"refs/heads/{branch}", sha, ours_sha)
    return Regeneration(gen_sha, result, branch, sha, None)
