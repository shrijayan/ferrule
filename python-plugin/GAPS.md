# Forge gaps found by the Phase 1 plugin spike

Pinned Forge `91f52a40`, interface `forge:plugin@0.3.0` (`.cache/forge/packages/forge/plugin/forge.wit`,
types in `plugin/index.d.ts`). Numbering continues `docs/forge-gaps.md` (G1–G8); the lead merges
these there. "Upstream" marks gaps worth a cloudflare/forge issue.

| #   | Need | Interface today (forge.wit) | Workaround in the spike | Upstream |
| --- | ---- | --------------------------- | ----------------------- | -------- |
| G2 (confirmed) | Package name and repo (README links in `with_raw_response` docstrings) from root `x-ferrule-config` | `record forge-info { version, description, base-url }`; root `x-` keys never reach a plugin (checked: the fixture's root `x-ferrule-config` is invisible) | Operation extension `x-ferrule-package: { name, repo }` mirrored onto every operation; the adapter rejects operations that disagree (FER101). The converter must add `repo` (stainless.yml `targets.python.production_repo`) | Yes: expose document extensions on `forge-info` |
| G3 (confirmed) | Python model names/modules for schemas (`Reference` -> `References` in `types/shared/references.py`) | `record model-info { schema-name, name, group, description, deprecated, ty }`, no extensions | Operation extension `x-ferrule-models: Record<schemaName, { name, module? }>` (module dotted under `types`, default snake_case of name). Only listed schemas become classes of their own; everything else is inlined and named by position, as Stainless does | Yes |
| G9 | Know which extensions the converter wrote vs Forge consumed | `method-info.extensions` drops the `x-fern-*` keys Forge reads (fine), keeps every other `x-` key | None needed; noted because `x-fern-sdk-method-name` spelling is lost (see G4) | No |
| G10 | `generateGroup` needs document-level data (base URL, all models) | `generate-group(group-id)` gets only a group; the group instance can't call `info()` through the TS library (`Group` has no back-pointer to `Forge`) | Named models (shared by groups) and `types/shared/__init__.py` are emitted from `generate`; groups emit resources and their synthesized method types only. `baseUrl` is empty in group models | Maybe: `Group.forge` accessor |
| G11 | Method order in a resource as stainless.yml lists it | `group-info.methods` is "sorted by name" | Spike resources have one method; real ordering needs `x-ferrule-method-order` (or an index in `x-ferrule-method-name`) | Maybe: keep document order |
| G12 | Field order as Stainless prints it | `object-info.fields` in document order (good) | Emitter sorts: required first, `id` first, then wire name case-insensitively (matches every OneBusAway model checked) | No |
| G13 | Distinguish `allOf` of a named model (Python base class) from inline objects | `intersection(list<type-id>)` with `model` members: works | Adapter maps named members to `bases`, object members to fields | No |
| G14 | Union naming for nullable vs real unions | `union-info.exclusive` and a `null` scalar member: works | `T | null` -> `Optional[T]` | No |
| G15 | Formatting and imports exactly as ruff/isort would (line length 120, length-sort, force-wrap) | Sandboxed, no ruff | `PythonPrinter` reproduces the oracle's isort settings and the few ruff wraps the spike needs (long `FieldInfo(...)`, long imports, `__all__`). General formatting stays plan §7 "Finish" (G8) | No |
| G16 | Header line | n/a | Emitter writes `# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.` | No |

## Pagination contract used

`x-ferrule-pagination: { scheme, page, request, response }`, `request`/`response` mapping **spec field
name -> purpose** (converter contract), e.g. `response: { data: "items", has_more: "has_more" }`.
The adapter finds the response field whose purpose is `items`, takes its array element as the page
item, and `page` (`cursor_page` or `CursorPage`) becomes `SyncCursorPage` / `AsyncCursorPage`.

## Not generated yet (FER201 diagnostics, not Forge gaps)

Path parameters (`path_template` + empty-value check), request bodies, streaming, non-object
response aliases, object-typed or aliased query params (`PropertyInfo`), nested resource
subpackages (`resources/<a>/__init__.py`), `types/__init__.py`, resource class names without the
`Resource` suffix (older SDKs such as lithic use `CardPrograms`), the full Stainless docstring
wrapping rules for `Args:` and multi-sentence descriptions.
