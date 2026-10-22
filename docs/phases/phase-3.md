# Phase 3 — Python parity

Plan §16 Phase 3: runtime from Apache-2.0 sources with NOTICE; full emitters; README,
api.md, .stats.yml, tests and project files. Gate: OneBusAway regenerates with 0 breaking
changes and its own tests pass.

## Tasks

| # | Task | Owner (files) | Independent |
|---|------|---------------|-------------|
| A | Runtime: the Stainless-generated Python runtime (Apache-2.0, from the pinned OneBusAway SDK) as a package-name-templated tree in `runtime/`, NOTICE + modified-file marks (ADR 0004), fault-injection tests for retries, timeouts, SSE, pagination | runtime agent (`runtime/**`) | yes |
| B | Code emitters: `_client.py` (sync/async, env vars, environments, auth, resources in order), every resource incl. path params, bodies, nested resources, `$client` methods, streaming, all pagination schemes + `pagination.py`, `types/__init__.py`, method order | python-plugin agent (`python-plugin/src/adapter/**`, `model/**`, `emit/{client,resources,models,params,pagination,common,project}.ts`, `type-mapper.ts`, `naming.ts`, `printer.ts`, their tests and goldens) | yes |
| C | Project-file emitters: README.md, api.md, .stats.yml, pyproject.toml, tests/api_resources/*, tests/conftest, scripts/, release-please config, CONTRIBUTING, LICENSE notice; pure functions over SdkModel | python-plugin agent 2 (`python-plugin/src/emit/files/**` new dir + its tests); requests model fields from B via report | yes |
| D | Pipeline + parity harness: `ferrule generate` (convert → forge → plugin → copy runtime → ruff format), `mise run parity -- onebusaway`: griffe API diff vs the oracle (0 breaking), run the oracle's own tests against the output with a Prism mock, normalized text-diff score | migrate agent (`migrate/**`, `scripts/parity/**`) | yes |
| E | Integrate, close gaps until the gate passes | lead | after A–D |

## Exit gate

```bash
mise run check && mise run golden && mise run parity -- onebusaway
```

`parity` must report `breaking: 0` and the oracle test suite passing against generated output.
