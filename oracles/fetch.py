# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Clone Tier 0 oracle repos into oracles/cache/<name>, pinned to their last Stainless commit.

Usage: python oracles/fetch.py [--kind sdk|config] [--language LANG|all] [--group G ...]
                               [name-or-group ...]

Groups (and "everything") are narrowed to one SDK language, python by default, plus the configs
those SDKs pair with; an oracle named exactly is always fetched. `config` oracles also get their
OpenAPI spec: from the checkout when the manifest gives a `path`, or downloaded into
oracles/cache/specs/ when it gives a `url`. Both are checked by md5.

Oracle checkouts are data. This script never runs code from them: git hooks, LFS filters and
submodules are disabled, and nothing inside the checkout is installed or imported.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING, cast

if TYPE_CHECKING:
    from collections.abc import Callable, Sequence

ROOT = Path(__file__).resolve().parent
MANIFEST = ROOT / "manifest.yaml"
CACHE = ROOT / "cache"

SHA_RE = re.compile(r"^[0-9a-f]{40}$")
MD5_RE = re.compile(r"^[0-9a-f]{32}$")
NAME_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
STATS_KEYS = ("configured_endpoints", "openapi_spec_url", "openapi_spec_hash", "config_hash")
KINDS = ("sdk", "config")
PAIRED = ("proven", "nearest")
# Stainless SDK targets we keep oracles for. A `kind: sdk` entry without `language` is python.
LANGUAGES = ("python", "typescript", "go", "java", "kotlin", "ruby", "php", "csharp")
DEFAULT_LANGUAGE = "python"
ALL_LANGUAGES = "all"
# Targets whose generated shape depends on the config key or an option (oracles/README.md).
LAYOUTS: dict[str, tuple[str, ...]] = {"typescript": ("node", "typescript"), "go": ("v1", "v2")}
LICENSES = ("Apache-2.0", "MIT")
SPEC_EXTS = (".yml", ".yaml", ".json")
MAX_SPEC_BYTES = 64 * 1024 * 1024

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
class Spec:
    """Where a config oracle's OpenAPI spec comes from: a file in the checkout or a URL."""

    path: str | None
    url: str | None
    md5: str


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
    license: str | None = None
    config_path: str | None = None
    spec: Spec | None = None
    paired: str | None = None
    language: str | None = None
    layout: str | None = None


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


def _is_relative(path: str) -> bool:
    pure = PurePosixPath(path)
    return bool(path) and not pure.is_absolute() and ".." not in pure.parts and "\\" not in path


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
        if not isinstance(digest, str) or not MD5_RE.match(digest) or not _is_relative(path):
            msg = f"{name}: verify_md5[{path!r}] must be a relative path -> md5 hex digest"
            raise ManifestError(msg)
        out[path] = digest
    return out


def _parse_spec(name: str, raw: object, verify_md5: dict[str, str]) -> Spec:
    if not isinstance(raw, dict):
        msg = f"{name}: config oracles need 'spec' with either 'path' or 'url' + 'md5'"
        raise ManifestError(msg)
    spec = cast("dict[str, object]", raw)
    if sorted(spec) == ["path"]:
        path = spec["path"]
        if not isinstance(path, str) or not _is_relative(path) or path not in verify_md5:
            msg = f"{name}: spec.path must be a relative path that is also in verify_md5"
            raise ManifestError(msg)
        return Spec(path=path, url=None, md5=verify_md5[path])
    if sorted(spec) == ["md5", "url"]:
        url, digest = spec["url"], spec["md5"]
        if not isinstance(url, str) or urllib.parse.urlsplit(url).scheme != "https":
            msg = f"{name}: spec.url must be an https URL"
            raise ManifestError(msg)
        if PurePosixPath(urllib.parse.urlsplit(url).path).suffix not in SPEC_EXTS:
            msg = f"{name}: spec.url must end in one of {', '.join(SPEC_EXTS)}"
            raise ManifestError(msg)
        if not isinstance(digest, str) or not MD5_RE.match(digest):
            msg = f"{name}: spec.md5 must be a 32-char md5 hex digest"
            raise ManifestError(msg)
        return Spec(path=None, url=url, md5=digest)
    msg = f"{name}: spec must have exactly 'path', or exactly 'url' and 'md5'"
    raise ManifestError(msg)


