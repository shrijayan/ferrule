# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Differential harness: the Rust converter (crates/converter) against the TypeScript one.

Usage: uv run python -m scripts.rs.converter_diff [--only SUBSTRING ...] [--skip-corpus]

Every case runs through TypeScript ``convertText`` (or ``main`` of converter/src/cli.ts) and
through the Rust port (crates/converter/examples/converter_diff.rs); the output bytes and the
diagnostics (code, severity, message, pointer, suggestion, in order) must be equal. Cases:

* every config oracle pair the Phase 2 gate converts (scripts/oracles/converter_oracles.py);
* every Tier 1 corpus spec, as JSON, with the config the corpus harness synthesizes
  (scripts/corpus/harness.py), skipping specs whose cached file is missing;
* scripts/rs/converter_cases.json: the TypeScript tests' text inputs plus edge cases (YAML
  errors and positions, transforms, editions, Swagger 2, normalisation, numbers, key order);
* CLI invocations: usage errors, unreadable files, error diagnostics and a successful write.

The OneBusAway plugin fixture (python-plugin/test/fixtures/onebusaway.json) was written by the
converter and reformatted; it is compared as parsed JSON with the Rust output of that pair.
Spec and config files are data: they are parsed by both converters and never executed.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from corpus.fetch import MANIFEST, cache_path, parse_manifest
from scripts.corpus.harness import synthesize
from scripts.oracles.converter_oracles import CACHE, config_entries, spec_path

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / ".cache" / "converter-diff"
CASES = Path(__file__).resolve().parent / "converter_cases.json"
FIXTURE = ROOT / "python-plugin" / "test" / "fixtures" / "onebusaway.json"

type Json = dict[str, object]

TS_DRIVER = r"""
import { readFileSync, appendFileSync, writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
const [root, casesPath, outPath] = process.argv.slice(1);
const src = (f) => pathToFileURL(`${root}/converter/src/${f}`).href;
const { convertText } = await import(src('index.ts'));
const { main } = await import(src('cli.ts'));
writeFileSync(outPath, '');
for (const c of JSON.parse(readFileSync(casesPath, 'utf8'))) {
  let line;
  if (c.cli) {
    const out = [];
    const err = [];
    const io = { stdout: (l) => out.push(`${l}\n`), stderr: (l) => err.push(`${l}\n`) };
    const code = main(c.cli, io);
    line = { code, stdout: out.join(''), stderr: err.join('') };
  } else {
    try {
      const r = convertText(readFileSync(c.spec, 'utf8'), readFileSync(c.config, 'utf8'));
      const diagnostics = r.diagnostics.map((d) => ({
        code: d.code, severity: d.severity, message: d.message, pointer: d.pointer,
        suggestion: d.suggestion,
      }));
      line = { output: r.output, diagnostics };
    } catch (e) {
      line = { error: e instanceof Error ? e.message : String(e) };
    }
  }
  appendFileSync(outPath, `${JSON.stringify(line)}\n`);
}
"""


@dataclass(frozen=True)
class Case:
    """One input pair, as files."""

    name: str
    spec: Path
    config: Path
    known: str = ""
    """Why the converters are known to differ on this case; empty when they must agree."""


