"""Clone Tier 0 oracle repos into oracles/cache/<name>, pinned to their last Stainless commit.

Usage: python oracles/fetch.py [name-or-group ...]

Oracle checkouts are data. This script never runs code from them: git hooks, LFS filters and
submodules are disabled, and nothing inside the checkout is installed or imported.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parent
MANIFEST = ROOT / "manifest.yaml"
CACHE = ROOT / "cache"

SHA_RE = re.compile(r"^[0-9a-f]{40}$")
MD5_RE = re.compile(r"^[0-9a-f]{32}$")
NAME_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
STATS_KEYS = ("configured_endpoints", "openapi_spec_url", "openapi_spec_hash", "config_hash")

# Every git call gets these: no hooks, no LFS smudge, no submodules, no local-file transports.
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
    "-c",
    "advice.detachedHead=false",
)


class ManifestError(ValueError):
    """The manifest is malformed."""


class VerifyError(RuntimeError):
    """A checkout does not match the manifest."""


@dataclass(frozen=True, slots=True)
class Oracle:
    """One pinned oracle repository."""

    name: str
    group: str
    kind: str
    repo: str
    url: str
    sha: str
    commit_date: str
    stats: dict[str, str] | None
    verify_md5: dict[str, str]
    pairs_with: str | None


def strip_comments(text: str) -> str:
    """Drop leading `#` comment lines so the remaining YAML is a plain JSON document."""
    lines = text.splitlines()
    while lines and (not lines[0].strip() or lines[0].lstrip().startswith("#")):
        lines.pop(0)
    return "\n".join(lines)


def _str(entry: dict[str, object], key: str) -> str:
    value = entry.get(key)
    if not isinstance(value, str) or not value:
        msg = f"{entry.get('name', '?')}: '{key}' must be a non-empty string"
        raise ManifestError(msg)
    return value


def _parse_stats(name: str, raw: object) -> dict[str, str] | None:
    if raw is None:
        return None
    if not isinstance(raw, dict):
        msg = f"{name}: 'stats' must be an object or null"
        raise ManifestError(msg)
    stats = cast("dict[str, object]", raw)
    if sorted(stats) != sorted(STATS_KEYS):
        msg = f"{name}: 'stats' must have exactly {', '.join(STATS_KEYS)}"
        raise ManifestError(msg)
    for key in ("openapi_spec_hash", "config_hash"):
        if not isinstance(stats[key], str) or not MD5_RE.match(cast("str", stats[key])):
            msg = f"{name}: stats.{key} must be a 32-char md5 hex digest"
            raise ManifestError(msg)
    endpoints = stats["configured_endpoints"]
    if not isinstance(endpoints, int) or isinstance(endpoints, bool) or endpoints < 1:
        msg = f"{name}: stats.configured_endpoints must be a positive integer"
        raise ManifestError(msg)
    return {key: str(stats[key]) for key in STATS_KEYS}


def _parse_md5_map(name: str, raw: object) -> dict[str, str]:
    if not isinstance(raw, dict):
        msg = f"{name}: 'verify_md5' must be an object"
        raise ManifestError(msg)
    out: dict[str, str] = {}
    for path, digest in cast("dict[str, object]", raw).items():
        if not isinstance(digest, str) or not MD5_RE.match(digest) or path.startswith(("/", "..")):
            msg = f"{name}: verify_md5[{path!r}] must be a relative path -> md5 hex digest"
            raise ManifestError(msg)
        out[path] = digest
    return out


def parse_entry(raw: object) -> Oracle:
    """Validate one manifest entry."""
    if not isinstance(raw, dict):
        msg = "each oracle must be an object"
        raise ManifestError(msg)
    entry = cast("dict[str, object]", raw)
    name = _str(entry, "name")
    if not NAME_RE.match(name):
        msg = f"{name}: name must be lowercase kebab-case"
        raise ManifestError(msg)
    sha = _str(entry, "sha")
    if not SHA_RE.match(sha):
        msg = f"{name}: sha must be a full 40-char commit SHA"
        raise ManifestError(msg)
    repo = _str(entry, "repo")
    url = _str(entry, "url")
    if url != f"https://github.com/{repo}.git":
        msg = f"{name}: url must be https://github.com/{repo}.git"
        raise ManifestError(msg)
    kind = _str(entry, "kind")
    if kind not in {"sdk", "config"}:
        msg = f"{name}: kind must be 'sdk' or 'config'"
        raise ManifestError(msg)
    stats = _parse_stats(name, entry.get("stats"))
    if kind == "sdk" and stats is None:
        msg = f"{name}: sdk oracles must record .stats.yml values"
        raise ManifestError(msg)
    pairs_with = entry.get("pairs_with")
    if pairs_with is not None and not isinstance(pairs_with, str):
        msg = f"{name}: pairs_with must be a string or null"
        raise ManifestError(msg)
    return Oracle(
        name=name,
        group=_str(entry, "group"),
        kind=kind,
        repo=repo,
        url=url,
        sha=sha,
        commit_date=_str(entry, "commit_date"),
        stats=stats,
        verify_md5=_parse_md5_map(name, entry.get("verify_md5", {})),
        pairs_with=pairs_with,
    )


def parse_manifest(text: str) -> list[Oracle]:
    """Parse and validate the whole manifest text."""
    try:
        data: object = json.loads(strip_comments(text))
    except json.JSONDecodeError as exc:
        msg = f"manifest is not JSON-subset YAML: {exc}"
        raise ManifestError(msg) from exc
    if not isinstance(data, dict):
        msg = "manifest root must be an object"
        raise ManifestError(msg)
    root = cast("dict[str, object]", data)
    if root.get("version") != 1:
        msg = "manifest version must be 1"
        raise ManifestError(msg)
    raw = root.get("oracles")
    if not isinstance(raw, list):
        msg = "'oracles' must be a list"
        raise ManifestError(msg)
    oracles = [parse_entry(item) for item in cast("list[object]", raw)]
    names = [o.name for o in oracles]
    if len(set(names)) != len(names):
        msg = "oracle names must be unique"
        raise ManifestError(msg)
    for o in oracles:
        if o.pairs_with is not None and o.pairs_with not in names:
            msg = f"{o.name}: pairs_with {o.pairs_with!r} is not an oracle"
            raise ManifestError(msg)
    return oracles


def select(oracles: list[Oracle], wanted: list[str]) -> list[Oracle]:
    """Pick oracles by name or group; no arguments selects all."""
    if not wanted:
        return oracles
    unknown = [w for w in wanted if not any(w in {o.name, o.group} for o in oracles)]
    if unknown:
        msg = f"unknown oracle(s): {', '.join(unknown)}"
        raise ManifestError(msg)
    return [o for o in oracles if o.name in wanted or o.group in wanted]


def parse_stats_yml(text: str) -> dict[str, str]:
    """Read the flat `key: value` lines of a Stainless .stats.yml."""
    out: dict[str, str] = {}
    for line in text.splitlines():
        key, sep, value = line.partition(":")
        if sep and key.strip() and not line.startswith((" ", "#")):
            out[key.strip()] = value.strip().strip("'\"")
    return out


def _git(*args: str, cwd: Path | None = None) -> str:
    env = {**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_LFS_SKIP_SMUDGE": "1"}
    result = subprocess.run(  # noqa: S603 - fixed git argv, no shell
        ["git", *GIT_SAFE, *args],  # noqa: S607 - git from PATH is intended
        cwd=cwd,
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def checkout(oracle: Oracle, cache: Path = CACHE) -> Path:
    """Clone (blobless) or reuse cache/<name> and detach at the pinned SHA."""
    dest = cache / oracle.name
    if not (dest / ".git").is_dir():
        cache.mkdir(parents=True, exist_ok=True)
        _git(
            "clone",
            "--filter=blob:none",
            "--no-checkout",
            "--no-recurse-submodules",
            oracle.url,
            str(dest),
        )
    elif _git("remote", "get-url", "origin", cwd=dest) != oracle.url:
        msg = f"{oracle.name}: {dest} has a different origin; delete it and retry"
        raise VerifyError(msg)
    if _git("rev-parse", "HEAD", cwd=dest) != oracle.sha or _git("status", "--porcelain", cwd=dest):
        try:
            _git("cat-file", "-e", f"{oracle.sha}^{{commit}}", cwd=dest)
        except subprocess.CalledProcessError:
            _git("fetch", "--filter=blob:none", "origin", oracle.sha, cwd=dest)
        _git("checkout", "--force", "--detach", oracle.sha, cwd=dest)
    head = _git("rev-parse", "HEAD", cwd=dest)
    if head != oracle.sha:
        msg = f"{oracle.name}: HEAD is {head}, expected {oracle.sha}"
        raise VerifyError(msg)
    return dest


def verify_files(oracle: Oracle, dest: Path) -> None:
    """Check .stats.yml and pinned file digests against the manifest."""
    if oracle.stats is not None:
        found = parse_stats_yml((dest / ".stats.yml").read_text(encoding="utf-8"))
        for key, expected in oracle.stats.items():
            if found.get(key) != expected:
                msg = f"{oracle.name}: .stats.yml {key} is {found.get(key)!r}, want {expected!r}"
                raise VerifyError(msg)
    for rel, expected in oracle.verify_md5.items():
        digest = hashlib.md5((dest / rel).read_bytes(), usedforsecurity=False).hexdigest()
        if digest != expected:
            msg = f"{oracle.name}: md5({rel}) is {digest}, want {expected}"
            raise VerifyError(msg)


def main(argv: list[str]) -> int:
    """Entry point."""
    try:
        oracles = select(parse_manifest(MANIFEST.read_text(encoding="utf-8")), argv)
    except ManifestError as exc:
        sys.stderr.write(f"error: {exc}\n")
        return 2
    failed = 0
    for oracle in oracles:
        try:
            dest = checkout(oracle)
            verify_files(oracle, dest)
        except (VerifyError, OSError, subprocess.CalledProcessError) as exc:
            detail = exc.stderr if isinstance(exc, subprocess.CalledProcessError) else exc
            sys.stderr.write(f"FAIL {oracle.name}: {detail}\n")
            failed += 1
        else:
            sys.stdout.write(f"ok   {oracle.name} {oracle.sha[:12]} -> {dest}\n")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
