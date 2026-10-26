# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule verify``: public-API diff and text closeness between two SDK trees.

Both packages are loaded statically with griffe (``allow_inspection=False``): nothing from
either tree is imported or run. Breaking changes are griffe's breakage kinds; additive
changes are public objects that only the new tree has, and new parameters with defaults.

Some breakages are only two runtimes spelling the same API (an SDK another generator wrote,
adopted onto Ferrule's runtime): a ``cached_property`` griffe reads as an attribute on one side
and a decorated function on the other, a method and its alias swapping which name is the alias,
or a default moving between omitted-argument sentinels (``NOT_GIVEN`` -> ``omit``). Callers see
no difference, so these are listed as ``normalized``, not breaking.
"""

from __future__ import annotations

import difflib
import io
import subprocess
import tarfile
from dataclasses import asdict, dataclass, field
from typing import TYPE_CHECKING

import griffe

from ferrule.errors import BAD_REF, LOAD_FAILED, fail
from ferrule.generate import relative_files

if TYPE_CHECKING:
    from pathlib import Path

TEXT_SUFFIXES = frozenset({".py", ".pyi", ".md", ".toml", ".yml", ".yaml", ".json", ".txt", ".cfg"})
EXCLUDED_DIRS = frozenset({".venv", "venv", "node_modules", "dist", "build"})
REQUIRED_CAPABLE = frozenset(
    {
        griffe.ParameterKind.positional_or_keyword,
        griffe.ParameterKind.keyword_only,
        griffe.ParameterKind.positional_only,
    },
)
CONTAINERS = frozenset({griffe.Kind.MODULE, griffe.Kind.CLASS})
# Defaults meaning "argument not given", in every runtime spelling (Stainless's older
# `NOT_GIVEN`, today's `omit`/`not_given`); the request leaves the field out for all of them.
OMITTED = frozenset({"NOT_GIVEN", "NotGiven()", "not_given", "omit", "Omit()"})
PROPERTY_DECORATORS = frozenset({"property", "cached_property"})


@dataclass(frozen=True)
class Change:
    """One API difference."""

    kind: str
    path: str
    detail: str


@dataclass(frozen=True)
class FileScore:
    """Closeness of one file, in [0, 1], weighted by its size in lines."""

    path: str
    ratio: float
    weight: int


@dataclass
class Report:
    """Everything ``verify`` found."""

    package: str
    breaking: list[Change] = field(default_factory=list[Change])
    additive: list[Change] = field(default_factory=list[Change])
    normalized: list[Change] = field(default_factory=list[Change])
    closeness: float = 0.0
    files: list[FileScore] = field(default_factory=list[FileScore])

    def summary(self) -> str:
        """Return the one-line verdict."""
        same = f" ({len(self.normalized)} runtime spellings not counted)" if self.normalized else ""
        return (
            f"public API: {len(self.breaking)} breaking · {len(self.additive)} additive{same}\n"
            f"text closeness: {self.closeness:.1f}%"
        )

    def to_json(self) -> dict[str, object]:
        """Return a JSON-ready dict (counts first, then details)."""
        return {
            "package": self.package,
            "breaking_count": len(self.breaking),
            "additive_count": len(self.additive),
            "normalized_count": len(self.normalized),
            "closeness": round(self.closeness, 2),
            "breaking": [asdict(c) for c in self.breaking],
            "additive": [asdict(c) for c in self.additive],
            "normalized": [asdict(c) for c in self.normalized],
            "files": [asdict(f) for f in self.files],
        }


def package_root(tree: Path, package: str) -> Path:
    """Return the directory to put on griffe's search path (``src/`` when the tree has one)."""
    return tree / "src" if (tree / "src" / package).is_dir() else tree


def detect_package(tree: Path) -> str:
    """Return the single package under ``tree/src`` (or ``tree``)."""
    for base in (tree / "src", tree):
        names = (
            sorted(
                p.name
                for p in base.iterdir()
                if (p / "__init__.py").is_file() and p.name.isidentifier()
            )
            if base.is_dir()
            else []
        )
        if len(names) == 1:
            return names[0]
    raise fail(LOAD_FAILED, f"cannot find one package in {tree}", "--against", "pass --package")


def load(tree: Path, package: str) -> griffe.Object:
    """Load ``package`` from ``tree`` statically."""
    try:
        obj = griffe.load(
            package,
            search_paths=[package_root(tree, package)],
            allow_inspection=False,
            try_relative_path=False,
            resolve_aliases=True,
            resolve_external=False,
        )
    except ImportError as exc:
        raise fail(
            LOAD_FAILED, f"griffe cannot find {package} in {tree}", str(tree), "check --package"
        ) from exc
    if isinstance(obj, griffe.Alias):  # pragma: no cover - a top-level package is never an alias
        raise fail(LOAD_FAILED, f"{package} is an alias", str(tree), "check --package")
    return obj


def _value(value: object) -> str:
    if isinstance(value, griffe.Parameter):
        return value.name
    if isinstance(value, griffe.Object | griffe.Alias):
        return value.path
    return str(value)


def breaking_changes(old: griffe.Object, new: griffe.Object) -> list[Change]:
    """Return griffe's breakages as plain text, sorted for stable output."""
    return classify(old, new)[0]


def classify(old: griffe.Object, new: griffe.Object) -> tuple[list[Change], list[Change]]:
    """Split griffe's breakages into breaking ones and runtime spellings of the same API."""
    breaking: list[Change] = []
    same: list[Change] = []
    for b in griffe.find_breaking_changes(old, new):
        change = Change(
            b.kind.name,
            b.obj.path,
            f"{b.obj.path}: {b.kind.value} ({_value(b.old_value)} -> {_value(b.new_value)})",
        )
        (same if _same_api(b, old, new) else breaking).append(change)

    def key(c: Change) -> tuple[str, str, str]:
        return (c.path, c.kind, c.detail)

    return sorted(breaking, key=key), sorted(same, key=key)


def _member(root: griffe.Object, path: str) -> griffe.Object | None:
    """Return the object at dotted ``path`` (starting with the package name) under ``root``."""
    rel = path.removeprefix(f"{root.path}.")
    try:
        found = root[rel]
    except (KeyError, griffe.AliasResolutionError, griffe.CyclicAliasError):
        return None
    return _target(found)


def _counterpart(old: griffe.Object, new: griffe.Object, path: str) -> griffe.Object | None:
    """Return the old object at ``path`` of the new tree, also when its class moved module.

    ``acme.Client`` may be defined in ``acme._client`` before and ``acme.lib._client`` after (an
    adopted SDK's custom client); griffe reports the new path, so it is mapped back through the
    public name both trees export.
    """
    found = _member(old, path)
    if found is not None:
        return found
    for name, member in new.members.items():
        target = _target(member)
        if target is None or not path.startswith(f"{target.path}."):
            continue
        before = _member(old, f"{old.path}.{name}")
        if before is not None:
            return _member(old, before.path + path.removeprefix(target.path))
    return None


def _is_property(obj: griffe.Object) -> bool:
    if isinstance(obj, griffe.Attribute):
        return "property" in obj.labels
    return isinstance(obj, griffe.Function) and any(
        str(d.value).rsplit(".", 1)[-1] in PROPERTY_DECORATORS for d in obj.decorators
    )


def _is_callable(obj: griffe.Object) -> bool:
    """Tell whether ``obj`` is a method, or a class attribute naming one (``copy = f``)."""
    if isinstance(obj, griffe.Function):
        return not _is_property(obj)
    parent = obj.parent
    if not isinstance(obj, griffe.Attribute) or not isinstance(parent, griffe.Class):
        return False
    target = parent.members.get(str(obj.value)) if obj.value is not None else None
    return isinstance(target, griffe.Function)


def _same_api(b: griffe.Breakage, old: griffe.Object, new: griffe.Object) -> bool:
    if b.kind is griffe.BreakageKind.PARAMETER_CHANGED_DEFAULT:
        before, after = b.old_value, b.new_value
        return (
            isinstance(before, griffe.Parameter)
            and isinstance(after, griffe.Parameter)
            and str(before.default) in OMITTED
            and str(after.default) in OMITTED
        )
    if b.kind is not griffe.BreakageKind.OBJECT_CHANGED_KIND:
        return False
    before, after = _counterpart(old, new, b.obj.path), _member(new, b.obj.path)
    if before is None or after is None:
        return False
    return (_is_property(before) and _is_property(after)) or (
        _is_callable(before) and _is_callable(after)
    )


def _target(member: griffe.Object | griffe.Alias) -> griffe.Object | None:
    """Return the object behind ``member``; ``None`` for aliases to outside the package."""
    if isinstance(member, griffe.Object):
        return member
    try:
        return member.final_target
    except (griffe.AliasResolutionError, griffe.CyclicAliasError):
        return None


def _public_members(obj: griffe.Object) -> dict[str, griffe.Object]:
    out: dict[str, griffe.Object] = {}
    for name, member in obj.members.items():
        target = _target(member)
        if target is not None and member.is_public:
            out[name] = target
    return out


def _params(obj: griffe.Object) -> dict[str, griffe.Parameter]:
    return {p.name: p for p in obj.parameters} if isinstance(obj, griffe.Function) else {}


def additive_changes(old: griffe.Object, new: griffe.Object) -> list[Change]:
    """Public objects only ``new`` has, and new optional parameters on shared functions."""
    found: list[Change] = []
    seen: set[str] = set()

    def walk(o: griffe.Object, n: griffe.Object, prefix: str) -> None:
        if n.path in seen:
            return
        seen.add(n.path)
        before_members = _public_members(o)
        for name, member in sorted(_public_members(n).items()):
            path = f"{prefix}.{name}"
            before = before_members.get(name)
            if before is None:
                found.append(Change("OBJECT_ADDED", path, f"{path} was added"))
                continue
            old_params = _params(before)
            found.extend(
                Change("PARAMETER_ADDED_OPTIONAL", path, f"{path}({pname}) was added")
                for pname, param in _params(member).items()
                if pname not in old_params
                and (param.default is not None or param.kind not in REQUIRED_CAPABLE)
            )
            if member.kind in CONTAINERS and before.kind == member.kind:
                walk(before, member, path)

    walk(old, new, new.path)
    return sorted(found, key=lambda c: (c.path, c.kind, c.detail))


def normalize(text: str) -> list[str]:
    """Lines compared for closeness: trailing space stripped, blank lines dropped."""
    return [s for line in text.splitlines() if (s := line.rstrip())]


def _text_files(tree: Path) -> dict[str, Path]:
    return {
        str(rel): tree / rel
        for rel in relative_files(tree)
        if rel.suffix in TEXT_SUFFIXES and not EXCLUDED_DIRS.intersection(rel.parts)
    }


def closeness(old_tree: Path, new_tree: Path) -> tuple[float, list[FileScore]]:
    """Weighted mean line-similarity over the union of text files, as a percentage."""
    old_files, new_files = _text_files(old_tree), _text_files(new_tree)
    scores: list[FileScore] = []
    for rel in sorted(old_files.keys() | new_files.keys()):
        a = normalize(old_files[rel].read_text(errors="replace")) if rel in old_files else []
        b = normalize(new_files[rel].read_text(errors="replace")) if rel in new_files else []
        weight = max(len(a), len(b))
        if weight:
            ratio = difflib.SequenceMatcher(None, a, b, autojunk=False).ratio()
            scores.append(FileScore(rel, round(ratio, 4), weight))
    total = sum(s.weight for s in scores)
    pct = 100.0 * sum(s.ratio * s.weight for s in scores) / total if total else 100.0
    return pct, scores


def verify(old_tree: Path, new_tree: Path, package: str | None = None) -> Report:
    """Compare two SDK trees; a missing new package counts as empty (all removed)."""
    name = package or detect_package(old_tree)
    old = load(old_tree, name)
    new = (
        load(new_tree, name)
        if (package_root(new_tree, name) / name).is_dir()
        else griffe.Module(name)
    )
    pct, scores = closeness(old_tree, new_tree)
    breaking, same = classify(old, new)
    return Report(name, breaking, additive_changes(old, new), same, pct, scores)


def export_ref(repo: Path, ref: str, dest: Path) -> Path:
    """Write the tree of ``ref`` in ``repo`` into ``dest`` (no checkout, no hooks)."""
    proc = subprocess.run(  # noqa: S603
        ["git", "-C", str(repo), "-c", "core.hooksPath=/dev/null", "archive", "--format=tar", ref],  # noqa: S607
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise fail(
            BAD_REF, f"git archive {ref} failed", "--against", "pass a directory or a valid git ref"
        )
    with tarfile.open(fileobj=io.BytesIO(proc.stdout)) as tar:
        tar.extractall(dest, filter="data")
    return dest