def write(path: Path, text: str) -> Path:
    """Write ``text`` and return the path."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


def inline_cases() -> list[Case]:
    """Return the cases of converter_cases.json, written out as files."""
    data = cast("list[dict[str, str]]", json.loads(CASES.read_text(encoding="utf-8")))
    out: list[Case] = []
    for c in data:
        base = WORK / "inline" / c["name"]
        spec = write(base / "spec", c["spec"])
        config = write(base / "stainless.yml", c["config"])
        out.append(Case(c["name"], spec, config, c.get("known_difference", "")))
    return out


def oracle_cases() -> list[Case]:
    """Return the config oracle pairs of the Phase 2 gate."""
    out: list[Case] = []
    for entry in config_entries():
        name = str(entry["name"])
        config = CACHE / name / str(entry["config_path"])
        spec = spec_path(entry)
        if spec.exists() and config.exists():
            out.append(Case(f"oracle-{name}", spec, config))
    return out


def node(*args: str) -> None:
    """Run node through mise with fixed arguments."""
    subprocess.run(["mise", "exec", "--", "node", *args], check=True, cwd=ROOT)  # noqa: S603, S607


def corpus_cases(skipped: list[str]) -> list[Case]:
    """Tier 1 corpus specs with the synthesized stainless.yml, as scripts/corpus/run.py does."""
    out: list[Case] = []
    manifest = parse_manifest(MANIFEST.read_text(encoding="utf-8"))
    for spec in manifest.tier1:
        raw = cache_path(spec)
        if not raw.exists():
            skipped.append(f"corpus-{spec.name} (not cached)")
            continue
        work = WORK / "corpus" / spec.name
        work.mkdir(parents=True, exist_ok=True)
        as_json = work / "spec.json"
        if spec.format == "json":
            shutil.copyfile(raw, as_json)
        else:
            node(str(ROOT / "scripts" / "corpus" / "to_json.mjs"), str(raw), str(as_json))
        doc = cast("Json", json.loads(as_json.read_text(encoding="utf-8")))
        config = synthesize(doc, spec.name).config
        path = write(work / "stainless.yml", json.dumps(config, indent=2) + "\n")
        out.append(Case(f"corpus-{spec.name}", as_json, path))
    return out


def cli_cases(out: str) -> list[list[str]]:
    """CLI argument vectors; ``out`` is the output path for this driver."""
    inline = WORK / "inline"
    spec = str(inline / "cli-ok" / "spec")
    config = str(inline / "cli-ok" / "stainless.yml")
    bad = str(inline / "cli-missing-endpoint" / "stainless.yml")
    invalid = str(inline / "cli-invalid" / "stainless.yml")
    missing = str(WORK / "missing.yml")
    return [
        [spec, config],
        [spec, "--out", "x"],
        [],
        ["-o"],
        [spec, config, "extra", "-o", out],
        [missing, config, "-o", out],
        [spec, missing, "-o", out],
        [spec, bad, "-o", out],
        [spec, invalid, "-o", out],
        [spec, config, "-o", out],
        [spec, config, "--out", out],
    ]


def lines(path: Path) -> list[Json]:
    """Read a JSON-lines file."""
    text = path.read_text(encoding="utf-8")
    return [cast("Json", json.loads(line)) for line in text.splitlines()]


def run_drivers(ts_cases: list[Json], rs_cases: list[Json]) -> tuple[list[Json], list[Json]]:
    """Run the TypeScript and Rust drivers; return their result lines."""
    ts_list = write(WORK / "cases.ts.json", json.dumps(ts_cases))
    rs_list = write(WORK / "cases.rs.json", json.dumps(rs_cases))
    ts_out, rs_out = WORK / "out.ts.jsonl", WORK / "out.rs.jsonl"
    node("--input-type=module", "-e", TS_DRIVER, "--", str(ROOT), str(ts_list), str(ts_out))
    cargo = ["cargo", "run", "--quiet", "--release", "--locked", "-p", "ferrule-converter"]
    example = ["--example", "converter_diff", "--", str(rs_list), str(rs_out)]
    subprocess.run([*cargo, *example], check=True, cwd=ROOT)  # noqa: S603
    return lines(ts_out), lines(rs_out)


def first_difference(a: str, b: str) -> str:
    """Describe where two texts first differ."""
    for i, (x, y) in enumerate(zip(a.splitlines(), b.splitlines(), strict=False)):
        if x != y:
            return f"line {i + 1}:\n      ts: {x[:200]}\n      rs: {y[:200]}"
    return f"lengths differ: ts {len(a)} bytes, rs {len(b)} bytes"


def compare_diagnostics(name: str, xs: list[object], ys: list[object]) -> list[str]:
    """Report the first differing diagnostic."""
    for i in range(max(len(xs), len(ys))):
        dx = xs[i] if i < len(xs) else None
        dy = ys[i] if i < len(ys) else None
        if dx != dy:
            return [f"{name}: diagnostic {i} differs:\n      ts: {dx}\n      rs: {dy}"]
    return []


def compare(name: str, ts: Json, rs: Json) -> list[str]:
    """Return the differences between two result lines."""
    problems: list[str] = []
    for key in sorted(set(ts) | set(rs)):
        x: object = ts.get(key)
        y: object = rs.get(key)
        if x == y:
            continue
        if isinstance(x, str) and isinstance(y, str) and key in {"output", "stdout", "stderr"}:
            problems.append(f"{name}: {key} differs at {first_difference(x, y)}")
        elif key == "diagnostics" and isinstance(x, list) and isinstance(y, list):
            problems += compare_diagnostics(name, cast("list[object]", x), cast("list[object]", y))
        else:
            shown_x, shown_y = json.dumps(ts.get(key))[:300], json.dumps(rs.get(key))[:300]
            problems.append(f"{name}: {key} differs:\n      ts: {shown_x}\n      rs: {shown_y}")
    return problems


def strip_out_path(result: Json, suffix: str) -> None:
    """Replace a driver's own output path in CLI output, so the two can be compared."""
    for key in ("stdout", "stderr"):
        text = result.get(key)
        if isinstance(text, str):
            result[key] = text.replace(str(WORK / f"cli.{suffix}.json"), "{OUT}")


