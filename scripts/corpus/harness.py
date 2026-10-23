# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Pure helpers for the corpus harness (scripts/corpus/run.py): no file or network access.

Synthesized stainless.yml (deterministic; corpus specs have no config of their own):

* ``organization.name`` is the corpus id; ``targets.python.package_name`` is
  ``corpus_<id>`` with ``-`` as ``_``.
* ``environments.production`` is the spec's first absolute ``http(s)`` server URL (OpenAPI 3
  ``servers`` or Swagger 2 ``schemes``/``host``/``basePath``), else ``https://api.<id>.test``.
* Operations are visited by sorted path, then in the order get, put, post, delete, options,
  head, patch, trace. Each goes to the resource named after its first tag, else its first
  path segment that is not a ``{param}``, else ``root``.
* Method names are the snake-cased ``operationId``, else ``<verb>_<path segments>``. Names
  are made valid identifiers (a leading digit gets an ``r_``/``m_`` prefix, keywords get a
  trailing ``_``) and deduplicated per resource with ``_2``, ``_3``, ... in visit order.
* ``readme.example_requests`` (required by the schema) both point at the first operation.
"""

from __future__ import annotations

import keyword
import re
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, cast

if TYPE_CHECKING:
    from collections.abc import Iterable, Mapping, Sequence

VERBS = ("get", "put", "post", "delete", "options", "head", "patch", "trace")
STAGES = ("fetch", "convert", "forge", "plugin", "runtime", "compile", "wheel")
FER_RE = re.compile(r"\bFER\d{4}\b")
_CAMEL_RE = re.compile(r"(?<=[a-z0-9])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])")
_NON_WORD_RE = re.compile(r"[^0-9a-zA-Z]+")


def snake(text: str) -> str:
    """``listUsers``, ``List-Users`` and ``list users`` all become ``list_users``."""
    spaced = _CAMEL_RE.sub("_", text)
    return _NON_WORD_RE.sub("_", spaced).strip("_").lower()


def identifier(text: str, prefix: str) -> str:
    """Return a valid, non-keyword Python identifier derived from ``text``."""
    name = snake(text) or prefix
    if name[0].isdigit():
        name = f"{prefix}_{name}"
    if keyword.iskeyword(name) or keyword.issoftkeyword(name):
        name = f"{name}_"
    return name


def package_name(spec_id: str) -> str:
    """Return the synthesized SDK's import name."""
    return "corpus_" + identifier(spec_id, "spec")


def _dict(value: object) -> dict[str, object]:
    return cast("dict[str, object]", value) if isinstance(value, dict) else {}


def base_url(spec: Mapping[str, object], spec_id: str) -> str:
    """Return the production URL for the synthesized config."""
    for server in cast("list[object]", spec.get("servers") or []):
        url = _dict(server).get("url")
        if isinstance(url, str) and url.startswith(("https://", "http://")):
            return url
    host = spec.get("host")
    if isinstance(host, str) and host:
        schemes = cast("list[object]", spec.get("schemes") or ["https"])
        scheme = "https" if "https" in schemes or not schemes else str(schemes[0])
        base = spec.get("basePath")
        return f"{scheme}://{host}{base if isinstance(base, str) else ''}"
    return f"https://api.{spec_id}.test"


@dataclass(frozen=True)
class Operation:
    """One HTTP operation of the spec."""

    verb: str
    path: str
    operation_id: str | None
    tag: str | None

    @property
    def endpoint(self) -> str:
        """Stainless ``"<verb> <path>"`` shorthand."""
        return f"{self.verb} {self.path}"


def operations(spec: Mapping[str, object]) -> list[Operation]:
    """Every operation in the deterministic visit order."""
    found: list[Operation] = []
    paths = _dict(spec.get("paths"))
    for path in sorted(paths):
        item = _dict(paths[path])
        for verb in VERBS:
            if verb not in item:
                continue
            op = _dict(item[verb])
            op_id = op.get("operationId")
            tags = cast("list[object]", op.get("tags") or [])
            tag = tags[0] if tags and isinstance(tags[0], str) and snake(tags[0]) else None
            found.append(Operation(verb, path, op_id if isinstance(op_id, str) else None, tag))
    return found


def resource_of(op: Operation) -> str:
    """Return the resource an operation is grouped under."""
    if op.tag is not None:
        return identifier(op.tag, "r")
    for segment in op.path.split("/"):
        if segment and not segment.startswith("{") and snake(segment):
            return identifier(segment, "r")
    return "root"


