---
status: Proposed
date: 2026-10-26
deciders: Ferrule maintainers
---

# 0017. Generate every major SDK language, from one language-neutral model

Amends [ADR 0001](./0001-languages.md) and [ADR 0002](./0002-adapter-model.md).

## Context and problem statement

Ferrule regenerates the Python SDK a team got from Stainless. A team leaving Stainless usually
ships several SDKs from the same `stainless.yml`: its `targets:` has 13 keys (node, typescript,
python, go, java, kotlin, ruby, php, csharp, terraform, cli, openapi, sql). A GitHub search finds
635 public repos with a Stainless `.stats.yml`: TypeScript 224, Python 176, Go 119, Java/Kotlin
33, Ruby 33, PHP 19, C# 15. Python alone covers part of a migration.

The evidence is in [docs/research/multi-language-targets.md](../research/multi-language-targets.md):
31 public Apache-2.0 or MIT SDK repos in other languages pair with our config oracles by
`.stats.yml` hashes. onebusaway pairs in TypeScript, Go, Java, Kotlin and Ruby at the same
generation as its proven Python pin.

How do we add languages without forking the pipeline per language, and without putting the
byte-identical Python output at risk?

## Decision drivers

- Stainless output is the spec, per language: each language is proven against its own oracles,
  as Python was (goldens, public-API diff, the SDK's own tests).
- Forge's model is already language-neutral (names are word lists with every casing, scalars
  keep sizes); Ferrule's `SdkModel` is not (Python spellings, `py_name`, module paths).
- Forge stays behind one adapter (ADR 0002); emitters see Ferrule's model only.
- DRY across languages: one adapter, one model, one config reading; per-language code is naming,
  emitting and printing.
- Python's goldens and parity must not move while other languages are added.

## Considered options

1. One plugin per language, each with its own adapter and model.
2. A language-neutral model and adapter in a shared crate; one plugin crate per language.
3. Forge's own TypeScript generator (`src/bin/forge/ts.rs`) for TypeScript.

## Decision outcome

Chosen option 2.

- `crates/sdk-model` holds the language-neutral model and the only adapter from Forge. The model
  keeps word lists and wire names (never a language spelling), stainless.yml spelling overrides
  (`x-ferrule-casings`), stable ids for named types, Forge's full scalar set, nullable apart from
  optional, named union variants and enum members, and the client, pagination and streaming
  settings.
- One plugin crate per language (`crates/python-plugin`, `crates/ts-plugin`, `crates/go-plugin`,
  `crates/jvm-plugin` for Java and Kotlin, `crates/ruby-plugin`, `crates/rust-plugin`, …): a namer, emitters with a
  mode flag per variant (sync/async, Java/Kotlin, TypeScript layouts), and a printer.
- `ferrule-gen --target <lang>` selects the plugin; `ferrule-gen model` prints the neutral model.
  The converter honours each target's edition and the `skip`/`only` lists per target.
- Runtimes: `runtime/<lang>/`, a tokenised template rendered byte-identical for every oracle of
  that language, copied only from Apache-2.0 Stainless SDKs and credited in NOTICE (ADR 0004).
  MIT oracles (ogx, opencode, stagehand) are used as test oracles, not copied, unless a later ADR
  adds them to the plan's reuse list.
- Oracles: `oracles/manifest.yaml` gains a `language` (and `layout`) per SDK entry; pairing is
  proven by `.stats.yml` hashes as for Python (ADR 0009).
- verify and adopt generalize per language: the same spec rescue and hand-edit handling, the
  SDK's own `scripts/test` against a mock server, and a public-API diff run with each language's
  own checker, never a port of it: griffe (Python), api-extractor or a declaration
  assignability check (TypeScript), `apidiff`/`gorelease` (Go), japicmp (Java), Kotlin's
  binary-compatibility validator (Kotlin), an RBS signature diff (Ruby),
  roave/backward-compatibility-check (PHP), ApiCompat package validation (C#),
  `cargo-semver-checks` (Rust), the command and flag tree (CLI) and `terraform providers schema -json` (Terraform). The table in the research
  doc is the source of truth. merge keeps libcst for Python and falls back to a three-way text
  merge elsewhere.

Order, by oracle availability and demand:

1. TypeScript, current `typescript` layout (isaacus, then knock and imagekit).
2. TypeScript, older `node` layout (onebusaway, lumaai, honcho).
3. Go (v1 onebusaway, knock; v2 imagekit); also the base for CLI and Terraform.
4. Java and Kotlin, one JVM plugin (onebusaway, imagekit, knock).
5. Ruby (onebusaway, knock, imagekit).
6. PHP and C#.
7. CLI and Terraform.
8. Rust, the first Ferrule-original target: Stainless has no Rust target, so there is no oracle;
   [ADR 0019](./0019-rust-sdk-ferrule-original-target.md) records its shape and its proof.

Python moves onto `sdk-model` as a separate step, after the Phase 4 changes to its adapter land.
Until then Python keeps its own adapter: a deliberate, temporary duplication. The exit criterion
is the Python plugin reading only `sdk-model` with every Python golden and the OneBusAway parity
unchanged; `crates/sdk-model/MIGRATION.md` maps each Python field to its neutral source.

Forge's own TypeScript generator (option 3) writes a different SDK shape, not Stainless's, so it
can't be the spec. Its `forge::ts` import-managing printer may be reused if the header and line
width become configurable upstream (Stainless formats TypeScript with prettier at 110 columns).

### Consequences

- Good: one model and one adapter serve every language; a new language is a namer, emitters, a
  runtime template and oracles.
- Good: the same proof (oracles, goldens, API diff, the SDK's own tests) applies to every
  language, so "nothing public changed" holds beyond Python.
- Bad: much more surface. Each language adds a toolchain (Node, Go, JDK, Ruby) to CI, run inside
  the SDK's devcontainer or pinned by mise.
- Bad: the temporary second adapter (Python's) until the migration step lands.
- Bad: the API-diff tools differ per language and some (Ruby) have no standard tool.

## More information

- Research: [docs/research/multi-language-targets.md](../research/multi-language-targets.md).
- ADR 0001 (languages), ADR 0002 (adapter model), ADR 0004 (runtime source), ADR 0009 (oracle
  configs), ADR 0014 (Rust).
