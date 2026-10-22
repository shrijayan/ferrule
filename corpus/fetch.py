"""Download the pinned spec corpus into corpus/cache/ and verify each file's sha256.

Usage: python corpus/fetch.py [--tier 1|2|all] [name ...]

Specs are data. Tier 1 files are downloaded and hashed and never parsed or executed here. The
Tier 2 directory is a blobless, no-checkout git clone with hooks, LFS and submodules disabled.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import cast
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parent
MANIFEST = ROOT / "manifest.yaml"
CACHE = ROOT / "cache"

SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
NAME_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
FORMATS = {"json": "json", "yaml": "yaml"}
TIMEOUT_S = 300
CHUNK = 1 << 20

GIT_SAFE = (
    "-c",
    "core.hooksPath=/dev/null",
    "-c",
    "filter.lfs.smudge=",
    "-c",
    "filter.lfs.process=",
    "-c",
    "filter.lfs.required=false",
    "-c",
    "submodule.recurse=false",
    "-c",
    "protocol.file.allow=never",
)


class ManifestError(ValueError):
    """The manifest is malformed."""


class VerifyError(RuntimeError):
    """A download does not match the manifest."""


@dataclass(frozen=True, slots=True)
class Spec:
    """One pinned Tier 1 spec file."""

    name: str
    url: str
    format: str
    sha256: str
    size: int
    source_sha: str | None


@dataclass(frozen=True, slots=True)
class GitSource:
    """One pinned Tier 2 git repository."""

    name: str
    url: str
    sha: str


@dataclass(frozen=True, slots=True)
class Manifest:
    """The parsed corpus manifest."""

    tier1: list[Spec]
    tier2: list[GitSource]


def strip_comments(text: str) -> str:
    """Drop leading `#` comment lines so the remaining YAML is a plain JSON document."""
    lines = text.splitlines()
    while lines and (not lines[0].strip() or lines[0].lstrip().startswith("#")):
        lines.pop(0)
    return "\n".join(lines)


def _obj(raw: object, what: str) -> dict[str, object]:
    if not isinstance(raw, dict):
        msg = f"{what} must be an object"
        raise ManifestError(msg)
    return cast("dict[str, object]", raw)


def _str(entry: dict[str, object], key: str) -> str:
    value = entry.get(key)
    if not isinstance(value, str) or not value:
        msg = f"{entry.get('name', '?')}: '{key}' must be a non-empty string"
        raise ManifestError(msg)
    return value


def _name(entry: dict[str, object]) -> str:
    name = _str(entry, "name")
    if not NAME_RE.match(name):
        msg = f"{name}: name must be lowercase kebab-case"
        raise ManifestError(msg)
    return name


def parse_spec(raw: object) -> Spec:
    """Validate one Tier 1 entry; GitHub-hosted URLs must be raw URLs at the pinned commit."""
    entry = _obj(raw, "tier1 entry")
    name = _name(entry)
    url = _str(entry, "url")
    if urlparse(url).scheme != "https":
        msg = f"{name}: url must be https"
        raise ManifestError(msg)
    digest = _str(entry, "sha256")
    if not SHA256_RE.match(digest):
        msg = f"{name}: sha256 must be 64 lowercase hex chars"
        raise ManifestError(msg)
    fmt = _str(entry, "format")
    if fmt not in FORMATS:
        msg = f"{name}: format must be json or yaml"
        raise ManifestError(msg)
    size = entry.get("size")
    if not isinstance(size, int) or isinstance(size, bool) or size < 1:
        msg = f"{name}: size must be a positive integer"
        raise ManifestError(msg)
    source_sha: str | None = None
    source = entry.get("source")
    if source is not None:
        src = _obj(source, f"{name}: source")
        repo, source_sha, path = _str(src, "repo"), _str(src, "sha"), _str(src, "path")
        if not SHA_RE.match(source_sha):
            msg = f"{name}: source.sha must be a full 40-char commit SHA"
            raise ManifestError(msg)
        if url != f"https://raw.githubusercontent.com/{repo}/{source_sha}/{path}":
            msg = f"{name}: url must be the raw URL of source.path at source.sha"
            raise ManifestError(msg)
    elif urlparse(url).netloc in {"github.com", "raw.githubusercontent.com"}:
        msg = f"{name}: GitHub-hosted specs must record a pinned source"
        raise ManifestError(msg)
    return Spec(name=name, url=url, format=fmt, sha256=digest, size=size, source_sha=source_sha)


def parse_git(raw: object) -> GitSource:
    """Validate one Tier 2 git entry."""
    entry = _obj(raw, "tier2 entry")
    name = _name(entry)
    if entry.get("kind") != "git":
        msg = f"{name}: only kind 'git' is supported in tier2"
        raise ManifestError(msg)
    repo, url, sha = _str(entry, "repo"), _str(entry, "url"), _str(entry, "sha")
    if url != f"https://github.com/{repo}.git":
        msg = f"{name}: url must be https://github.com/{repo}.git"
        raise ManifestError(msg)
    if not SHA_RE.match(sha):
        msg = f"{name}: sha must be a full 40-char commit SHA"
        raise ManifestError(msg)
    return GitSource(name=name, url=url, sha=sha)


def parse_manifest(text: str) -> Manifest:
    """Parse and validate the whole manifest text."""
    try:
        data: object = json.loads(strip_comments(text))
    except json.JSONDecodeError as exc:
        msg = f"manifest is not JSON-subset YAML: {exc}"
        raise ManifestError(msg) from exc
    root = _obj(data, "manifest root")
    if root.get("version") != 1:
        msg = "manifest version must be 1"
        raise ManifestError(msg)
    t1, t2 = root.get("tier1"), root.get("tier2")
    if not isinstance(t1, list) or not isinstance(t2, list):
        msg = "'tier1' and 'tier2' must be lists"
        raise ManifestError(msg)
    manifest = Manifest(
        tier1=[parse_spec(x) for x in cast("list[object]", t1)],
        tier2=[parse_git(x) for x in cast("list[object]", t2)],
    )
    names = [s.name for s in manifest.tier1] + [g.name for g in manifest.tier2]
    if len(set(names)) != len(names):
        msg = "corpus names must be unique"
        raise ManifestError(msg)
    return manifest


def cache_path(spec: Spec, cache: Path = CACHE) -> Path:
    """Where a Tier 1 spec lives on disk."""
    return cache / "tier1" / f"{spec.name}.{spec.format}"


def sha256_file(path: Path) -> str:
    """Stream a file through sha256."""
    digest = hashlib.sha256()
    with path.open("rb") as fh:
        while chunk := fh.read(CHUNK):
            digest.update(chunk)
    return digest.hexdigest()


def fetch_spec(spec: Spec, cache: Path = CACHE) -> Path:
    """Download a spec unless a verified copy is cached; fail on hash mismatch."""
    dest = cache_path(spec, cache)
    if dest.is_file() and sha256_file(dest) == spec.sha256:
        return dest
    dest.parent.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(spec.url, headers={"User-Agent": "ferrule-corpus-fetch"})  # noqa: S310 - https enforced by parse_spec
    digest = hashlib.sha256()
    with (
        urllib.request.urlopen(request, timeout=TIMEOUT_S) as resp,  # noqa: S310 - https enforced
        tempfile.NamedTemporaryFile(dir=dest.parent, delete=False) as tmp,
    ):
        while chunk := cast("bytes", resp.read(CHUNK)):
            digest.update(chunk)
            tmp.write(chunk)
    tmp_path = Path(tmp.name)
    if digest.hexdigest() != spec.sha256:
        tmp_path.unlink()
        msg = f"{spec.name}: sha256 is {digest.hexdigest()}, want {spec.sha256}"
        raise VerifyError(msg)
    tmp_path.replace(dest)
    return dest


def _git(*args: str, cwd: Path | None = None) -> str:
    env = {**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_LFS_SKIP_SMUDGE": "1"}
    return subprocess.run(  # noqa: S603 - fixed git argv, no shell
        ["git", *GIT_SAFE, *args],  # noqa: S607 - git from PATH is intended
        cwd=cwd,
        env=env,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def fetch_git(source: GitSource, cache: Path = CACHE) -> Path:
    """Blobless, no-checkout clone; verify the pinned commit exists and point HEAD at it."""
    dest = cache / "tier2" / source.name
    if not (dest / ".git").is_dir():
        dest.parent.mkdir(parents=True, exist_ok=True)
        _git("clone", "--filter=blob:none", "--no-checkout", source.url, str(dest))
    try:
        _git("cat-file", "-e", f"{source.sha}^{{commit}}", cwd=dest)
    except subprocess.CalledProcessError:
        _git("fetch", "--filter=blob:none", "origin", source.sha, cwd=dest)
    _git("update-ref", "--no-deref", "HEAD", source.sha, cwd=dest)
    head = _git("rev-parse", "HEAD", cwd=dest)
    if head != source.sha:
        msg = f"{source.name}: HEAD is {head}, expected {source.sha}"
        raise VerifyError(msg)
    return dest


def main(argv: list[str]) -> int:
    """Entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tier", choices=["1", "2", "all"], default="1")
    parser.add_argument("names", nargs="*")
    args = parser.parse_args(argv)
    tier = cast("str", args.tier)
    wanted = set(cast("list[str]", args.names))
    try:
        manifest = parse_manifest(MANIFEST.read_text(encoding="utf-8"))
    except ManifestError as exc:
        sys.stderr.write(f"error: {exc}\n")
        return 2
    known = {s.name for s in manifest.tier1} | {g.name for g in manifest.tier2}
    if unknown := wanted - known:
        sys.stderr.write(f"error: unknown corpus entries: {', '.join(sorted(unknown))}\n")
        return 2
    jobs: list[Spec | GitSource] = []
    if tier in {"1", "all"}:
        jobs += [s for s in manifest.tier1 if not wanted or s.name in wanted]
    if tier in {"2", "all"}:
        jobs += [g for g in manifest.tier2 if not wanted or g.name in wanted]
    failed = 0
    for job in jobs:
        try:
            dest = fetch_spec(job) if isinstance(job, Spec) else fetch_git(job)
        except (VerifyError, OSError, subprocess.CalledProcessError) as exc:
            detail = exc.stderr if isinstance(exc, subprocess.CalledProcessError) else exc
            sys.stderr.write(f"FAIL {job.name}: {detail}\n")
            failed += 1
        else:
            sys.stdout.write(f"ok   {job.name} -> {dest}\n")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