def _parse_config_fields(
    name: str, entry: dict[str, object], verify_md5: dict[str, str]
) -> tuple[str, Spec, str | None]:
    """Return (config_path, spec, paired).

    `pairs_with` and `paired` name and qualify the config's Python SDK. A config without a
    Python target has neither; another language's SDK names it instead (`_check_pairs`).
    """
    config_path = _str(entry, "config_path")
    if not _is_relative(config_path) or config_path not in verify_md5:
        msg = f"{name}: config_path must be a relative path that is also in verify_md5"
        raise ManifestError(msg)
    spec = _parse_spec(name, entry.get("spec"), verify_md5)
    if entry.get("pairs_with") is None:
        if "paired" in entry:
            msg = f"{name}: paired qualifies the python sdk in pairs_with, which is null"
            raise ManifestError(msg)
        return config_path, spec, None
    paired = _str(entry, "paired")
    if paired not in PAIRED:
        msg = f"{name}: paired must be one of {', '.join(PAIRED)}"
        raise ManifestError(msg)
    return config_path, spec, paired


def _parse_links(name: str, entry: dict[str, object]) -> tuple[str | None, str | None]:
    """Return (pairs_with, license)."""
    pairs_with = entry.get("pairs_with")
    if pairs_with is not None and not isinstance(pairs_with, str):
        msg = f"{name}: pairs_with must be a string or null"
        raise ManifestError(msg)
    license_ = entry.get("license")
    if license_ is None:
        return pairs_with, None
    if not isinstance(license_, str) or license_ not in LICENSES:
        msg = f"{name}: license must be one of {', '.join(LICENSES)} (ADR 0009)"
        raise ManifestError(msg)
    return pairs_with, license_


def _parse_language(
    name: str, kind: str, entry: dict[str, object]
) -> tuple[str | None, str | None]:
    """Return (language, layout). An sdk oracle defaults to python; a config has neither."""
    if kind == "config":
        if "language" in entry or "layout" in entry:
            msg = f"{name}: language and layout are only for sdk oracles"
            raise ManifestError(msg)
        return None, None
    language = entry.get("language", DEFAULT_LANGUAGE)
    if not isinstance(language, str) or language not in LANGUAGES:
        msg = f"{name}: language must be one of {', '.join(LANGUAGES)}"
        raise ManifestError(msg)
    layouts = LAYOUTS.get(language)
    layout = entry.get("layout")
    if layouts is None:
        if layout is not None:
            msg = f"{name}: layout is only for {' and '.join(LAYOUTS)} sdk oracles"
            raise ManifestError(msg)
        return language, None
    if not isinstance(layout, str) or layout not in layouts:
        msg = f"{name}: a {language} sdk oracle needs layout, one of {', '.join(layouts)}"
        raise ManifestError(msg)
    return language, layout


def _parse_sdk_paired(name: str, language: str | None, entry: dict[str, object]) -> str | None:
    """`paired` on an sdk oracle: only non-Python SDKs carry their own pairing."""
    if "paired" not in entry:
        return None
    paired = entry["paired"]
    if language == DEFAULT_LANGUAGE:
        msg = f"{name}: a python sdk's pairing is recorded on its config oracle"
        raise ManifestError(msg)
    if not isinstance(paired, str) or paired not in PAIRED:
        msg = f"{name}: paired must be one of {', '.join(PAIRED)}"
        raise ManifestError(msg)
    if entry.get("pairs_with") is None:
        msg = f"{name}: paired needs pairs_with to name a config oracle"
        raise ManifestError(msg)
    return paired


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
    if kind not in KINDS:
        msg = f"{name}: kind must be 'sdk' or 'config'"
        raise ManifestError(msg)
    stats = _parse_stats(name, entry.get("stats"))
    if kind == "sdk" and stats is None:
        msg = f"{name}: sdk oracles must record .stats.yml values"
        raise ManifestError(msg)
    pairs_with, license_ = _parse_links(name, entry)
    verify_md5 = _parse_md5_map(name, entry.get("verify_md5", {}))
    language, layout = _parse_language(name, kind, entry)
    config_path: str | None = None
    spec: Spec | None = None
    paired: str | None = None
    if kind == "config":
        config_path, spec, paired = _parse_config_fields(name, entry, verify_md5)
    elif any(key in entry for key in ("config_path", "spec")):
        msg = f"{name}: config_path and spec are only for config oracles"
        raise ManifestError(msg)
    else:
        paired = _parse_sdk_paired(name, language, entry)
    return Oracle(
        name=name,
        group=_str(entry, "group"),
        kind=kind,
        repo=repo,
        url=url,
        sha=sha,
        commit_date=_str(entry, "commit_date"),
        stats=stats,
        verify_md5=verify_md5,
        pairs_with=pairs_with,
        license=license_,
        config_path=config_path,
        spec=spec,
        paired=paired,
        language=language,
        layout=layout,
    )