def check_fixture(cases: list[Case], rs_lines: list[Json]) -> tuple[str, list[str]]:
    """Compare the converter-made plugin fixture with the Rust output of its pair."""
    index = next((i for i, c in enumerate(cases) if c.name == "oracle-onebusaway-config"), None)
    if index is None or not FIXTURE.exists():
        return "skipped", []
    out = rs_lines[index].get("output")
    fixture: object = json.loads(FIXTURE.read_text(encoding="utf-8"))
    if isinstance(out, str) and json.loads(out) == fixture:
        return "equal", []
    return "DIFFERENT", ["fixture: python-plugin/test/fixtures/onebusaway.json differs"]


def select_cases(only: list[str], skip_corpus: bool, skipped: list[str]) -> list[Case]:  # noqa: FBT001
    """Every case, narrowed by ``--only``."""
    cases = oracle_cases() + inline_cases()
    if not skip_corpus:
        cases += corpus_cases(skipped)
    if only:
        cases = [c for c in cases if any(w in c.name for w in only)]
    return cases


def main(argv: list[str] | None = None) -> int:
    """Build the cases, run both converters and report every difference."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", nargs="*", default=[], help="cases whose name contains these")
    parser.add_argument("--skip-corpus", action="store_true", help="leave out the Tier 1 corpus")
    args = parser.parse_args(argv)
    only = cast("list[str]", args.only)

    shutil.rmtree(WORK, ignore_errors=True)
    WORK.mkdir(parents=True)
    skipped: list[str] = []
    cases = select_cases(only, bool(args.skip_corpus), skipped)
    with_cli = not only or any("cli" in w for w in only)
    ts_cli = cli_cases(str(WORK / "cli.ts.json")) if with_cli else []
    rs_cli = cli_cases(str(WORK / "cli.rs.json")) if with_cli else []

    pairs: list[Json] = [{"spec": str(c.spec), "config": str(c.config)} for c in cases]
    ts_lines, rs_lines = run_drivers(
        pairs + [{"cli": a} for a in ts_cli], pairs + [{"cli": a} for a in rs_cli]
    )
    names = [c.name for c in cases] + [f"cli-args-{i}" for i in range(len(ts_cli))]
    known = {c.name: c.known for c in cases if c.known}
    problems: list[str] = []
    for name, ts, rs in zip(names, ts_lines, rs_lines, strict=True):
        if name.startswith("cli-args-"):
            strip_out_path(ts, "ts")
            strip_out_path(rs, "rs")
        found = compare(name, ts, rs)
        if name in known and found:
            sys.stdout.write(f"known {name}: {known[name]}\n")
        else:
            problems += found
    ts_file, rs_file = WORK / "cli.ts.json", WORK / "cli.rs.json"
    if with_cli and (not ts_file.exists() or ts_file.read_bytes() != rs_file.read_bytes()):
        problems.append("cli: the written output files differ")
    fixture, fixture_problems = check_fixture(cases, rs_lines)
    problems += fixture_problems

    for p in problems:
        sys.stdout.write(f"DIFF {p}\n")
    for s in skipped:
        sys.stdout.write(f"skipped {s}\n")
    counts = {
        "oracle pairs": sum(c.name.startswith("oracle-") for c in cases),
        "corpus specs": sum(c.name.startswith("corpus-") for c in cases),
        "inline cases": sum(not c.name.startswith(("oracle-", "corpus-")) for c in cases),
        "cli invocations": len(ts_cli),
        "known differences": len(known),
    }
    summary = ", ".join(f"{v} {k}" for k, v in counts.items())
    sys.stdout.write(f"converter-diff: {summary}; fixture {fixture}; {len(problems)} differences\n")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
