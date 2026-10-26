# ferrule-go-plugin

The Go plugin (ADR 0017): the neutral model of `ferrule-sdk-model` to a Go SDK in either of
Stainless's Go layouts. It never imports Forge; the model comes from
`ferrule_sdk_model::forge_to_model`.

```text
annotated spec --sdk-model adapter--> model::Client --for_target("go")--> namer + emitters
  --> generated files + runtime options --ferrule_runtime.render_go_files--> SDK
```

- `naming.rs`: exported names with Stainless's initialisms (`FareURL`, `StopIDs`, `AITags`,
  `OAuth`) and `x-ferrule-casings`; services and type prefixes singular per path segment
  (`TripDetail`, `Branch`).
- `types.rs`: the one type mapper; inline objects and enums are named by position
  (`AgencyGetResponseDataEntry`, array items singular), v2 param variants of models by
  `Position::param_variant` (`DurationParam`).
- `printer.rs`: gofmt's alignment (a `text/tabwriter` emulation over `\v` cells, `\f` for
  gofmt's formfeeds) and Stainless's 80-column comment wrap (Markdown links kept whole, hard
  breaks and lists kept). gofmt only verifies the output.
- `emit/`: resource files, `client.go`, `aliases.go`, `shared/shared.go`, `api.md`, tests,
  `packages/pagination/pagination.go`, `internal/version.go`, `go.mod`/`go.sum`/`.keep`
  files, and the runtime template's slots.
- `meta.rs`: the one accessor for Go settings, `go_target` (`targets.go`, overridden by
  `GoMeta`, which also carries the release version, the README examples and
  `settings.disable_mock_tests`), and the `Layout`.

`ferrule-gen generate --target go` writes the files and `.ferrule/go-runtime.json`;
`ferrule generate --target go` renders the runtime from it, removes it and checks `gofmt -l`.

## Layouts

`targets.go.options.enable_v2` (or `GoMeta::layout`) selects `Layout::V2`; the emitters make
one pass and branch on it. v1 is OneBusAway's (`internal/param`'s `param.Field[T]`, `JSON`
metadata structs, services by pointer). v2 is imagekit-go's and knock-mgmt-go's:

- services and the client by value, `any` for unknown values;
- response structs with an inline `JSON struct { ...respjson.Field }`, enums without
  `IsKnown`, an any-object response as an empty struct;
- params with `param.Opt[T]` for optional and nullable scalars, `omitzero`, `paramObj`,
  `param.MarshalObject`; every path param but the last is a `path:"..."` field of the params
  struct; a query- or path-only argument is `query` for a `GET` and `body` otherwise;
- `<Model>Param` variants of the models params reach, documented with their required
  properties, listed under "Params Types" in `api.md`;
- v2 test literals (`imagekit.String(...)`), `t.Skip("Mock server tests are disabled")`;
- the v2 runtime (`runtime/go-v2/template`: `layout="v2"`, `new_file` when the root package
  declares `File`), value config fields, HTTP Basic auth, `gjson` 1.18.0 in `go.mod`.

The package name loses its dashes (`knock-mapi` is `knockmapi`) and a major version of 2 or
more suffixes the module path (`imagekit-go/v2`).

## Pagination

Cursor schemes of `x-ferrule-pagination` become generic page types in
`packages/pagination/pagination.go` (knock-go's text), paginated methods return
`*pagination.<Page>[<Item>]` and get a `<Method>AutoPaging` twin, and the README's pagination
example gets `paginationauto_test.go` and `paginationmanual_test.go`. Field types come from the
method's response. Other schemes (`cursor_id`, `cursor_url`, `offset`, `page_number`,
`fake_page`) have no Go oracle: their methods return the plain response.

## Proof

| Oracle | Layout | Identical / total generated | Held by |
|---|---|---|---|
| onebusaway-go (Apache-2.0, `d115209e`) | v1 | 65/69 | `tests/oracle.rs` (exact) |
| knock-go (Apache-2.0, `4b86a0d6`, 2025) | v1 | `pagination.go`: 3 of 4 page types | `tests/oracle.rs` |
| imagekit-go (Apache-2.0, `bc22fec7`) | v2 | 15/45 | `tests/oracle_v2.rs` (floor) |
| knock-mgmt-go (Apache-2.0, `85ae774c`) | v2 | 7/50 | `tests/oracle_v2.rs` (floor) |

onebusaway-go is the same generation as the proven Python pin; its four other files are Forge
gap G18 (parameter `example` and `maximum` are not in the model): `arrivalanddeparture_test.go`,
`scheduleforroute_test.go`, `routesforagency_test.go` (path param examples) and
`arrivalsanddeparturesforlocation_test.go` (`maxCount`'s maximum). Its runtime slots match the
oracle's `requestconfig.go` and `requestoption.go`.

knock-go is a 2025 generation (`json:"x,required"` tags), so only its page types are compared;
`MsTeamsPagination` differs because the converter writes `x-ferrule-pagination`'s keys sorted,
not in stainless.yml's order. knock-go and knock-mgmt-go are measured only once the adapter
reads their pagination (the converter's items fix, PR #16, and `$` in request param names);
until then their tests skip.

imagekit-go's `.stats.yml` hashes equal the pinned imagekit-config, so its differences are the
plugin's: unions and constants (`shared/constant`), spec examples in tests (G18), `format:"uri"`
(not in the model), operation servers (`WithBaseURL` on upload), the webhooks resource, and its
hand-edited `client.go`. knock-mgmt-go was generated from a later spec and config than the
pinned knock-mgmt-config, and most of its services carry their OpenAPI tag's description, which
the model does not have.

`tests/golden.rs` pins two small APIs: `synthetic` (bodies, subresources, resource models) and
`paginated` (cursor pagination shaped like knock's `items_cursor`).

## Deliberate differences

- Header: `// File generated from our OpenAPI spec by Ferrule.` and the `.keep` files' text,
  as for Python (`crates/python-plugin/src/emit/files/BRANDING.md`).

## Not done yet

- v2: unions (`XUnion`, `XUnionParam`, `RegisterUnion`), constants (`shared/constant`,
  `constant.X` fields, `NewXParam`), `RegisterFieldValidator` for nested param enums,
  `backcompat.go`, multipart bodies (`MarshalMultipart`), README example params in
  `usage_test.go`.
- Both layouts: SSE streaming, unions in v1 (emitted as `interface{}`), header params,
  non-struct model aliases, string formats beyond uuid/date/date-time (`uri`, `email`: not in
  the model), resource docs from OpenAPI tags and inline response descriptions (not in the
  model), the stainless.yml resource of models whose `x-ferrule-models` module names none.
- Project files (README, CONTRIBUTING, CI, scripts, `.stats.yml`, release-please).
- The 2025 Stainless generation (`json:"x,required"` tags, knock-go, opencode-go).
- Unverified against an oracle: v1 methods without a response (`Accept` header), v1
  pagination after 2025 (v2's `GetNextPage` also returns early on an empty page), bearer and
  header auth options in v1, integer path params, a bearer `scheme_keyword`, a number or
  boolean setting sent in a header or query (`fmt.Sprint`), and `default_headers` (sent after
  the settings' defaults in `DefaultClientOptions`). An API key in a cookie is FER201.