def method_of(op: Operation) -> str:
    """Return the method name before deduplication."""
    if op.operation_id and snake(op.operation_id):
        return identifier(op.operation_id, "m")
    segments = [s for s in op.path.split("/") if s and not s.startswith("{")]
    return identifier("_".join([op.verb, *segments]), "m")


@dataclass
class Synthesis:
    """A synthesized config and what went into it."""

    config: dict[str, object]
    operations: int
    resources: int
    methods: int
    renamed: list[str] = field(default_factory=list[str])


def synthesize(spec: Mapping[str, object], spec_id: str) -> Synthesis:
    """Build a minimal stainless.yml (as a JSON-ready dict) for ``spec``."""
    ops = operations(spec)
    resources: dict[str, dict[str, str]] = {}
    renamed: list[str] = []
    for op in ops:
        methods = resources.setdefault(resource_of(op), {})
        base = name = method_of(op)
        n = 1
        while name in methods:
            n += 1
            name = f"{base}_{n}"
        if name != base:
            renamed.append(f"{op.endpoint} -> {name}")
        methods[name] = op.endpoint
    first = ops[0].endpoint if ops else "get /"
    example: dict[str, object] = {"type": "request", "endpoint": first, "params": {}}
    config: dict[str, object] = {
        "organization": {"name": spec_id},
        "environments": {"production": base_url(spec, spec_id)},
        "targets": {"python": {"package_name": package_name(spec_id)}},
        "readme": {"example_requests": {"default": example, "headline": dict(example)}},
        "resources": {name: {"methods": methods} for name, methods in sorted(resources.items())},
    }
    return Synthesis(
        config=config,
        operations=len(ops),
        resources=len(resources),
        methods=sum(len(m) for m in resources.values()),
        renamed=renamed,
    )


def fer_codes(texts: Iterable[str]) -> dict[str, int]:
    """Histogram of FER codes mentioned in tool output, sorted by code."""
    counts: dict[str, int] = {}
    for text in texts:
        for code in FER_RE.findall(text):
            counts[code] = counts.get(code, 0) + 1
    return dict(sorted(counts.items()))


def reached(report: Mapping[str, object]) -> str:
    """Return the furthest stage that passed, ``"none"`` if the first one failed."""
    passed = "none"
    stages = _dict(report.get("stages"))
    for stage in STAGES:
        if _dict(stages.get(stage)).get("ok") is not True:
            break
        passed = stage
    return passed


def failed_at(report: Mapping[str, object]) -> str | None:
    """Return the first stage that ran and failed, or ``None``."""
    stages = _dict(report.get("stages"))
    for stage in STAGES:
        if _dict(stages.get(stage)).get("ok") is False:
            return stage
    return None


def summarize(reports: Sequence[Mapping[str, object]], top: int = 10) -> str:
    """Render a Markdown table of per-spec results and the most frequent FER codes."""
    lines = [
        "| spec | reached | failed at | ops | files | FER codes | seconds |",
        "|------|---------|-----------|-----|-------|-----------|---------|",
    ]
    totals: dict[str, int] = {}
    specs_with: dict[str, int] = {}
    compiled = 0
    for report in sorted(reports, key=lambda r: str(r.get("spec"))):
        codes = cast("dict[str, int]", report.get("fer_codes") or {})
        for code, n in codes.items():
            totals[code] = totals.get(code, 0) + n
            specs_with[code] = specs_with.get(code, 0) + 1
        stage = reached(report)
        if stage in {"compile", "wheel"}:
            compiled += 1
        counts = _dict(report.get("counts"))
        shown = ", ".join(f"{c}x{n}" for c, n in codes.items()) or "-"
        lines.append(
            f"| {report.get('spec')} | {stage} | {failed_at(report) or '-'} | "
            f"{counts.get('operations', '-')} | {counts.get('files', '-')} | {shown} | "
            f"{report.get('seconds', '-')} |"
        )
    lines += ["", f"Compile gate: {compiled} of {len(reports)} specs pass.", ""]
    ranked = sorted(totals.items(), key=lambda kv: (-specs_with[kv[0]], -kv[1], kv[0]))[:top]
    if ranked:
        lines += ["| FER code | specs | occurrences |", "|----------|-------|-------------|"]
        lines += [f"| {code} | {specs_with[code]} | {n} |" for code, n in ranked]
    return "\n".join(lines) + "\n"
