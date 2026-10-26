---
status: Proposed
date: 2026-10-26
deciders: Ferrule maintainers
---

# 0019. Generate Rust SDKs as a Ferrule-original target

Extends [ADR 0017](./0017-multi-language-targets.md).

## Context and problem statement

Every target Ferrule generates so far is one Stainless generates, and each is proven the same
way: byte-identical to an oracle SDK, then a public-API diff and the SDK's own tests. Teams
leaving Stainless also ask for Rust, which Stainless does not offer: its config schema
(`crates/converter/schema/stainless-config.schema.json`) has no `rust` key under `targets`, and
no public Stainless-generated Rust SDK exists. So there is no oracle, and "Stainless output is
the spec" cannot decide anything.

What does a Ferrule-generated Rust SDK look like, how does a config ask for one, and what
replaces "byte-identical to Stainless" as the proof that it is right?

## Decision drivers

- Plan §8 still applies: Forge behind the adapter, one emitter pass with a mode flag, the runtime
  written once, deterministic output, pure convert and emit, FER diagnostics.
- No oracle, so the shape follows the conventions Rust users expect (the Rust API Guidelines,
  `clippy::pedantic`-clean code, `cargo-semver-checks`) and stays close to what our Python SDK
  does at runtime (errors, retries, timeouts), so a team's SDKs behave alike.
- `stainless.yml` must stay valid against the Stainless schema: a team that has not left yet
  must be able to keep using the same file.
- Small dependency footprint, all Apache-2.0 or MIT.

## Considered options

1. A Rust target built on `sdk-model` like the others (a namer, emitters, a runtime template),
   with its own proof in place of an oracle.
2. Wait for Forge's own Rust generator, or an OpenAPI generator such as `progenitor`, and wrap it.
3. No Rust target.

## Decision outcome

Chosen option 1. Option 2 gives an SDK whose surface (names, pagination, errors) has nothing in
common with the team's other SDKs, and nothing in the config (resources, method names,
pagination schemes) would shape it. Option 3 leaves a common request unanswered.

### SDK shape

- **Client**: async, on `reqwest` (rustls) + `serde` + `tokio`. A `blocking` cargo feature adds
  `blocking::<Client>`, which owns a current-thread tokio runtime and exposes the same resources
  with the same request builders, whose `send()` blocks. Async and blocking come from one
  emitter pass: resources and requests are generic over a mode marker (`Async`,
  `Blocking<'_>`), and only `send` differs.
- **Requests**: builder style. A method takes its path params and required params positionally
  and returns a `Request` with one setter per optional param, plus `timeout`, `max_retries` and
  `header` per request. The async `Request` implements `IntoFuture`, so
  `client.agency().retrieve("1").await?` works without `.send()`.
- **Types**: named models are structs and enums in the module of their stainless.yml resource
  (`$shared` → `shared`); inline objects are named by position (`AgencyRetrieveResponseData`),
  as Stainless names them in Python. Optional or nullable fields are `Option<T>`; string enums
  are enums with a `#[serde(other)] Unknown` variant, so a new server value does not break
  deserialization; unions are `#[serde(untagged)]` enums; maps are `BTreeMap`; recursive
  references are boxed; dates and times stay `String` (no `chrono` dependency).
- **Pagination**: all six schemes the converter emits (`cursor`, `cursor_id`, `cursor_url`,
  `offset`, `page_number`, `fake_page`). The runtime implements each scheme once over the raw
  page JSON; the plugin emits one `PageConfig` constant per config scheme (the purposes and
  field paths). `send()` returns the first `Page<T>`; `paginate()` returns a
  `futures::Stream<Item = Result<T>>` (async) or an `Iterator` (blocking) over every item.
- **Streaming**: SSE and JSON Lines, as a `Stream` of typed events (an `Iterator` when
  blocking); an SSE `error` event becomes an error, `[DONE]` ends the stream.
