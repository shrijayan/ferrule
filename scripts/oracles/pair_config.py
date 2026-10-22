# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Pair a public Stainless config repo with its Python SDK by md5 (ADR 0009).

Usage (from the repo root):
    python3 -m scripts.oracles.pair_config CONFIG_REPO CONFIG_PATH SDK_REPO [--spec PATH ...]

Both repos are cloned bare into oracles/cache/pairing/ with the git safety settings of
oracles/fetch.py; nothing is checked out or run. For each stainless-app[bot] commit of the SDK
(newest first) whose `.stats.yml` config_hash equals md5(CONFIG_PATH) at some config-repo
commit, it prints that commit and whether md5(spec) also equals openapi_spec_hash. With no
match it prints the config-repo commit nearest before the last Stainless SDK commit.
The method is described in oracles/README.md.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import subprocess
import sys
from dataclasses import dataclass
from typing import TYPE_CHECKING

from oracles.fetch import CACHE, GIT_SAFE, parse_stats_yml

if TYPE_CHECKING:
    from pathlib import Path

WORK = CACHE / "pairing"
BOT = "stainless-app[bot]"
MAX_MATCHES = 5


@dataclass(frozen=True, slots=True)
class Version:
    """One commit with the md5 of each file we care about (None when absent)."""

    sha: str
    date: str
    subject: str
    md5: dict[str, str | None]


def git(repo: Path | None, *args: str) -> bytes | None:
    """Run git with hooks, LFS and submodules off; return stdout, or None on failure."""
    env = {**os.environ, "GIT_TERMINAL_PROMPT": "0", "TZ": "UTC"}
    result = subprocess.run(  # noqa: S603 - fixed git argv, no shell
        ["git", *GIT_SAFE, "-c", "core.commitGraph=false", *args],  # noqa: S607 - git on PATH
        cwd=repo,
        env=env,
        capture_output=True,
        check=False,
    )
    return result.stdout if result.returncode == 0 else None


def clone(repo: str) -> Path:
    """Bare-clone (or update) github.com/<repo> under WORK."""
    dest = WORK / repo.replace("/", "__")
    if dest.is_dir():
        git(dest, "fetch", "--quiet", "origin", "+refs/heads/*:refs/heads/*")
    else:
        WORK.mkdir(parents=True, exist_ok=True)
        url = f"https://github.com/{repo}.git"
        git(None, "clone", "--quiet", "--bare", "--no-recurse-submodules", url, str(dest))
    return dest


def history(repo: Path, paths: list[str], *, author: str | None = None) -> list[Version]:
    """List commits on HEAD touching any of paths, newest first, with the md5 of each path."""
    fmt = "--format=%H|%an|%cd|%s"
    log = git(repo, "log", fmt, "--date=iso-strict-local", "HEAD", "--", *paths) or b""
    out: list[Version] = []
    for line in log.decode().splitlines():
        sha, name, date, subject = line.split("|", 3)
        if author is not None and name != author:
            continue
        md5: dict[str, str | None] = {}
        for path in paths:
            blob = git(repo, "show", f"{sha}:{path}")
            md5[path] = (
                None if blob is None else hashlib.md5(blob, usedforsecurity=False).hexdigest()
            )
        out.append(Version(sha, date.replace("+00:00", "Z"), subject, md5))
    return out


def sdk_stats(repo: Path) -> list[tuple[Version, dict[str, str]]]:
    """Stainless-authored SDK commits that changed .stats.yml, with its parsed values."""
    out: list[tuple[Version, dict[str, str]]] = []
    for version in history(repo, [".stats.yml"], author=BOT):
        text = git(repo, "show", f"{version.sha}:.stats.yml") or b""
        out.append((version, parse_stats_yml(text.decode(errors="replace"))))
    return out


def main(argv: list[str]) -> int:
    """Entry point."""
    parser = argparse.ArgumentParser(description="Pair a Stainless config repo with its SDK.")
    parser.add_argument("config_repo")
    parser.add_argument("config_path")
    parser.add_argument("sdk_repo")
    parser.add_argument("--spec", action="append", default=[], help="spec path in config repo")
    args = parser.parse_args(argv)
    config_path: str = args.config_path
    specs: list[str] = args.spec
    cfg = history(clone(args.config_repo), [config_path, *specs])
    stats = sdk_stats(clone(args.sdk_repo))
    write = sys.stdout.write
    write(f"{len(cfg)} config-repo versions, {len(stats)} stainless .stats.yml versions\n")
    found = 0
    for sdk, values in stats:
        hits = [c for c in cfg if c.md5[config_path] == values.get("config_hash")]
        if not hits:
            continue
        spec_hits = [c for c in hits if values.get("openapi_spec_hash") in c.md5.values()]
        best = (spec_hits or hits)[0]
        spec = "spec md5 matches" if spec_hits else "spec not in repo or different"
        write(f"SDK {sdk.sha} {sdk.date} {sdk.subject!r}\n")
        write(f"  config {best.sha} {best.date} {best.subject!r}: config md5 matches, {spec}\n")
        found += 1
        if found >= MAX_MATCHES:
            break
    if not found and stats:
        last = stats[0][0]
        before = [c for c in cfg if c.date <= last.date and c.md5[config_path]]
        write(f"no md5 match; last Stainless .stats.yml change {last.sha} {last.date}\n")
        if before:
            write(f"  nearest config commit before it: {before[0].sha} {before[0].date}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
