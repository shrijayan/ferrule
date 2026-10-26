# ferrule-csharp-plugin

The C# plugin: the neutral model of `ferrule-sdk-model` to a .NET SDK in Stainless's C# layout
(`src/<Ns>/` with `Services/`, `Models/` and `Core/`, `src/<Ns>.Tests/`, the solution and
project files). It never imports Forge; the model comes from `ferrule_sdk_model::forge_to_model`
(`tests/boundary.rs`). A config with a `csharp` target and no Python one works: the settings come
from `Client::target("csharp")` and the model is read through `for_target("csharp")`.

```text
annotated spec --sdk-model adapter--> model::Client --for_target("csharp")--> namer + emitters
  --> generated files + runtime options --ferrule_runtime (runtime/csharp)--> SDK
  --> csharpier format .
```

- `naming.rs`: PascalCase names (only `AI` and `ID` upper-cased, plus `x-ferrule-casings`),
  locals, singular service prefixes.
- `types.rs`: the one type mapper, from the model's types to C# (`IReadOnlyList<T>` stored as
  `ImmutableArray<T>`, `ApiEnum<TRaw, TEnum>`, `JsonElement`).
- `tree.rs`: the model files and their declarations (classes, enums, unions, params); inline
  types named after their property, the parent's name prefixed on a clash; params and responses
  claim names before named models.
- `doc.rs`: XML doc comments, filled to a width per place.
- `emit/`: services and their raw-response views, models, unions, params, the client and its
  interface, `ClientOptions`, `EnvironmentUrl`, `ModelBase`, the tests (`emit/example.rs` builds
  their values), the project files and the runtime template's options.
- `meta.rs`: the one accessor for C# settings, `csharp_target` (`targets.csharp`, overridden by
  `CSharpMeta`, which also carries the release version, licence, docs URL and copyright year).

The plugin writes the generator's form; csharpier lays it out, as the SDK's `scripts/format`
does. `ferrule-gen generate --target csharp` writes the files and `.ferrule/csharp-runtime.json`;
`ferrule generate --target csharp` renders the runtime from it and runs csharpier on the tree.

## The formatter

`tools/dotnet-tools.json` pins csharpier 1.1.2, the version the oracle uses. Install it once, with
the .NET SDK that `mise.toml` pins:

```bash
mise run csharp:tools      # dotnet tool install into .cache/csharp-tools (gitignored); network once
```

`tools/bin/csharpier` runs it; `$FERRULE_CSHARPIER` overrides it. Without either, the oracle
comparison and the golden test's formatting check skip with a note saying so. To start over,
delete `.cache/csharp-tools` and run the task again.

## Proof

`tests/oracle.rs` compares the formatted SDK with imagekit-dotnet (`265ab207`, Apache-2.0),
proven against imagekit-csharp-config by the `.stats.yml` hashes: 109 of 184 generated files are
byte-identical, and each other file is listed with its reason (doc-comment wrapping Stainless
does not explain by width, webhooks the model lacks, schema examples for the tests, inline
naming and mixed unions, multipart bodies). The generated SDK with the rendered runtime builds
with `dotnet build`.

`tests/golden.rs` checks a small synthetic API with a C# target only against
`tests/synthetic/golden`, byte for byte; csharpier accepts it and is settled after one pass, and
the SDK builds and its tests pass against a mock server. Golden changes are reviewed by a human.