def check_pair(config: Oracle, sdk: Oracle, paired: str | None = None) -> None:
    """Check a config oracle against an sdk oracle generated from it (ADR 0009).

    `proven` means md5(config) is the SDK's `config_hash` and md5(spec) its `openapi_spec_hash`.
    `nearest` means the two are only close in time, so their hashes must not both match.
    `paired` defaults to the config's own, which is about its Python SDK; another language's
    SDK passes its own.
    """
    paired = paired or config.paired
    subject = sdk.name if sdk.paired else config.name
    if sdk.kind != "sdk" or sdk.stats is None:
        msg = f"{config.name}: pairs_with {sdk.name!r} must be an sdk oracle"
        raise ManifestError(msg)
    if sdk.license is None:
        msg = f"{sdk.name}: sdk oracles paired with a config must record their license"
        raise ManifestError(msg)
    if config.config_path is None or config.spec is None:
        msg = f"{config.name}: config oracles need config_path and spec"
        raise ManifestError(msg)
    config_md5 = config.verify_md5[config.config_path]
    matches = (
        config_md5 == sdk.stats["config_hash"],
        config.spec.md5 == sdk.stats["openapi_spec_hash"],
    )
    if paired == "proven" and not all(matches):
        msg = (
            f"{subject}: paired is 'proven' but md5(config), md5(spec) = "
            f"{config_md5}, {config.spec.md5}; {sdk.name} has config_hash, openapi_spec_hash = "
            f"{sdk.stats['config_hash']}, {sdk.stats['openapi_spec_hash']}"
        )
        raise ManifestError(msg)
    if paired == "nearest" and all(matches):
        msg = f"{subject}: hashes match {config.name} and {sdk.name}, so paired should be 'proven'"
        raise ManifestError(msg)


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
    _check_pairs(oracles)
    return oracles


def _check_pairs(oracles: list[Oracle]) -> None:
    """Check every pairs_with link and the md5 pairing behind it."""
    by_name = {o.name: o for o in oracles}
    for o in oracles:
        if o.pairs_with is not None and o.pairs_with not in by_name:
            msg = f"{o.name}: pairs_with {o.pairs_with!r} is not an oracle"
            raise ManifestError(msg)
        if o.kind == "config" and o.pairs_with is not None:
            check_pair(o, by_name[o.pairs_with])
    for o in oracles:
        if o.kind == "sdk" and o.language != DEFAULT_LANGUAGE and o.pairs_with is not None:
            _check_language_pair(o, by_name[o.pairs_with])
    for config in (o for o in oracles if o.kind == "config" and o.pairs_with is None):
        if not paired_sdks(oracles, config):
            msg = f"{config.name}: no sdk oracle pairs with this config"
            raise ManifestError(msg)
    _check_one_sdk_per_language(oracles)


def _check_language_pair(sdk: Oracle, config: Oracle) -> None:
    """Check a non-Python sdk against the config it names, using its own `paired`."""
    if config.kind != "config":
        msg = f"{sdk.name}: pairs_with {config.name!r} must be a config oracle"
        raise ManifestError(msg)
    if sdk.paired is None:
        msg = f"{sdk.name}: a {sdk.language} sdk paired with a config needs 'paired'"
        raise ManifestError(msg)
    if sdk.group != config.group:
        msg = f"{sdk.name}: group must be {config.group!r}, the group of {config.name}"
        raise ManifestError(msg)
    check_pair(config, sdk, sdk.paired)


