# Forge plugin interface gaps

Pinned Forge `91f52a40`, interface `forge:plugin@0.3.0` (`.cache/forge/packages/forge/plugin/forge.wit`).
Each gap: what Ferrule needs, what the interface lacks, and the workaround. "Upstream" marks
gaps worth an issue on cloudflare/forge (queued in [HUMAN-ACTIONS.md](HUMAN-ACTIONS.md); not filed).

| # | Need | Interface today | Workaround | Upstream |
|---|------|-----------------|------------|----------|
| G1 | Group operations by stainless.yml resources | Groups come only from `x-fern-sdk-group-name`; an operation without it makes no group, and `forge.groups` is empty | Converter writes `x-fern-sdk-group-name` / `x-fern-sdk-method-name` (ADR 0003 amendment) | No (works as designed) |
| G2 | Root-level settings (package name, client, environments, auth) | `forge-info` has only `version`, `description`, `base-url`; root `x-` extensions are dropped | **Confirmed by the spike.** Converter mirrors `x-ferrule-package: {name, repo}` onto every operation; the adapter rejects operations that disagree (FER101) | Yes: expose document extensions on `forge-info` |
| G3 | Schema-level names and settings (`x-ferrule-model-name`) | `model-info` and `field-info` carry no extensions; only `x-fern-type-name` / `x-fern-property-name` are read | **Confirmed by the spike.** Converter mirrors `x-ferrule-models: Record<schemaName, {name, module}>` onto every operation; only listed schemas become classes of their own, the rest are inlined and named by position, as Stainless does | Yes: pass schema extensions like method extensions |
| G4 | Exact spelling of names (`$shared`, acronyms, `list` vs `list_`) | `name` is a list of lower-case words | Carry spelling in `x-ferrule-resource` / `x-ferrule-method-name` and use those over `Name` | No |
| G5 | Servers / environments list | Only one `base-url` | `x-ferrule-client.environments` (mirrored per G2) | Yes: expose all servers |
| G6 | Security schemes (api-key header/query, bearer) | Not exposed | `x-ferrule-client.auth` from stainless.yml `client_settings` | Yes |
| G7 | Client-wide files from group data (api.md, `_client.py` resource list) | `generate` sees all groups; `generate-group` runs isolated per group | Emit client-wide files from `generate` only (plan §3 note) | No |
| G8 | Formatting (ruff) and file reads (runtime copy) | Sandboxed: no fs, clock, process | Stage 04 "Finish" in Python after Forge (plan §7) | No |
| G9 | Know which extensions the converter wrote vs Forge consumed | `method-info.extensions` drops the `x-fern-*` keys Forge reads (fine), keeps every other `x-` key | None needed; noted because `x-fern-sdk-method-name` spelling is lost (see G4) | No |
| G10 | `generateGroup` needs document-level data (base URL, all models) | `generate-group(group-id)` gets only a group; the group instance can't call `info()` through the TS library (`Group` has no back-pointer to `Forge`) | Named models (shared by groups) and `types/shared/__init__.py` are emitted from `generate`; groups emit resources and their synthesized method types only. `baseUrl` is empty in group models | Maybe: `Group.forge` accessor |
| G11 | Method order in a resource as stainless.yml lists it | `group-info.methods` is "sorted by name" | Converter writes `x-ferrule-method-order` and `x-ferrule-resource.order` (stainless.yml order); the plugin sorts by them from Phase 3 | Maybe: keep document order |
| G12 | Field order as Stainless prints it | `object-info.fields` in document order (good) | Emitter sorts: required first, `id` first, then wire name case-insensitively (matches every OneBusAway model checked) | No |
| G13 | Distinguish `allOf` of a named model (Python base class) from inline objects | `intersection(list<type-id>)` with `model` members: works | Adapter maps named members to `bases`, object members to fields | No |
| G14 | Union naming for nullable vs real unions | `union-info.exclusive` and a `null` scalar member: works | `T | null` -> `Optional[T]` | No |
| G15 | Formatting and imports exactly as ruff/isort would (line length 120, length-sort, force-wrap) | Sandboxed, no ruff | `PythonPrinter` reproduces the oracle's isort settings and the few ruff wraps the spike needs (long `FieldInfo(...)`, long imports, `__all__`). General formatting stays plan §7 "Finish" (G8) | No |
| G16 | Header line | n/a | Emitter writes `# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.` | No |
| G17 | Union members in document order (`Union[Literal[...], str]`) | `union-info.members` comes back reordered (lumaai `anyOf: [enum, string]` arrives string first) | type-mapper lists literal members first, which matches the oracles checked | Yes: keep document order |
| G18 | Parameter `example`, `minimum`, `maximum` (README and api-resource test examples) | `field-info` has `default` only | `SdkParam.example/minimum/maximum` exist but stay unset; project tests fall back to defaults | Yes: expose them, or mirror via `x-ferrule-*` |
| G19 | One endpoint used by two stainless.yml methods (lumaai `generations.create` and `generations.video.create`) | An operation belongs to one group; the second method is dropped | None yet (lumaai `generation_create_params.py` and `generations.create` missing); needs a converter alias (`x-forge` alias operation) | Maybe: method aliases |
| G20 | Description of an inline object schema (class docstring) | `object-info` has no description; a property's own description stands in | Adapter copies the property description onto its inline object (matches every OneBusAway class) | Yes: describe object types |
| G21 | Multipart / binary request bodies (`FileTypes`) | `content-info.ty` is `unknown` for non-JSON media types | Body typed `unknown`; non-object bodies are FER201 | Yes: types for non-JSON content |
| G22 | Read imagekit's `type: [array, null]` (YAML `null`, not the string `"null"`; 8 schemas) | `SchemaType` is a strict serde enum: `expected value` and the whole document is refused. The spec is invalid JSON Schema, which Stainless tolerates | Converter normalisation `type-null-literal` (FER1601) rewrites the entry to `"null"` and notes `x-ferrule-normalized` on the node | Maybe: accept a `null` literal leniently, or report the JSON pointer |
| G23 | Read knock's `discriminator: __typename` (a bare string, OpenAPI requires a Discriminator Object) | Deserialiser expects an object: `invalid type: string "__typename"`, document refused | Converter normalisation `discriminator-string` (FER1602) rewrites to `{ propertyName }`, noted with `x-ferrule-normalized` | Maybe: accept the string shorthand, or report the JSON pointer |

