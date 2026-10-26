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

### Amendment 2026-10-23 (Phase 2): client, streaming, pagination, order, `$client`

This amendment replaces the `x-ferrule-client`, `x-ferrule-pagination` and
`x-ferrule-streaming` rows of the initial table (nothing read them yet, so `version` stays 1;
an incompatible change to a shape a released plugin reads still bumps it). Code lives in
`converter/src/extensions/`. Objects whose order matters are arrays, because the annotated spec
is written with sorted keys.

**`x-ferrule-config`** (root) gains `pythonEdition`. `edition` and `pythonEdition` are the
editions whose defaults were applied: a config without `edition` gets the first ones,
`2025-10-08` and `python.2025-10-08` (OneBusAway; its SDK ships Rye files).

**`x-ferrule-client`** (root, and mirrored onto every annotated operation per gap G2). Written
only when `targets.python.package_name` is set, like `x-ferrule-config`:

```ts
{
  name: string,        // client_settings.default_client_name ?? organization.name (as written)
  envPrefix: string,   // client_settings.default_env_prefix ?? UPPER_SNAKE(organization.name)
  opts: {              // client_settings.opts, config order
    name: string, type: "string" | "number" | "integer" | "boolean",
    nullable: boolean, default?: unknown, readEnv?: string, description?: string,
    auth?: { securityScheme: string, role: "value" | "username" | "password" | "client_id" | "client_secret" },
    send?: { in: "header" | "query" | "body" | "path", name: string },
    serverVariable?: string,
  }[],
  environments: { name: string, url: string }[],  // config order; else the spec's first server as "production"
  defaultEnvironment?: string,                     // the first environment
  securitySchemes: Record<string,                  // spec components.securitySchemes, overlaid by config security_schemes
    | { type: "http", scheme: "bearer" | "digest", keyword?: string }
    | { type: "http", scheme: "basic", keyword?: string, disableBase64: boolean }
    | { type: "apiKey", in: "header" | "query" | "cookie", name: string }
    | { type: "oauth2", flows: object }
    | { type: "openIdConnect", url: string }>,
  security: Record<string, string[]>[],            // config security ?? spec security
  timeoutSeconds: number,                          // default_timeout (ms or ISO 8601); default 60
  retries: { maxRetries: number, initialDelaySeconds: number, maxDelaySeconds: number }, // default 2, 0.5, 8
  idempotencyHeader?: string,                      // client_settings.idempotency.header
  defaultHeaders: Record<string, string>,
  omitPlatformHeaders: boolean,                    // client_settings.omit_stainless_headers
  query: { nestedFormat: "brackets" | "dots", arrayFormat: "comma" | "repeat" | "indices" | "brackets" },
  httpLimits?: { maxConnections?: number, maxKeepaliveConnections?: number, keepaliveExpiry?: number },
}
```

**`x-ferrule-streaming`** (operation), from a method's `streaming:` block unless its `skip`
lists `python`:

```ts
{
  format: "sse" | "jsonl",           // streaming.type, else from the 2xx content type, else "sse"
  paramDiscriminator: string | null, // null: the method always streams
  eventModel?: string,               // stream_event_model as written ("$resource.$model")
  eventSchema?: string,              // the components.schemas name it resolves to
  paramsTypeName?: string, methodName?: string,
  synthesizeEventAndData: boolean,
  onEvent: { kind: "event" | "fallthrough" | "data", handle: "break" | "continue" | "done" | "yield" | "error",
             eventType?: string | string[] | null, errorProperty?: string, dataStartsWith?: string }[],
}                                    // onEvent: top-level streaming.on_event, config order
```

**`x-ferrule-pagination`** (operation):

```ts
{ scheme: "cursor" | "cursor_id" | "cursor_url" | "fake_page" | "offset" | "page_number",
  page: string, paramLocation: "query" | "body", continueOnEmptyItems: boolean,
  request: Record<string, string>, response: Record<string, string>,
  headers?: Record<string, string> }  // header name -> purpose, for from_header properties
```

Field maps run spec field path → purpose. A field with nested annotated properties is listed by
dotted path (`meta.next_cursor`), and `.[].` steps into array items (`data.[].id` for
`cursor_item_id`); un-annotated leaves keep their name as purpose. The single-object
`pagination:` form is named `page`. Matching (methods named `list` / `list_*`, or
`paginated: true`): every request field is a param in `param_location`, every response path
exists in the 2xx JSON response, and the `items` field is an array; of several matches the
one with the most fields wins, then config order. `paginated: <name>` and `false` behave as
before.