def _check_one_sdk_per_language(oracles: list[Oracle]) -> None:
    """At most one sdk per config and language, so a language fixes the pair."""
    seen: dict[tuple[str, str], str] = {}
    for config in (o for o in oracles if o.kind == "config"):
        for sdk in paired_sdks(oracles, config):
            key = (config.name, sdk_language(sdk))
            if key in seen:
                msg = f"{config.name}: {seen[key]} and {sdk.name} are both {key[1]} sdks"
                raise ManifestError(msg)
            seen[key] = sdk.name


def sdk_language(oracle: Oracle) -> str:
    """Return an sdk oracle's language (python when the entry has none)."""
    return oracle.language or DEFAULT_LANGUAGE


def paired_sdks(oracles: Sequence[Oracle], config: Oracle) -> list[Oracle]:
    """Return the sdk oracles generated from a config: the one it names, those naming it."""
    return [
        o
        for o in oracles
        if o.kind == "sdk" and (o.name == config.pairs_with or o.pairs_with == config.name)
    ]


def _in_language(oracles: list[Oracle], picked: list[Oracle], language: str) -> set[str]:
    """Names in `picked` that belong to `language`: its sdks and the configs they pair with."""
    keep = {o.name for o in picked if o.kind == "sdk" and sdk_language(o) == language}
    for config in (o for o in picked if o.kind == "config"):
        if any(sdk.name in keep for sdk in paired_sdks(oracles, config)):
            keep.add(config.name)
    return keep


def select(
    oracles: list[Oracle],
    wanted: list[str],
    kind: str | None = None,
    language: str = DEFAULT_LANGUAGE,
) -> list[Oracle]:
    """Pick oracles by name or group (no names selects all), optionally only one kind.

    Groups and "all" keep only the sdks in `language` and the configs they pair with
    (`ALL_LANGUAGES` keeps every one). An oracle named exactly is kept whatever its language.
    """
    if language != ALL_LANGUAGES and language not in LANGUAGES:
        msg = f"unknown language {language!r}: use one of {', '.join(LANGUAGES)} or all"
        raise ManifestError(msg)
    unknown = [w for w in wanted if not any(w in {o.name, o.group} for o in oracles)]
    if unknown:
        msg = f"unknown oracle(s): {', '.join(unknown)}"
        raise ManifestError(msg)
    grouped = [o for o in oracles if not wanted or o.group in wanted]
    keep = (
        {o.name for o in grouped}
        if language == ALL_LANGUAGES
        else _in_language(oracles, grouped, language)
    )
    picked = [o for o in oracles if o.name in wanted or o.name in keep]
    return [o for o in picked if kind is None or o.kind == kind]


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
    if urllib.parse.urlsplit(oracle.url).scheme != "https":
        msg = f"{oracle.name}: refusing non-https clone URL {oracle.url}"
        raise VerifyError(msg)
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


def _md5(data: bytes) -> str:
    return hashlib.md5(data, usedforsecurity=False).hexdigest()


def verify_files(oracle: Oracle, dest: Path) -> None:
    """Check .stats.yml and pinned file digests against the manifest."""
    if oracle.stats is not None:
        found = parse_stats_yml((dest / ".stats.yml").read_text(encoding="utf-8"))
        for key, expected in oracle.stats.items():
            if found.get(key) != expected:
                msg = f"{oracle.name}: .stats.yml {key} is {found.get(key)!r}, want {expected!r}"
                raise VerifyError(msg)
    for rel, expected in oracle.verify_md5.items():
        digest = _md5((dest / rel).read_bytes())
        if digest != expected:
            msg = f"{oracle.name}: md5({rel}) is {digest}, want {expected}"
            raise VerifyError(msg)


def download(url: str) -> bytes:
    """Fetch an https URL of at most MAX_SPEC_BYTES. The bytes are data, never executed."""
    if urllib.parse.urlsplit(url).scheme != "https":
        msg = f"refusing non-https URL {url}"
        raise VerifyError(msg)
    with urllib.request.urlopen(url, timeout=120) as response:  # noqa: S310 - https checked above
        data = cast("bytes", response.read(MAX_SPEC_BYTES + 1))
    if len(data) > MAX_SPEC_BYTES:
        msg = f"{url} is larger than {MAX_SPEC_BYTES} bytes"
        raise VerifyError(msg)
    return data