## Verdict for Phase 1

No gap blocks the plan: every one has an `x-ferrule-*` or Ferrule-side workaround, so **no
upstream patch or temporary fork is needed** (plan §17). G2 and G3 are worth upstream issues
(queued in [HUMAN-ACTIONS.md](HUMAN-ACTIONS.md)); G10 and G11 are worth mentioning in the same
RFC. Re-check this table on every Forge bump.

### Pagination contract used

`x-ferrule-pagination: { scheme, page, request, response }`, `request`/`response` mapping **spec field
name -> purpose** (converter contract), e.g. `response: { data: "items", has_more: "has_more" }`.
The adapter finds the response field whose purpose is `items`, takes its array element as the page
item, and `page` (`cursor_page` or `CursorPage`) becomes `SyncCursorPage` / `AsyncCursorPage`.

### Not generated yet (FER201 diagnostics, not Forge gaps)

Path parameters (`path_template` + empty-value check), request bodies, streaming, non-object
response aliases, object-typed or aliased query params (`PropertyInfo`), nested resource
subpackages (`resources/<a>/__init__.py`), `types/__init__.py`, resource class names without the
`Resource` suffix (older SDKs such as lithic use `CardPrograms`), the full Stainless docstring
wrapping rules for `Args:` and multi-sentence descriptions.
