---

status: Accepted
date: 2026-10-22
deciders: Ferrule maintainers
---

# 0003. Extension names: the x-ferrule-* namespace

## Context and problem statement

Forge plugins see only the spec. Every `stainless.yml` setting the Python plugin needs must
travel as an OpenAPI overlay or a vendor extension, which Forge passes through unchanged. We need
stable names and shapes the converter writes and the adapter reads.

## Decision drivers

- No collision with Stainless (`x-stainless-*`), Speakeasy or Forge extensions.
- No vendor names in product identifiers.
- Shapes small, explicit and versionable.

## Decision outcome

All Ferrule extensions use the `x-ferrule-` prefix. The converter is the only writer; the
plugin's `adapter/extensions.ts` is the only reader. Unknown `x-ferrule-*` keys are a FER
diagnostic. Initial set (shapes are TypeScript-like; `?` means optional):

| Extension               | Where     | Shape                                                                                                                                                                                                                                                              |
| ----------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `x-ferrule-config`      | root      | `{ version: 1, edition: string, packageName: string, projectName: string, specHash?: string, configHash?: string }`                                                                                                                                                |
| `x-ferrule-client`      | root      | `{ name: string, asyncName: string, envPrefix: string, defaultTimeoutSeconds: number, maxRetries: number, environments: Record<string, string>, defaultEnvironment?: string, auth: { envVar: string, header: string, scheme?: "bearer" \| "basic" \| "none" }[] }` |
| `x-ferrule-resource`    | operation | `{ path: string[] }`; resource tree position, e.g. `["chat", "completions"]`; `$client` and `$shared` are reserved names                                                                                                                                           |
| `x-ferrule-method-name` | operation | `string`; Python method name, e.g. `"list"`                                                                                                                                                                                                                        |
| `x-ferrule-model-name`  | schema    | `{ name: string, module?: string }`; Python type name and module under `types/`                                                                                                                                                                                    |
| `x-ferrule-pagination`  | operation | `{ scheme: "cursor" \| "cursor_id" \| "cursor_url" \| "offset" \| "page_number" \| "fake_page", page: string, request: Record<string, string>, response: Record<string, string> }`                                                                                 |
| `x-ferrule-streaming`   | operation | `{ format: "sse" \| "jsonl", paramDiscriminator?: string, eventModel?: string }`                                                                                                                                                                                   |

`version` in `x-ferrule-config` is bumped on any incompatible shape change, and the adapter
rejects versions it does not know.


### Amendment 2026-10-23: Forge's own grouping extensions

Forge builds groups, method names and model names only from Fern extensions
(`.cache/forge/src/link/openapi.rs`): an operation without
`x-fern-sdk-group-name` makes no group, and `forge.groups` is empty. So the
converter also writes, on every operation it keeps:

| Extension                | Where     | Value                                                  |
| ------------------------ | --------- | ------------------------------------------------------ |
| `x-fern-sdk-group-name`  | operation | the resource path, e.g. `["agencies_with_coverage"]` |
| `x-fern-sdk-method-name` | operation | the Python method name, e.g. `"list"`                  |
| `x-fern-type-name`       | schema    | the model name, when the config names it               |

`x-ferrule-resource` and `x-ferrule-method-name` stay as the source of truth for
the plugin (Forge splits names into words and loses spelling such as `$shared`);
the Fern ones only drive Forge's grouping.

Forge passes neither document-level nor schema-level extensions to plugins
(docs/forge-gaps.md G2, G3), so the converter also mirrors these onto every
operation it annotates:

| Extension           | Where     | Shape                                                                                   |
| ------------------- | --------- | --------------------------------------------------------------------------------------- |
| `x-ferrule-package` | operation | `{ name: string, repo: string }`: Python package and `targets.python.production_repo`  |
| `x-ferrule-models`  | operation | `Record<schemaName, { name: string, module: string }>`: class name and dotted module under `types/` (`$shared` → `shared.<key>`; a resource's models sit in its parent's directory) |

`x-ferrule-pagination` field maps run spec field name → purpose (purpose from
`x-stainless-pagination-property.purpose`, else the field name); the items field
has purpose `items`.

### Consequences

- Good: one place defines the contract between converter and plugin; both sides test against it.
- Good: the annotated spec stays a valid OpenAPI document that any tool can read.
- Bad: changes need coordinated converter and plugin releases; mitigated by `version`.
- Changing or adding an extension amends this ADR.