def spec_file(oracle: Oracle, dest: Path, cache: Path = CACHE) -> Path:
    """Where a config oracle's spec lives: in its checkout, or cache/specs/<name>.<ext> (URL)."""
    if oracle.spec is None:
        msg = f"{oracle.name}: not a config oracle"
        raise VerifyError(msg)
    if oracle.spec.path is not None:
        return dest / oracle.spec.path
    suffix = PurePosixPath(urllib.parse.urlsplit(cast("str", oracle.spec.url)).path).suffix
    return cache / "specs" / f"{oracle.name}{suffix}"


def fetch_spec(
    oracle: Oracle,
    dest: Path,
    cache: Path = CACHE,
    fetch: Callable[[str], bytes] = download,
) -> Path:
    """Make sure a config oracle's spec is on disk with the manifest's md5; return its path."""
    target = spec_file(oracle, dest, cache)
    spec = cast("Spec", oracle.spec)
    if spec.url is not None and not (target.is_file() and _md5(target.read_bytes()) == spec.md5):
        data = fetch(spec.url)
        if _md5(data) != spec.md5:
            msg = f"{oracle.name}: md5({spec.url}) is {_md5(data)}, want {spec.md5}"
            raise VerifyError(msg)
        target.parent.mkdir(parents=True, exist_ok=True)
        partial = target.with_name(target.name + ".part")
        partial.write_bytes(data)
        partial.replace(target)
    digest = _md5(target.read_bytes())
    if digest != spec.md5:
        msg = f"{oracle.name}: md5({target}) is {digest}, want {spec.md5}"
        raise VerifyError(msg)
    return target


@dataclass(frozen=True, slots=True)
class Args:
    """What to fetch: names or groups, narrowed by kind and language."""

    names: list[str]
    kind: str | None = None
    language: str = DEFAULT_LANGUAGE
    groups: tuple[str, ...] = ()

    def wanted(self, oracles: list[Oracle]) -> list[str]:
        """Names and groups for `select`; each `--group` must be a group."""
        known = {o.group for o in oracles}
        bad = [g for g in self.groups if g not in known]
        if bad:
            msg = f"unknown group(s): {', '.join(bad)}"
            raise ManifestError(msg)
        return [*self.groups, *self.names]


def parse_args(argv: list[str]) -> Args:
    """Parse the command line."""
    parser = argparse.ArgumentParser(description="Fetch and verify pinned oracle repos.")
    parser.add_argument("--kind", choices=KINDS, help="only oracles of this kind")
    parser.add_argument(
        "--language",
        choices=(*LANGUAGES, ALL_LANGUAGES),
        default=DEFAULT_LANGUAGE,
        help=f"sdk language that groups and 'all' are narrowed to (default {DEFAULT_LANGUAGE})",
    )
    parser.add_argument("--group", action="append", default=[], help="an oracle group")
    parser.add_argument("names", nargs="*", help="oracle names or groups (default: all)")
    args = parser.parse_args(argv)
    return Args(
        cast("list[str]", args.names),
        cast("str | None", args.kind),
        cast("str", args.language),
        tuple(cast("list[str]", args.group)),
    )


def main(argv: list[str]) -> int:
    """Entry point."""
    args = parse_args(argv)
    try:
        manifest = parse_manifest(MANIFEST.read_text(encoding="utf-8"))
        oracles = select(manifest, args.wanted(manifest), args.kind, args.language)
    except ManifestError as exc:
        sys.stderr.write(f"error: {exc}\n")
        return 2
    failed = 0
    for oracle in oracles:
        try:
            dest = checkout(oracle)
            verify_files(oracle, dest)
            extra = ""
            if oracle.kind == "config":
                spec = fetch_spec(oracle, dest)
                extra = f" ({oracle.paired}; config {oracle.config_path}; spec {spec})"
            elif sdk_language(oracle) != DEFAULT_LANGUAGE:
                layout = f", {oracle.layout} layout" if oracle.layout else ""
                extra = f" ({oracle.language}{layout}; {oracle.paired} with {oracle.pairs_with})"
        except (VerifyError, OSError, subprocess.CalledProcessError) as exc:
            detail = exc.stderr if isinstance(exc, subprocess.CalledProcessError) else exc
            sys.stderr.write(f"FAIL {oracle.name}: {detail}\n")
            failed += 1
        else:
            sys.stdout.write(f"ok   {oracle.name} {oracle.sha[:12]} -> {dest}{extra}\n")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
