# ferrule-rust-plugin

The Rust plugin (ADR 0019): the neutral model of `ferrule-sdk-model` to a Rust SDK crate.
Rust is Ferrule's first target that Stainless does not have, so there is no oracle; the
goldens and the crate's own checks are the proof. It never imports Forge; the model comes from
`ferrule_sdk_model::forge_to_model`.

```text
annotated spec --sdk-model adapter--> model::Client --for_target("rust")--> namer + emitters
  --> generated files + runtime/rust/template --rustfmt--> crate
```

- `naming.rs`: `snake_case`, `UpperCamelCase` with acronyms as words (RFC 430; `custom_casings`
  are not applied), raw identifiers or a trailing `_` for keywords, unique names per scope.
- `types.rs`: the one type mapper. Named models are declared once in their resource's module
  (`$shared` in `shared`); inline objects, enums and unions are named by position
  (`AgencyRetrieveResponseData`). Each rendering also gives an example value for the tests.
- `emit/method.rs`: a method's params struct, setters, `Endpoint` impl, the call that starts
  it, its fake-server route and its test; a streaming discriminator gives `<name>` and
  `<name>_streaming` from one pass.
- `emit/resource.rs`, `emit/client.rs`: resource structs generic over the mode (`Async`, or
  `Blocking` with the `blocking` feature), the client, its builder and the blocking client,
  whose accessors come from the same pass with the other mode.
- `emit/project.rs`: `Cargo.toml`, `README.md` (the crate docs; its example is a doctest),
  `src/lib.rs`, the fake server's routes and `tests/api.rs`.
- `fake.rs`: the fake server's reply bodies, the smallest JSON value each response type reads.
- `meta.rs`: `targets.rust` of a `ferrule.yml` (`crate_name`, `version`, `repository`,
  `license`), as stainless.yml has no Rust target.
- `runtime.rs`: `runtime/rust/template`, embedded.

`ferrule-gen generate --target rust [--ferrule-config ferrule.yml]` writes the whole crate and
lays it out with rustfmt.

Tests: `golden.rs` (OneBusAway from the pinned oracle inputs, and a synthetic API with every
pagination scheme and streaming), `crate_checks.rs` (each golden crate passes `cargo fmt
--check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test
--all-features` and `cargo semver-checks`), `runtime.rs` and `boundary.rs`.