**Order (gap G11).** `x-ferrule-resource` gains `order: number[]`, the index of each path
segment among its siblings in stainless.yml (`resources`, then `subresources`), and each
operation gets `x-ferrule-method-order: number`, the method's index in its resource's
`methods`. The plugin sorts client attributes, sub-resources and methods by these.

**`$client` methods.** Forge drops an operation whose `x-fern-sdk-group-name` is empty, and
turns `["$client"]` into the group `client`, merging it with a real `client` resource (checked
against the pinned Forge CLI). So `$client` methods get
`x-fern-sdk-group-name: ["$ferrule_client"]` (Forge words `ferrule client`); a top-level
resource whose words are the same is FER1006. `x-ferrule-resource.path` stays `["$client"]`,
and the plugin emits those methods on the client class.

**`x-fern-type-name`** is now written on every `components.schemas` entry a `models:` key
names (the Python class name, e.g. `References`); a model naming a missing schema is FER1005.
Forge keeps `model-info.schema-name` as the component key, so `x-ferrule-models` lookups are
unchanged.

**Diagnostic ranges.** FER10xx annotate, FER11xx config validation and editions, FER12xx
`openapi.transforms`, FER13xx client settings, FER14xx streaming, FER15xx pagination.

### Amendment 2026-10-25 (Phase 4): config spelling in `repo` and `module`

No shape changes; this pins how the plugin reads two values the converter copies from
stainless.yml as written (as it already does for `x-ferrule-resource.path`):

- `x-ferrule-package.repo` is `targets.python.production_repo` verbatim, which may end in
  `#<branch>` (imagekit `imagekit-developer/imagekit-python#master`). The plugin links
  `owner/name`, as Stainless does; the raw value stays in the spec for files that print it.
- `x-ferrule-models[*].module` segments are the stainless.yml resource keys and model key as
  written (opencode `assistantMessage`, imagekit `accounts.UrlEndpointResponse`). The plugin
  `snake_cases` every segment that is not already a Python module name, with the function it
  uses for resource keys, giving Stainless's `types/assistant_message.py` and
  `types/accounts/url_endpoint_response.py`. Snake-case segments are kept byte for byte.

### Amendment 2026-10-25 (Phase 4): one endpoint, several methods

stainless.yml may bind one endpoint to several methods (lumaai `generations.create` and
`generations.video.create` are both `post /generations/video`; ogx `models.list` and
`models.openai.list` are both `get /v1/models`), and Stainless emits each. Forge makes one
method per operation unless the operation has `x-forge-aliases` (documented on
`forge::Method` at the pinned commit): one object per method, each with its own group, name
and extensions; the HTTP method, path, parameters, body and responses stay the operation's.

So an operation bound to more than one method gets, in place of the per-method keys on the
operation itself:

| Extension         | Where     | Value                                                                                         |
| ----------------- | --------- | --------------------------------------------------------------------------------------------- |
| `x-forge-aliases` | operation | one object per method, in config order, holding every key that method would have had on the operation: `x-fern-sdk-group-name`, `x-fern-sdk-method-name`, `x-ferrule-resource`, `x-ferrule-method-name`, `x-ferrule-method-order`, `x-ferrule-pagination`, `x-ferrule-streaming` and the mirrored `x-ferrule-models`, `x-ferrule-package`, `x-ferrule-client` |

An alias does not inherit the operation's extensions, so each carries all of its own. An
operation bound once is written exactly as before. The plugin needs no change: Forge hands it
one method per alias, with that alias's extensions.

### Amendment 2026-10-25 (Phase 4): custom casings and standalone APIs

stainless.yml `custom_casings` changes how words are spelled in class names (isaacus `qa:
{ initialism: true }` gives `QAResource`; lumaai's pascal `LumaAI` names the client and
`LumaAIError`), and `standalone_api: true` puts a resource in its own package with its own
`api.md` (isaacus `extractions.qa`, `ilgs.v1`). Both reach the plugin as:

| Extension              | Where                         | Value                                                                                   |
| ---------------------- | ----------------------------- | --------------------------------------------------------------------------------------- |
| `x-ferrule-casings`    | root, mirrored on every method | `Record<word, { initialism: boolean } \| { pascal: string }>`, in config order          |
| `x-ferrule-standalone` | root, mirrored on every method | `{ path: string[], order: number[] }[]`, one per `standalone_api: true` resource, in config order |

