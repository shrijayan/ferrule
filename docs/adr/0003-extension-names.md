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

### Consequences

- Good: one place defines the contract between converter and plugin; both sides test against it.
- Good: the annotated spec stays a valid OpenAPI document that any tool can read.
- Bad: changes need coordinated converter and plugin releases; mitigated by `version`.
- Changing or adding an extension amends this ADR.