- **Errors**: one `thiserror` enum mirroring the Python runtime's classes: `Connection`,
  `Timeout`, `BadRequest` (400), `Authentication` (401), `PermissionDenied` (403), `NotFound`
  (404), `Conflict` (409), `UnprocessableEntity` (422), `RateLimit` (429), `InternalServer`
  (≥500), `Status` (any other non-2xx), `Decode` (Python's `APIResponseValidationError`) and
  `Config` (a missing API key). Status variants carry the status, headers and body.
- **Retries and timeouts**: the Python runtime's defaults and rules, from the same
  `x-ferrule-client` settings: timeout 60 s and 2 retries unless the config says otherwise;
  exponential backoff from 0.5 s to 8 s with up to 25% jitter; retry on connection errors,
  timeouts, 408, 409, 429 and ≥500, unless the server sends `x-should-retry`; `retry-after-ms`
  and `retry-after` (seconds, at most 60) override the delay.
- **Auth and environments**: client options from `client_settings.opts` (each read from its
  environment variable when not set), sent where the config says; `environments` become an
  `Environment` enum and `base_url` overrides it (`<PREFIX>_BASE_URL` too).

### Naming

- Crate: `targets.rust.crate_name` in `ferrule.yml`, else `organization.name` (`onebusaway-sdk`);
  the library is its snake case (`onebusaway_sdk`).
- Client type: the organization name in Rust's casing (RFC 430: acronyms are words), so
  `onebusaway-sdk` gives `OnebusawaySdk`. `custom_casings` are not applied, as they spell
  acronyms in upper case, which `clippy::upper_case_acronyms` rejects.
- Resources: the stainless.yml key in snake case is the module and the accessor
  (`client.stops_for_location()`, `resources::stops_for_location`); the type is
  `<Key>Resource`. Subresources nest. Methods are the config's method names; Rust keywords get
  a raw identifier (`r#type`) for fields and a trailing underscore for methods and modules.

### Configuration

A `rust` key under `targets:` would fail the Stainless schema, so the Rust target is configured
outside `stainless.yml`, in a Ferrule config file passed to `ferrule-gen generate --target rust
--ferrule-config ferrule.yml`:

```yaml
targets:
  rust:
    crate_name: onebusaway-sdk # default: organization.name
    version: 0.1.0 # default: the spec's info.version
    repository: OneBusAway/rust-sdk # default: none
```

Every key is optional, and without the file the target uses the defaults. The converter and the
Stainless schema are unchanged.

### Proof

There is no oracle to match byte for byte, so the target is proven by:

1. Goldens: the OneBusAway crate (from the pinned oracle spec and config) is committed under
   `crates/rust-plugin/tests/onebusaway/golden` and compared byte for byte, approved by a human
   like every golden.
2. The generated crate's own checks, run by a test of the plugin: `cargo build`, `cargo clippy
   --all-targets --all-features -- -D warnings` and `cargo test`. The runtime's unit tests
   (retries, pagination, SSE) and the generated tests run there; the generated tests call every
   method against the fake server (`testing` feature, the Rust counterpart of
   [ADR 0016](./0016-sdk-testing-fake-server.md)): a local HTTP server that answers each route
   with a JSON body built from its response schema and records the requests.
3. Public API: `cargo-semver-checks` between releases of the generated crate, as the other
   targets use their own languages' checkers.

### Consequences

- Good: Rust users get an SDK that follows the same config as the team's other SDKs, with the
  same retries, errors and pagination.
- Good: the proof is mechanical and runs in CI; the goldens catch any change in output.
- Bad: no external spec decides the shape. Each choice above is ours to maintain, and a change
  to it is a breaking change for users, caught by `cargo-semver-checks`.
- Bad: the plugin's tests need a Rust toolchain that can fetch the crate's dependencies, so
  the build check is skipped offline.
- Bad: optional and nullable are both `Option<T>`, so an explicit `null` and an absent field
  read the same.

## More information

- ADR 0016 (fake server), ADR 0017 (targets and the per-language checker table).
- Research: the "Public-API diff tool per language" table in
  [docs/research/multi-language-targets.md](../research/multi-language-targets.md).