Each is written only when the config has some, so specs without them annotate exactly as
before. Like `x-ferrule-client`, both are mirrored onto every method (and into every
`x-forge-aliases` entry) because Forge drops root extensions (gap G2). Matching is per word
and case-insensitive; `initialism: true` keeps a plural `s` (`QAs`). Model names from `models:`
keys use Stainless's default initialisms plus these casings (knock-mgmt `SMSTemplate`).
Entries Ferrule cannot apply are FER1701-FER1703 warnings and left out.

### Amendment 2026-10-26 (Phase 4): pagination maps keep config order

The annotated spec is still written with sorted keys, except for three objects:
`x-ferrule-pagination`'s `request`, `response` and `headers` maps, including those inside
`x-forge-aliases`. These keep the order stainless.yml lists them in, because Stainless declares a
page's fields in that order (knock-go `MsTeamsPagination` has `SkipToken`, then
`MsTeamsTeams`; honcho-python's `SyncPage` lists `total`, `items`, `page`, `size`, `pages`).
Insertion order is deterministic, so the output still is. The list lives in
`CONFIG_ORDER_MAPS` (`crates/converter/src/extensions/pagination.rs`); a map joins it only when
an oracle shows Stainless follows config order there. The plugins already read these maps in
written order, and `version` stays 1, since the shape is unchanged.

### Amendment 2026-10-26 (ADR 0017): every target, and what each item is generated for

Plugins for languages other than Python need their own target's settings and stainless.yml's
per-language `skip:`/`only:` lists, which the converter used to read for Python only. Two
extensions carry them:

| Extension                | Where                                               | Value |
| ------------------------ | --------------------------------------------------- | ----- |
| `x-ferrule-targets`      | root, mirrored on every method                      | `Record<target, { package_name?: string, edition?: string, production_repo?: string, options: object }>` |
| `x-ferrule-targets-only` | method; each `x-ferrule-models` entry               | `string[]`: the target keys the item is generated for, in config order |

**`x-ferrule-targets`** has a member for every key under stainless.yml `targets` (node,
typescript, python, go, java, kotlin, ruby, php, csharp, terraform, cli, openapi, sql, and keys
the schema does not know, kept as written). The annotated spec is written with sorted keys, so
readers see the targets in key order, not config order.

- `edition` is the edition whose defaults apply: the one written if Ferrule knows it, else the
  target's first ("the very first edition" in the public schema), the way `x-ferrule-config`
  reports Python's. An edition Ferrule does not know is a FER1103 warning (Python's stays
  FER1102). For an unknown key it is the edition as written. Python's `options.use_uv`
  default is the only edition default with a config key; it is filled in.
- `package_name` and `production_repo` are copied when they are strings (stagehand writes
  `production_repo: null`, which is left out); `production_repo` may end in `#<branch>`.
- `options` is every other key of the target as written (its own `options:` among them, so
  imagekit's `typescript.options.mcp_server` is `options.options.mcp_server`), minus `publish`.

It is written only when `targets` has a key other than `python` or a `skip:`/`only:` list
restricts something, so a Python-only config without lists is annotated exactly as before. Like
`x-ferrule-client`, it is mirrored onto every method and every `x-forge-aliases` entry (gap G2).

**`x-ferrule-targets-only`** is written only when a list restricts the item. Lists apply down
the tree: a resource's restriction is its parent's, then its own `only:` (keep the listed
targets) and `skip:` (drop them; `skip: true` drops all); a method's and a model's start from
their resource's. The targets are the config's `targets` keys (every target the schema knows,
for a config fragment without `targets`).

`x-ferrule-models` now lists every `models:` entry some target generates, not only Python's:
an entry no target generates is left out, an entry with an `openapi_uri` Ferrule cannot read is
FER1004 whichever target generates it, and an entry leaving Python out never replaces a Python
entry for the same schema (the map has one entry per schema). `x-fern-type-name` is written for
every listed entry.

The Python plugin accepts both extensions and leaves out the `x-ferrule-models` entries whose
`x-ferrule-targets-only` lacks `python`, inlining those schemas as before; it does not yet drop
methods that leave Python out (no oracle has one). `crates/sdk-model` reads them as
`Client.targets`, `Method.only` and `ModelDecl.only`; `Client::for_target` gives one target's
view.

### Consequences

- Good: one place defines the contract between converter and plugin; both sides test against it.
- Good: the annotated spec stays a valid OpenAPI document that any tool can read.
- Bad: changes need coordinated converter and plugin releases; mitigated by `version`.
- Changing or adding an extension amends this ADR.
