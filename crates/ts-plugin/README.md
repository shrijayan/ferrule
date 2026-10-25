# ferrule-ts-plugin

Emits the TypeScript SDK Stainless generates, from the language-neutral model
(`crates/sdk-model`, ADR 0017), in either of Stainless's two TypeScript layouts:

- `typescript` target, the current layout (TypeScript 5.8): `src/client.ts`, `src/core/**`,
  `src/internal/**`;
- `node` target, the legacy layout (TypeScript 4.8): the client in `src/index.ts`, the request
  machinery in `src/core.ts`, fetch shims in `src/_shims/**`, `Core.RequestOptions` in resources.

`Layout::of` picks the layout from the targets the config has (`typescript`, else `node`); the
same emitters write both under that mode flag (`Ctx::node`). The crate never names Forge
(`tests/boundary.rs`); `ferrule-gen generate --target typescript` (or `--target node`) maps the
annotated spec to the model and calls `ferrule_ts_plugin::generate_layout`.

```text
annotated spec --sdk-model adapter--> Client --for_target(layout)--> namer + emitters --> files
```

The runtime is not emitted here: `ferrule_runtime.render_typescript_files` renders the current
layout's (`src/core/**`, `src/internal/**`, `src/index.ts` and the `src/*.ts` shims) from
`runtime/typescript/template`, and `render_typescript_node_files` the `node` layout's
(`src/core.ts`, `src/_shims/**`, `src/error.ts`, `src/uploads.ts` and the entry points) from
`runtime/typescript-node/template`, as the Python runtime is rendered from `runtime/template`.

### What the `node` layout changes

| Part | Current layout | `node` layout |
|---|---|---|
| client | `src/client.ts`, standalone class | `src/index.ts`, `extends Core.APIClient`, then the package's exports (`templates/node/index_*`) |
| members | `userID`, `stopIDs` | `userId`, `stopIds` (acronyms capitalized like other words) |
| path parameters | the last positional, the others in params | all positional, a plain template literal |
| optional params | `query: P \| null \| undefined = {}` | overloads with `isRequestOptions` |
| `@example` | every method | only when a value comes from a spec example (a path parameter's) |
| resource tests | required / all params | a method without required params is called bare, then with request options in the params' place |
| project files | `templates/project/` | `templates/node/project/`, plus the ten files both layouts share (`NODE_SHARED`) |

## Layout

| Module | What |
|---|---|
| `meta.rs` | `Layout`, `TsMeta` (what the annotated spec does not carry yet) and `ts_settings`, the one accessor; package name, repository and options come from `client.target("typescript")` or `client.target("node")` |
| `extras.rs` | lookups into the OpenAPI document: examples, tag descriptions, written property order, nullable-variant descriptions |
| `ctx.rs` | the resource tree: classes, properties, files, where each model is declared |
| `naming.rs` | spellings: `PascalCase` types with acronyms (`URLEndpoints`), `camelCase` members, `kebab-case` files |
| `types.rs` | the one type mapping; inline objects become interfaces in their parent's namespace |
| `examples.rs` | example values for `@example`, tests and the README |
| `doc.rs`, `pretty.rs` | prettier's document printer and the JS/TS shapes Stainless emits |
| `comment.rs` | JSDoc: the description as prettier formats markdown at 80 columns |
| `emit/` | resource modules, `client.ts`, `api.md`, `tests/api-resources/**`, project files |

## Formatter decision

Stainless formats TypeScript with prettier (`printWidth: 110`, single quotes, trailing commas,
experimental ternaries). The plugin reproduces prettier's layout in `doc.rs`/`pretty.rs` instead
of running prettier in a finish step, because:

1. The `@example` snippets inside JSDoc are prettier-formatted at width 60 and the descriptions
   are prettier-markdown at width 80 (`proseWrap: always`): text inside comments, which a
   prettier pass over the file never reaches. A port is needed for those regardless.
2. Running another prettier version changes output: prettier 3.9.10 is not idempotent on the
   oracles (5 to 7 files per oracle change: comment placement after `?? X.DEFAULT_TIMEOUT`, a
   `type X = A | B` union layout). The oracles' own lockfiles pin prettier 3.1.1.
3. Emitters stay pure and generation needs no Node toolchain.

Evidence: all 84 emitted files of isaacus-typescript are byte-identical without any formatter
run. README snippets are prettier at width 100 (checked against prettier on the knock-mgmt and
isaacus READMEs). prettier 3.9.10 (a local npx install) was used only during development to
check layouts; it is not a dependency.

## Proof

- `tests/oracle_isaacus.rs`: isaacus-typescript 0.23.0, 84/84 emitted files identical after the
  substitutions of `BRANDING.md`; every other oracle file is a runtime file (33, proven by
  `runtime/tests/test_render_oracles_typescript.py`), `CHANGELOG.md`, the dropped
  `scripts/utils/upload-artifact.sh`, or `packages/mcp-server/**` (not generated yet).
- `tests/oracle_onebusaway.rs`: onebusaway-typescript 1.20.0 (`node` layout), 108/109 emitted
  files identical; `README.md` differs only by OneBusAway's own "Examples and Sample Code"
  section. Every other oracle file is a runtime file (36, proven by
  `runtime/tests/test_render_oracles_typescript_node.py`), `CHANGELOG.md`, the dropped
  `scripts/utils/upload-artifact.sh`, or OneBusAway's hand-written `examples/` and
  `package-lock.json`.
- `tests/oracle_more.rs`: imagekit-typescript (identical/total reported, with a floor);
  knock-mgmt-typescript is skipped while the adapter rejects its pagination (FER101).
- `tests/golden.rs`: a small synthetic SDK (`tests/fixtures/petstore`), byte for byte.

The oracle tests read the config and SDK checkouts at `oracles/cache/<name>`, under their
`oracles/manifest.yaml` names (`isaacus-config`, `isaacus-typescript`; checkouts at the pinned
commits, gitignored), and skip without them.

## Not generated yet

In the `node` layout, lines whose prettier layout depends on name lengths are proven only for
OneBusAway's names (`tests/index.test.ts`, the README snippets). In both layouts: pagination (`src/core/pagination.ts`, `getAPIList` methods, page
aliases); streaming; file uploads (`Uploadable` bodies); unions of objects as params; webhooks
(a resource with models and no methods is not in the model); the MCP server package; client
settings beyond one auth option (several opts, environments, idempotency header, default
headers, query `qs` serialisation in `client.ts`).
