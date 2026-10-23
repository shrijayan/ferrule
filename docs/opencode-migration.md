# opencode-python-sdk migration

Nothing here was pushed to the SDK repository and no PR was opened. Decision record: ADR 0013.

## Target

| Field | Value |
| --- | --- |
| Repository | <https://github.com/shrijayan/opencode-python-sdk> |
| Release | `v0.2.3`, commit `7bc98edc5566f7a791bbab5cbad7eb4afd786e84` |
| API covered | OpenCode 2.1.2: 142 operations in 31 groups, 412 schema definitions |
| Source contract | `spec/contract.json`, md5 `0e762710699a69eb1ace3cd81c2c8967` |
| Lineage | not Stainless: built by the SDK's own `codegen/` from `contract.json` (no `.stats.yml`) |

The checkout lives in `.cache/opencode-python-sdk` (gitignored). None of its code, scripts or hooks
were run.

## Spec and config provenance

| Artifact | How | Result |
| --- | --- | --- |
| OpenAPI 3.1 | `uv run python -m scripts.opencode.contract_to_openapi .cache/opencode-python-sdk/spec/contract.json .cache/opencode-openapi.json` | 117 paths, 142 operations, 412 schemas; md5 `8eeab36b38b71967913b9587fb2139a3`, byte-identical on re-run |
| `stainless.yml` | `uv run python -m scripts.opencode.api_md_to_config .cache/opencode-python-sdk/api.md oracles/configs/opencode-python-sdk/stainless.yml` | 45 resources, 142 methods; basic auth and `OPENCODE_` settings from `_client.py` |
| Config check | `node converter/src/cli.ts <spec> <config> -o .cache/opencode-annotated.json` | **0 diagnostics** (one FER1111, missing `readme`, fixed before the final run) |

## Generate

`ferrule generate --spec … --config … --out .cache/opencode-gen --runtime-from .cache/opencode-python-sdk`
stops at the first unsupported shape, so the gaps were collected by dropping each failing
operation from a scratch copy of the spec and re-running. Every FER code seen:

| Code | Where | Cause | Owner | Status |
| --- | --- | --- | --- | --- |
| FER4003 / Forge | `GET /api/websearch/provider` (and every op with a `location` query object) | `style: deepObject` query parameter: Forge covers only `form` | Forge (upstream) | worked around in the converter: emits `form` + `explode` |
| FER201 | `GET /api/agent/{agentID}` (4 schemas: `*.Settings`) | `allOf` member of kind map not generated | python-plugin | worked around in the converter: Struct+Record `allOf` folded into `additionalProperties` (equivalent schema) |
| FER201 | `POST /api/experimental/fs/write` | request body that is not an object (binary upload) | python-plugin | **open**; operation dropped for the run (→ `file.write` removed) |
| FER4003 (format) | `types/message_list_response.py`, `types/session/message_get_response.py` | union member that `$ref`s another union emits `Union[, , ]` (`Session.Message.Info` → `Session.Message.Compaction`) | python-plugin | **open**; nested unions inlined in the scratch spec for the run |

With those, generate succeeds: spec md5 `dacc24c6d441a20d6879371819aea77d` (141 operations).
The generated package does **not import**: generated code needs `opencode._utils` (and the
`omit`/`not_given` sentinels), which the SDK's own runtime does not have. `--runtime-from` assumes
a Stainless runtime (owner: migrate, with runtime).

## Verify against v0.2.3

`ferrule verify --against .cache/opencode-python-sdk --new .cache/opencode-gen --json .cache/opencode-verify.json`
exits 1. Closeness **25.75**. **1144 breaking, 610 additive.**

### Breaking, by cause

| Count | Cause | Owner |
| --- | --- | --- |
| 693 | Named schema types removed from `opencode.types` (`AgentInfo`, `SessionInfo`, `WorktreeInfo` …). The SDK exports every `$defs` entry from `types/_generated.py`; Ferrule, like Stainless, emits only types reachable from methods, named after the response (`AgentGetResponse`). Fix is config: `models:` entries per resource | config (this repo's `stainless.yml`), then python-plugin if `models:` naming falls short |
| 67 | Top-level objects removed from `opencode` (incl. `API_VERSION`): per-error exception classes (`SessionNotFoundError` …), RPC client (`RpcClient`, `rpc_method` …), PTY (`PtyConnection`), server/service (`OpencodeServer`, `Endpoint`), event hub, cursor pages | SDK-specific runtime (`_rpc.py`, `_pty.py`, `_server.py`, `_service.py`, `_events.py`); custom code for `merge`, or runtime if generic |
| 206 | Parameter defaults changed (202 on resources, 4 on the client): `NOT_GIVEN` → `omit`/`not_given` (`timeout`, `location`, `limit`, `type`, `cursor` …) | runtime sentinel version; expected for any pre-`omit` SDK. Classify as non-breaking in verify? (migrate) |
| 154 | Kind changed attribute → function: `with_raw_response`, `with_streaming_response` and client resource properties. Both sides are `cached_property`; griffe sees the SDK's `functools.cached_property` as an attribute and Ferrule's `_compat.cached_property` as a function | migrate (verify normalisation) |
| 16 | Client: `events`, `fake`, `from_service`, `wait_until_ready` removed (8, sync + async); constructor `directory`, `transport` removed, `base_url` made keyword, `username` and `timeout` defaults changed, base classes renamed (8) | SDK-specific client methods (merge); `directory`/`transport` have no `client_settings` equivalent (converter/runtime) |
| 6 | Resources: `file.write` removed (FER201 above); `SyncResources`/`AsyncResources` mixins removed; `shell.create(timeout_)` removed (spec has a body field `timeout`, the SDK renamed it to avoid the request option) | python-plugin |
| 2 | `types._generated.PermissionCreateResponse.{id,effect}` | follows from the type layout |

### Additive

586 objects added (per-response types, `AsyncClient` alias, `auth_headers`, `default_headers` …)
and 24 optional parameters added (`_strict_response_validation`, `copy(base_url)` …): Stainless
runtime surface the SDK did not have.

## Files that differ from generator output

The SDK is the output of its own generator, so "hand edits" means drift from `python -m codegen`;
that check needs running SDK code and was not done. Against Ferrule's output, `verify` compared
393 files: 13 identical, 47 partly similar, 333 with no counterpart or nothing in common. The
type layout alone accounts for most: the SDK has 2 files under `types/`, Ferrule 155.

## What remains

1. python-plugin: non-object (binary) request bodies; nested union `$ref` emitting `Union[, , ]`.
2. migrate: `--runtime-from` on a non-Stainless runtime gives an SDK that cannot import; verify
   should normalise `cached_property` kinds and `NOT_GIVEN` → `omit` defaults (360 of 1144).
3. Forge (upstream): `deepObject` query parameters.
4. Config: add `models:` to `oracles/configs/opencode-python-sdk/stainless.yml` to keep the
   693 named types; add a rename for `shell.create(timeout)`.
5. SDK-specific modules (RPC, PTY, server, event hub, typed errors) need custom code kept by
   `ferrule merge` (ADR 0011) or a decision to drop them.
6. Register ADR 0013 in `docs/adr/README.md` (owner: ci-docs).
7. Wire `scripts/opencode/tests` into `pytest` testpaths and the `typecheck:py` mypy list (root
   config, not owned here). Today: `uv run python -m pytest scripts/opencode/tests`.
