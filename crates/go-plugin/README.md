# ferrule-go-plugin

The Go plugin (ADR 0017): the neutral model of `ferrule-sdk-model` to a Go SDK in Stainless's Go
v1 layout. It never imports Forge; the model comes from `ferrule_sdk_model::forge_to_model`.

```text
annotated spec --sdk-model adapter--> model::Client --for_target("go")--> namer + emitters
  --> generated files + runtime options --ferrule_runtime.render_go_files--> SDK
```

- `naming.rs`: exported names with Stainless's initialisms (`FareURL`, `StopIDs`) and
  `x-ferrule-casings`; services and type prefixes singular per path segment (`TripDetail`).
- `types.rs`: the one type mapper; inline objects and enums are named by position
  (`AgencyGetResponseDataEntry`, array items singular).
- `printer.rs`: gofmt's alignment (a `text/tabwriter` emulation over `\v` cells, `\f` for
  gofmt's formfeeds) and Stainless's 80-column comment wrap. gofmt only verifies the output.
- `emit/`: resource files, `client.go`, `aliases.go`, `shared/shared.go`, `api.md`, tests,
  `internal/version.go`, `go.mod`/`go.sum`/`.keep` files, and the runtime template's slots.
- `meta.rs`: the one accessor for Go settings, `go_target` (`targets.go`, overridden by
  `GoMeta`, which also carries the release version and the README example).

`ferrule-gen generate --target go` writes the files and `.ferrule/go-runtime.json`;
`ferrule generate --target go` renders the runtime from it, removes it and checks `gofmt -l`.

## Proof

`tests/oracle.rs` compares with onebusaway-go (Apache-2.0, `d115209e`, same generation as the
proven Python pin): 65 of 69 generated files are byte-identical after the header branding, and
the runtime slots match the oracle's `requestconfig.go` and `requestoption.go`. The four others
are Forge gap G18 (parameter `example` and `maximum` are not in the model):
`arrivalanddeparture_test.go`, `scheduleforroute_test.go`, `routesforagency_test.go` (path
param examples) and `arrivalsanddeparturesforlocation_test.go` (`maxCount`'s maximum).
`tests/golden.rs` pins a synthetic API that covers bodies, subresources and resource models.

## Deliberate differences

- Header: `// File generated from our OpenAPI spec by Ferrule.` and the `.keep` files' text,
  as for Python (`crates/python-plugin/src/emit/files/BRANDING.md`).

## Not done yet

- Go v2 layout (`enable_v2`), pagination (`packages/pagination`), SSE streaming, unions
  (emitted as `interface{}`), header params, a params struct mixing query and body fields,
  `XParam` variants of models used in requests, non-struct model aliases, string formats
  beyond uuid/date/date-time (`uri`, `email`: not in the model).
- Project files (README, CONTRIBUTING, CI, scripts, `.stats.yml`, release-please).
- The 2025 Stainless generation (`json:"x,required"` tags, knock-go, opencode-go).
- Unverified against an oracle: a method without a response (`Accept` header), bearer and
  header auth options, integer path params.
