# opencode-python-sdk migration (draft)

> **Draft.** Re-run after Phase 5. Nothing here was pushed to the SDK repository.

## Target

| Field | Value |
| --- | --- |
| Repository | <https://github.com/shrijayan/opencode-python-sdk> |
| Latest release tag | `v0.2.3` |
| Commit | `7bc98edc5566f7a791bbab5cbad7eb4afd786e84` (2026-10-22) |
| API covered | OpenCode 2.1.2, 142 operations in 31 groups, 412 schema definitions |
| Spec file | `spec/contract.json`, md5 `0e762710699a69eb1ace3cd81c2c8967` |

## Finding: this SDK has no Stainless lineage

The brief assumed a Stainless-generated SDK. It is not one:

- There is no `.stats.yml`, so no `openapi_spec_url`, `openapi_spec_hash` or `config_hash`.
- The SDK is built by its own generator (`codegen/`, run as `python -m codegen`) from
  `spec/contract.json`. That file is not OpenAPI: `codegen/extract` exports it from the Effect
  `HttpApi` definitions in the `@opencode/protocol` npm package (see
  `.github/workflows/sync-spec.yml`).
- The runtime (`src/opencode/_base_client.py`, `_models.py`, …) has the Stainless file layout
  but extra modules (`_rpc.py`, `_pty.py`, `_service.py`, `_server.py`, `_aiohttp.py`, `_unions.py`).

### Spec and config provenance

| Source | Status |
| --- | --- |
| Stainless spec for this SDK | none exists |
| Pinned public pair `opencode-config` (`anomalyco/opencode@33cef07`, 2025-07-31) | targets the 0.x API (26 endpoints, spec md5 `4ff9376c…`); cannot match a 2.1.2 SDK with 142 operations |
| OpenAPI spec for OpenCode 2.x in `anomalyco/opencode` | none committed; the only `openapi.json` is a console route that serves it at runtime |

So no config hash can be matched, and `oracles/configs/opencode-python-sdk/stainless.yml` was
**not** written: there is no OpenAPI document to validate it against with `converter/src/cli.ts`,
and writing one for the 0.x spec would describe a different API.

## Generate and verify

Not run. `ferrule generate` needs an OpenAPI spec; none is available without running third-party
code (the `@opencode/protocol` package or `opencode serve`), which this task forbids.

| Metric | Value |
| --- | --- |
| FER diagnostics | n/a (converter not run) |
| Breaking differences | n/a |
| Hand edits vs. generator output | n/a; the SDK's own `python -m codegen --check` is the equivalent check and was not run (no SDK code executed) |

## What remains

1. **Get an OpenAPI 3 document for OpenCode 2.1.2.** Options, for a human to choose:
   - export it from `@opencode/protocol` with Effect's `HttpApi` OpenAPI generator, in a sandbox;
   - fetch it from a running `opencode serve` (`/doc` or the console `openapi.json` route);
   - add a `contract.json` → OpenAPI step (generic, so upstream-first: discuss before building).
2. Write `oracles/configs/opencode-python-sdk/stainless.yml` from `api.md` (resource tree,
   method names, `client.session.*` etc.), the generated models and `opencode-config`'s client
   settings, as an adopter would (ADR 0009). Validate with the converter, zero error diagnostics.
3. Run `ferrule generate --runtime-from .cache/opencode-python-sdk` and `ferrule verify --against`
   `v0.2.3`. Expect breaking differences from the custom runtime modules above (owner: runtime)
   and from naming the SDK chose itself (owner: python-plugin / converter).
4. Decide whether "Stainless-identical" is the right bar for this SDK at all: it was never
   Stainless output, so parity here means "Ferrule reproduces a hand-built generator", which may
   need its own ADR.
