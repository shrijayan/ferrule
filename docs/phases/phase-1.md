# Phase 1 — Plugin spike and adapter

Plan §16 Phase 1: a minimal Forge Python plugin generating one OneBusAway
resource with pagination via the adapter; list every gap in Forge's plugin
interface in `docs/forge-gaps.md`.

Spike resource: `agencies_with_coverage` (`list`, `paginated: false`; it is
also the README's pagination example). OneBusAway has no paginated endpoint,
so pagination is exercised by a small synthetic cursor-page fixture through
the same adapter path (ADR 0007).

## Tasks

| # | Task | Owner (files) | Independent |
|---|------|---------------|-------------|
| A | Converter spike: stainless.yml `resources` → `x-fern-sdk-group-name`, `x-fern-sdk-method-name`, `x-ferrule-resource`, `x-ferrule-method-name`, `x-ferrule-pagination`; YAML in, JSON out; `ferrule-convert` bin | agent-converter (`converter/`) | yes |
| B | Plugin spike: Forge adapter → SdkModel, resource + types emitters for the spike, WASM build, golden test from an annotated fixture | agent-plugin (`python-plugin/`) | yes (fixture is hand-annotated per ADR 0003) |
| C | Spike pipeline `scripts/spike.sh` (convert → forge → plugin → ruff → pyright vs oracle runtime), `docs/forge-gaps.md`, ADR 0007 | lead (`scripts/`, `docs/`) | yes |

## Exit gate

```bash
mise run check && mise run spike
```

`spike` generates `agencies_with_coverage` from the pinned OneBusAway pair,
places it inside a copy of the oracle package, and runs ruff + pyright strict
on the generated files plus a normalized diff against the oracle files.
