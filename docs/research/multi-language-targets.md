# Ferrule multi-language research

Date: 2026-10-26. Read-only research. Sources: `crates/converter/schema/stainless-config.schema.json`
(the public Stainless config schema), `oracles/manifest.yaml` and the pinned configs in
`oracles/cache/*-config`, blobless clones of 31 public SDK repos (history of `.stats.yml` read with
git), `gh api` for licences, a GitHub code search for `.stats.yml`, Forge at `.cache/forge`
(commit `91f52a40`), and `crates/python-plugin/src/model.rs`.

No `stlc` source was read. Everything below comes from public generated SDK repos, the public
config schema and Forge's Apache-2.0 source.

---

## 1. Target languages in the stainless.yml schema

The schema's `targets` object has 13 keys. Per-resource and per-method `skip`/`only` lists accept
the same names plus `http`: `node, typescript, python, go, java, kotlin, ruby, terraform, cli,
php, csharp, sql, http, openapi`.

| target | edition values (newest first) | target keys besides the shared ones¹ | `options` | `publish` |
|---|---|---|---|---|
| `node` (old TS) | `node.2025-10-08` | `package_name` | `mcp_server`, `browser` | npm, jsr, github, release |
| `typescript` | `typescript.2025-10-10`, `typescript.2025-10-08` | `package_name` | `mcp_server`, `browser`, `node_migration`, `enable_treeshaking`, `package_manager` | npm, jsr, github, release |
| `python` | `python.2025-11-20`, `python.2025-10-08` | `package_name`, `project_name` | `use_uv`, `use_eager_iterables`, `python_version_range`, `websockets_extra_name`, `standardwebhooks_extra_name` | pypi, github, release |
| `go` | `go.2026-04-15`, `go.2026-03-10`, `go.2026-2-3`, `go.2025-10-08` | `package_name` | `go_module_path_override`, `option_package_name`, `client_name`, `go_language_version`, `code_style`, `enable_v2`, `union_property_getters`, `subtle_union_naming`, `remove_params_suffix`, `path_param_encoding`, `escape_html_in_json`, `nullable_map_items`, `back_compat_options_field_name` | github, release |
| `java` | `java.2025-10-08` | `reverse_domain` | `gradle_properties` | maven, docs, proguard, graalvm_metadata, spring_boot_starter, github, release |
| `kotlin` | `kotlin.2025-10-08` | `reverse_domain` | `gradle_properties` | same as java |
| `ruby` | `ruby.2025-10-08` | `gem_name`, `package_name` | `client_name` | rubygems, github, release |
| `php` | `php.2025-10-08` | `package_name`, `composer_package_name`, `composer_repository_url` | none | packagist, github, release |
| `csharp` | `csharp.2025-10-08` | `package_name`, `strong_name` | `language_version`, `dotnetStandardTests` | nuget, github, release |
| `terraform` | `terraform.2025-10-08` | `provider_name` | `go_module_path_override`, `go_sdk_package`, `go_sdk_version`, `go_language_version`, `infer_all_services`, `generate_docs`, `timeouts`, `data_source` | hashicorp_registry, github, release |
| `cli` | `cli.2025-10-08` | `binary_name`, `default_format`, `default_json_get_format`, `default_error_format` | `go_module_path_override`, `go_sdk_package`, `go_sdk_version`, `go_language_version` | release_environment, macos, homebrew, github, release |
| `openapi` | `openapi.2025-10-08` | none | `files` | none |
| `sql` | `sql.2025-12-08` | `extension_name` | none | pgxn, github, release |

¹ Every target also has `readme_title`, `production_repo` (`owner/repo[#branch]`), `screencast`,
`skip`, `keep_files` and `edition`.

The top-level `edition` takes `2026-05-06` ("switches the default normalizer to v2"),
`2026-02-23`, `2026-01-30`, `2025-10-10` and `2025-10-08` ("the very first edition").

Other places that name a language:
- `readme.example_requests.<lang>`, for each SDK language.
- `streaming.method_name.{go,ruby}` on a method.
- `ResourceConfig.terraform` and method-level `terraform`, `cli` and `mcp` blocks.

**Major languages.** "Major" here means general-purpose SDK languages with real usage. These are
TypeScript (in two flavours, `node` and `typescript`), Python, Go, Java, Kotlin, Ruby, PHP and C#.
Terraform and CLI are Go programs built on top of the generated Go SDK: their options include
`go_sdk_package` and `go_sdk_version`. `openapi`, `sql` and `http` are not SDK languages.

**Demand.** A GitHub code search for `openapi_spec_hash` in `.stats.yml` finds 635 repos. Their
primary languages, forks excluded:

| primary language | repos | notes |
|---|---|---|
| TypeScript | 224 | |
| Python | 176 | |
| Go | 119 | about 80 SDKs, 29 CLIs, 10 Terraform providers |
| Kotlin | 33 | 24 are Java-target SDKs, because Stainless writes Java SDKs in Kotlin; 7 are Kotlin SDKs |
| Ruby | 33 | |
| PHP | 19 | |
| C# | 15 | |

## 2. Oracle inventory (non-Python targets)

These are the targets enabled in each pinned config (`oracles/cache/<g>-config/<config_path>`):

| group | non-Python targets in stainless.yml |
|---|---|
| onebusaway | node `OneBusAway/js-sdk`, go `OneBusAway/go-sdk`, java `OneBusAway/java-sdk`, kotlin `OneBusAway/kotlin-sdk`, ruby `OneBusAway/ruby-sdk` |
| opencode | typescript `sst/opencode-sdk-js`, go `sst/opencode-sdk-go` (both redirect to `anomalyco/…`) |
| isaacus | typescript `isaacus-dev/isaacus-typescript` (+ `mcp_server`) |
| lumaai | node `lumalabs/lumaai-node` |
| honcho | node `plastic-labs/honcho-node-core` (+ `mcp_server`) |
| stagehand | go, java, kotlin, ruby, php, csharp (`browserbase/stagehand-{go,java,kotlin,ruby,php,net}`, every edition `*.2025-10-08`); typescript has `production_repo: null` |
| ogx | node `ogx-ai/ogx-client-typescript`, go `ogx-ai/ogx-client-go` (`enable_v2`) |
| imagekit | typescript `imagekit-nodejs`, go `imagekit-go` (`enable_v2`), java `imagekit-java`, ruby `imagekit-ruby` (all `#master`); php and csharp have no production_repo, but `imagekit-php` and `imagekit-dotnet` exist; cli, terraform and kotlin have `production_repo: null` |
| knock | typescript `knocklabs/knock-node`, go `knock-go`, java `knock-java`, ruby `knock-ruby` |
| knock-mgmt | typescript `knocklabs/knock-mgmt-node`, go `knock-mgmt-go` (`enable_v2`) |
| papr | typescript `Papr-ai/papr-TypescriptSDK` |

**How a repo was paired.** For every commit that touched `.stats.yml` on any branch, its
`openapi_spec_hash` and `config_hash` were compared with the manifest's `verify_md5`.

- **proven**: both hashes equal the pinned config.
- **= py**: both hashes equal the Python SDK's `.stats.yml`. This applies to knock-mgmt and papr,
  whose configs are pinned as "nearest".
- **cfg-only**: only the config hash matches, so the spec has to come from that commit's
  `openapi_spec_url`. Those GCS URLs still return HTTP 200; this was checked on 2026-10-26.

"Pin" is the newest commit carrying the matching `.stats.yml`. Where there is one, it is the
newest Stainless `release:` commit on the default branch. "Last Stainless" is the newest
`stainless-app[bot]` commit on any branch. All 31 repos exist and are public.

| repo | licence | pairing | pin (date, branch) | last Stainless |
|---|---|---|---|---|
| OneBusAway/js-sdk (`node`) | Apache-2.0 | proven | `f3ca3f47` release 1.20.0 (07-18, main) | `70b0a95` 2026-08-20 |
| OneBusAway/go-sdk | Apache-2.0 | proven | `d115209e` release 1.15.0 (07-18, main) | `08df6a0` 2026-08-21 |
| OneBusAway/java-sdk | Apache-2.0 | proven | `827099e` alpha.59 (07-18); same stats through `c29c48ad` alpha.60 (08-04) | `8af5da7` 2026-08-20 |
| OneBusAway/kotlin-sdk | Apache-2.0 | proven | `69234467` alpha.81 (07-18, main) | `a14b278` 2026-08-20 |
| OneBusAway/ruby-sdk | Apache-2.0 | proven | `46a574dd` release 1.12.0 (07-18, main) | `ee01d18` 2026-08-20 |
| anomalyco/opencode-sdk-js | MIT | cfg-only (5 commits, 2025-07-28, 5 different specs) | `7b50ae5` (spec from its URL) | `74afae0` 2026-02-03 |
| anomalyco/opencode-sdk-go | MIT | proven | `04c05a5d` alpha.28 (2025-07-29, main) | `a2b37c9` 2026-01-17 |
| isaacus-dev/isaacus-typescript | Apache-2.0 | proven | `c56af182` release 0.23.0 (06-17, main) | `3778a1a` 2026-08-25 |
| lumalabs/lumaai-node (`node`) | Apache-2.0 | proven | `4dbfae0f` release 1.21.x (04-07, main) | `d3e5782` 2026-08-20 |
| plastic-labs/honcho-node-core (`node`) | Apache-2.0 | proven | `91d25806` (2025-10-08, main; release 1.5.0 `88918f0` follows) | `afc5f9d` 2026-01-17 |
| browserbase/stagehand-go | MIT | proven, only on `next` | `8e7365aa` (07-21, next) | `2422956` 2026-07-21 |
| browserbase/stagehand-java (archived) | MIT | proven, only on `next` | `ca86bdb3` (08-14, next) | `42d2758` 2026-08-14 |
| browserbase/stagehand-kotlin (archived) | MIT | proven, only on `next` | `cbdefde6` (08-14, next) | `ab301e6` 2026-08-14 |
| browserbase/stagehand-ruby | MIT | proven, only on `next` | `dadba887` (07-21, next) | `87fdd67` 2026-07-21 |
| browserbase/stagehand-php (archived) | MIT | proven, only on `next` | `38312f0f` (08-14, next) | `570c2c3` 2026-08-14 |
| browserbase/stagehand-net (archived, C#) | MIT | proven, only on `next` | `0d81242a` (07-25, next) | `06de8bf` 2026-07-25 |
| ogx-ai/ogx-client-typescript (`node`) | MIT | proven | `029aa5f8` release 1.1.0-alpha.6 (06-11, main) | `3a87723` 2026-08-19 |
| ogx-ai/ogx-client-go | MIT | proven, only on `generated` | `c66deeff` (06-10, generated) | `6fac7c3` 2026-08-19 |
| imagekit-developer/imagekit-nodejs | Apache-2.0 | proven | `9161f99f` (05-17, main) | `4279fef` 2026-06-18 |
| imagekit-developer/imagekit-go (v2) | Apache-2.0 | proven | `bc22fec7` (05-17, main) | `97a134a` 2026-06-18 |
| imagekit-developer/imagekit-java | Apache-2.0 | proven | `f96c35e8` (05-17, main) | `133b9e2` 2026-06-18 |
| imagekit-developer/imagekit-ruby | Apache-2.0 | proven | `65401086` (05-17, main) | `c7bd81a` 2026-06-18 |
| imagekit-developer/imagekit-php | MIT | proven | `9f9309d5` (05-17) | `ada6b90` 2026-05-23 |
| imagekit-developer/imagekit-dotnet | Apache-2.0 | unpaired: its hashes come from July 2026 configs | none | `bdf0d20` 2026-07-27 |
| knocklabs/knock-node | Apache-2.0 | proven | `e2c0f18a` release 1.7.0 (2025-06-06, main) | `b7cdf1a` 2026-07-14 |
| knocklabs/knock-go | Apache-2.0 | proven | `4b86a0d6` release 1.5.0 (2025-06-06, main) | `a6cf779` 2026-07-14 |
| knocklabs/knock-java | Apache-2.0 | proven, only on `generated` (main was rewritten by hand later) | `6e0df234` (2025-06-06) | `9fc337c` 2025-07-09 |
| knocklabs/knock-ruby | Apache-2.0 | proven | `5bc0a747` release 1.6.0 (2025-06-06, main) | `750c0ce` 2026-07-14 |
| knocklabs/knock-mgmt-node | Apache-2.0 | = py (nearest) | `ca88fdba` release 0.32.0 (07-09, main) | `a912c9f` 2026-07-15 |
| knocklabs/knock-mgmt-go (v2) | Apache-2.0 | = py (nearest) | `85ae774c` release 0.2.0 (07-09, main) | `0e73fe1` 2026-07-15 |
| Papr-ai/papr-TypescriptSDK | Apache-2.0 | = py (nearest) | `41dbb3b3` release 1.20.0 (2025-12-07, main) | `80e77d9` 2026-09-18 |

Full 40-character SHAs are recorded in `oracles/manifest.yaml` as each oracle is pinned.

Notes on the table:

- **OneBusAway has a complete proven set.** All five non-Python SDKs, every one Apache-2.0,
  carry spec `8df6b187…` and config `c28ddf5b…`, exactly like python-sdk 1.30.0.
  - Their `.stats.yml` `openapi_spec_url` is the same open-transit URL the Python oracle uses.
  - The 2026-07-18 release commits are the same generation as the Python pin (4c20fa83, also
    2026-07-18).
  - Commits from 2026-08-11 on ("feat: initial stlc build", `stlc-bot`) come from a different
    generator.
- **Coverage of the default branch.** Every Apache-2.0 repo except knock-java and imagekit-dotnet
  has a proven commit on its default branch. The stagehand SDKs (MIT) pair only on `next`: their
  last generation was never merged to `main`, which was later marked deprecated.
- **`generated` branch.** Almost every repo has a `generated` branch (pure generator output).
  It is the natural merge base for `adopt` and `merge`.

## 3. Shape of each Stainless SDK

Observations come from the pinned trees. File counts are for OneBusAway (30 endpoints).

### TypeScript has two layouts

The layout depends on which config key the SDK uses:

| | `node` key: legacy layout | `typescript` key: current layout |
|---|---|---|
| oracles | OBA, lumaai, honcho, ogx | opencode, isaacus, imagekit, knock, knock-mgmt, papr |
| runtime files | `src/core.ts`, `src/_shims/**` (28 files), `src/shims/*`, `src/internal/**`, `src/error.ts`, `src/uploads.ts`, `src/resource.ts` | `src/core/{api-promise,error,resource,uploads}.ts` plus `src/core/README.md`, `src/internal/**` (about 22 files: `headers`, `parse`, `request-options`, `shims`, `to-file`, `utils/*`, …), `src/{api-promise,error,uploads,resource}.ts` |
| generated files | `src/index.ts` (client), `src/resources/**`, `api.md`, `tests/api-resources/**`, `examples/` | `src/client.ts`, `src/resources/**`, `api.md`, `tests/**` |
| MCP server | none | `packages/mcp-server/**` (about 31 files) when `options.mcp_server` is set (isaacus, imagekit, papr; also honcho, which uses the `node` key) |
| TypeScript version | 4.8 | 5.8.3 |

Both layouts:
- Tooling: `package.json` with yarn 1, jest, eslint, prettier (`printWidth` 110, single quotes,
  trailing commas, experimental ternaries), `tsc-multi` for the CJS and ESM builds, and four
  tsconfigs (`build`, `deno`, `dist-src`, main).
- Header line: `// File generated from our OpenAPI spec by Stainless. See CONTRIBUTING.md for
  details.`
- Property names keep their wire spelling; methods and classes are camelCase or PascalCase.

OBA js: 176 files, of which 36 runtime, 31 resources and 34 tests.

### Go

- **Generated files:** one file per resource at the root (`agency.go`, `stop.go`, … 32 files),
  each with a `_test.go` (36 test files), plus `client.go`, `aliases.go`, `api.md`, `examples/*/`
  and `shared/shared.go`.
- **Runtime (v1, OBA and knock):** `internal/{apierror, apiform, apijson, apiquery, param,
  requestconfig, testutil, version.go}`, `option/{middleware.go, requestoption.go}`, `field.go`
  and `default_http_client.go`. That is 26 files.
- **Runtime (v2, `enable_v2`: imagekit, knock-mgmt, ogx, stagehand):** also `internal/encoding/`,
  `internal/paramutil/`, `packages/param/`, `packages/respjson/`, `shared/constant/` and
  `backcompat.go`. That is 55 files.
- **Tooling:** `go.mod` (OBA: go 1.22; deps only `tidwall/gjson` and `sjson`), formatted with
  gofmt.

### Java and Kotlin

- **Language.** The Java target is written in Kotlin, with an API shaped for Java callers. The
  `java` and `kotlin` targets have almost the same layout and runtime: 54 and 52 runtime files.
- **Gradle multi-module project:**
  - `build.gradle.kts`, `settings.gradle.kts` and `buildSrc` convention plugins
    (`<pkg>.java/.kotlin/.publish.gradle.kts`);
  - the Gradle wrapper;
  - the modules `<artifact>-core`, `<artifact>-client-okhttp`, `<artifact>` (aggregate),
    `-example`, `-lib`, and `-proguard-test` (Java only).
- **Runtime:**
  - `-core/src/main/kotlin/<reverse_domain>/core/**`: `ClientOptions`, `RequestOptions`,
    `Values` (`JsonField`), `ObjectMappers`, `Params`, `PhantomReachable*`, `Sleeper`,
    `Timeout`, `handlers/*` and `http/*` (17 files: `HttpClient`, `RetryingHttpClient`,
    `QueryParams`, `StreamResponse`, …);
  - `errors/**` (13 files);
  - the client-okhttp module.

  These files sit in the customer's package namespace (`org.onebusaway.core`), so copying them
  means renaming the package.
- **Generated:** `client/*` (four classes: sync and async, interface and impl), `models/<resource>/*`
  (60 files) and `services/{blocking,async}/**` (116 files), plus 133 test files. OBA java has
  438 files in total. It is the largest output per endpoint.
- **Dependencies:** Jackson 2.18 (tests force 2.14), okhttp, and in tests JUnit 5, wiremock,
  assertj and mockito.
- **Formatting:** palantir-java-format and ktfmt `--kotlinlang-style`. Docs are built with dokka.

### Ruby

- **Three views of every file:** `lib/<pkg>/**.rb`, `rbi/<pkg>/**.rbi` (Sorbet) and
  `sig/<pkg>/**.rbs` (RBS). The gemspec ships all three.
- **Runtime:** `internal/{transport/base_client, transport/pooled_net_requester, type/*
  (base_model, union, enum, converter, …), util}`, `errors`, `file_part` and `request_options`,
  in each of the three views. That is 57 files.
- **Generated:** `client`, `models/**` (60 files in each view) and `resources/**` (29 in each view).
  OBA ruby has 402 files in total.
- **Tooling:** Gemfile, Rakefile tasks (minitest, rubocop, syntax_tree formatting,
  `typecheck:steep`, `typecheck:sorbet` with tapioca), Steepfile, `sorbet/config`, Ruby ≥ 3.2.
  Runtime dependencies are only `cgi` and `connection_pool`.

### PHP (stagehand, imagekit-php)

- **Runtime:** `src/Core.php` and `src/Core/**` (44 files), `src/Client.php`,
  `src/RequestOptions.php` and `src/SSEStream.php`.
- **Generated:** `src/Services/`, `src/ServiceContracts/` and model directories per resource.
- **Tooling:** composer, phpstan, php-cs-fixer and phpunit.

### C# (stagehand-net)

- **Runtime:** `src/<Pkg>/Core/**` (22 files) and `Exceptions/**` (15 files).
- **Generated:** `Models/**`, `Services/**`, `<Pkg>Client.cs`, a `.sln`, csproj files with
  `Directory.Build.props`, and `<Pkg>.Tests`.
- **Tooling:** csharpier formatting.

### Tests against a mock server

The test scripts are the same across languages. `scripts/test` starts `scripts/mock --daemon`
unless `TEST_API_BASE_URL` is set. The mock runs Stainless's Steady mock server
(`npm exec --package=@stdy/cli@0.22.1 -- steady … --validator-query-array-format=repeat …`) on
`127.0.0.1:4010` against the `openapi_spec_url` in `.stats.yml`. Then each language runs its own
test command:

| language | command |
|---|---|
| TypeScript | `./node_modules/.bin/jest` |
| Go | `go test ./...` |
| Java / Kotlin | `./gradlew test` |
| Ruby | `bundle exec rake test` |

Some newer SDKs have no `scripts/mock` (isaacus-typescript, imagekit-go); their `scripts/test`
runs the tests directly. So "run the SDK's own tests" does not depend on the language: it is
`./scripts/test` with the right toolchain installed.

### Public-API diff tool per language

None of these tools is copied into Ferrule; they are only run.

| language | tool | what it compares |
|---|---|---|
| TypeScript | `@microsoft/api-extractor` (MIT) on the emitted `.d.ts` (API report diff), or a small TS-compiler-API checker that tests whether each old exported type is assignable to the new one | Stainless already runs `attw` (`scripts/utils/attw-report.cjs`) for package-shape checks |
| Go | `golang.org/x/exp/apidiff` (BSD-3), or `gorelease`, which wraps apidiff and suggests a semver bump | packages, old against new |
| Java | japicmp (Apache-2.0) | the core jar, binary and source compatibility |
| Kotlin | kotlinx binary-compatibility-validator (Apache-2.0; ABI validation is now built into the Kotlin Gradle plugin) | `.api` dumps |
| Ruby | no standard tool | the generated `sig/**/*.rbs`, parsed with the `rbs` gem (`RBS::Parser`) and compared by class, method and type signature; the `.rbi` files are a second check |
| PHP | roave/backward-compatibility-check (MIT) | |
| C# | Microsoft.DotNet.ApiCompat / package validation (MIT) | |

## 4. Forge and other languages

Forge ships one generator besides the plugin interfaces: TypeScript.

- **Library** (`src/plugin/ts.rs`, `src/plugin/ts/{file,print}.rs`, exported as `forge::ts`): a
  printer with `File`, `Module`, `Ident` and the `p!`, `frag!` and `join!` macros. It resolves
  and aliases imports automatically.
- **Binary** (`src/bin/forge/ts.rs` plus `ts/{client,codec,index,models,operation,types}.rs`): a
  `Plugin` impl "ported from Go's `internal/ts`". It writes a Cloudflare-style SDK: `client.ts`,
  `models.ts` and `index.ts` per group, plus a runtime in `_/` (`codec.ts`, `transport.ts`,
  `error.ts`).

The binary's output is nothing like Stainless's layout, so it cannot produce a Stainless TS SDK.
`forge::ts::File` would be useful, but it hard-codes two things that conflict with Stainless
output:
- the preamble `// Code generated by forge. DO NOT EDIT.\n/* eslint-disable */`;
- a wrap width of 80 columns, where Stainless uses 110 and prettier.

Making the preamble and width configurable is a small upstream PR. Until then, a Ferrule TS
emitter needs its own printer, the way `PythonPrinter` exists for Python. Forge has nothing for
Go, Java, Kotlin, Ruby, PHP or C#. Its README says targets "namely, TypeScript, Go, Python,
Terraform" will come as Cloudflare ships them for its own SDKs.

### Is Forge's IR language-neutral?

Yes:
- `Name` is a list of lower-case words with `camel()`, `pascal()`, `snake()` and `kebab()`.
- `Type` covers `Scalar` (`Uuid`, `DateTime`, `Date`, `Bytes`, `Binary`, `Number`) with integer
  and float sizes, plus `Enum`, `Array`, `Object` (fields and extra keys), `Union` (with
  `Discriminator`), `Intersection` and `Model`.
- `Group`, `Method`, `Location`, `Body` and `Response` describe HTTP, not a language.
- `Plugin::{generate, generate_group}` makes no assumption about the language.

So one `Forge` value can drive several plugins in one process: call `forge::run` once per
language.

The gaps in `docs/forge-gaps.md` (G2–G6, G18, G20, G21, …) apply equally to every language. The
`x-ferrule-*` workarounds in the converter are already language-neutral.

### What in Ferrule is Python-specific

**`crates/python-plugin/src/model.rs` (SdkModel)** bakes in Python:
- `SdkField.py_name` and `SdkParam.py_name`;
- `SdkRef.module` is a dotted path under Python's `types` package;
- `SdkPagination.page` is a Python class name such as `CursorPage`;
- `SdkClientOpt.name` is a Python argument name;
- `SdkClientSettings.class_name` is the Python client class;
- `SdkResource.path`, `class_segment` and `singular_class_segment` are Python spellings;
- `SdkPrimitive` drops integer sizes, `uuid`, `bytes` and `binary`, which Go, Java and C# need.

The adapter (`adapter/to_model.rs`) fills these with `naming::snake_case`/`pascal_case` and
Python's keyword guard and acronym list.

**The converter** handles only `targets.python.edition` (`crates/converter/src/editions.rs`).

### What a language-neutral model needs

- **Names:** keep the word list (`Name`), the wire name, and any explicit spelling from
  stainless.yml (`custom_casings`, `x-ferrule-method-name`, `$shared`). Each language's `Namer`
  turns them into identifiers. The rules differ:
  - Go: exported PascalCase with initialisms (`ID`, `URL`) and JSON tags;
  - TS: wire spelling for properties, camelCase for methods;
  - Java/Kotlin: camelCase accessors and builders;
  - Ruby: snake_case;
  - Python: today's rules.
- **Type references:** a model is identified by its schema name and owning resource path. Each
  language decides where it lives: Python `types/x.py`, TS `resources/x.ts`, Go root `x.go`, JVM
  `models/<resource>/X.kt`.
- **Types:** keep Forge's full scalar set with number sizes, and keep "nullable" separate from
  "optional". Go `param.Opt`, Java `JsonField` with `Optional`, and Kotlin `?` all need the
  difference. Also needed:
  - union variants with stable variant names and the discriminator;
  - enum members with names, because Java and Kotlin generate "known/unknown" enum classes;
  - inline objects keyed by path, as today.
- **Pagination and streaming:** the scheme, the purpose map and the item type. The page class
  name comes from the language.
- **Client settings:** options, environments, auth, retries and timeout as data. Names such as
  `api_key`/`apiKey`/`APIKey` and `ONEBUSAWAY_SDK_API_KEY` come from the language.
- **Per-target data:** an `SdkTarget` enum with that target's config (package_name,
  reverse_domain, gem_name, go options, edition defaults). The converter fills it from
  `targets.<lang>` and the resource/method `skip`/`only` lists.

## 5. Recommendation

### Order

1. **TypeScript `typescript` key** (current layout).
   - Demand: the most repos (224).
   - Oracles: 6 Apache/MIT oracles, of which isaacus (5 endpoints), knock, imagekit,
     knock-mgmt and papr pair on their default branch. Start with isaacus.
   - Work involved: a prettier-exact printer (110 columns), and a later phase for the MCP
     subpackage.
2. **TypeScript `node` key** (legacy layout). Mostly a different runtime plus file layout over
   the same emitter. OBA js (proven) and lumaai, honcho and ogx are the oracles.
3. **Go.**
   - Demand: about 80 SDKs, and Go is also the base that CLI and Terraform require.
   - Oracles: OBA (v1), knock (v1), imagekit (v2), knock-mgmt (v2), opencode, ogx and stagehand.
   - gofmt is deterministic, so formatting is the cheapest of all languages.
   - Do v1 and v2 (`enable_v2`) as two runtime flavours.
4. **Java and Kotlin together.**
   - One JVM plugin that emits Kotlin source in two API flavours, with a shared `core/` runtime.
   - Oracles: OBA java and kotlin (proven, Apache), imagekit-java, knock-java (`generated`
     branch), stagehand.
   - The largest emitter per endpoint (builders, `JsonField`, validation, async services).
5. **Ruby.** About 33 repos. Oracles: OBA, knock, imagekit, stagehand. Three views (rb, rbi, rbs)
   of the same model; the RBS view gives the API diff for free.
6. **PHP and C#.** 19 and 15 repos. Oracles: stagehand (MIT, `next`), imagekit-php (MIT, proven),
   imagekit-dotnet (Apache, unpaired with our pin).
7. **CLI and Terraform.** Only after Go. The imagekit config enables both, but no public repo
   exists.

OneBusAway should be the cross-language reference oracle at every step. It is the only group
with a proven, Apache-2.0 pair in five languages besides Python, from the same generation date
as the Python oracle.

### Architecture

```
crates/converter        stainless.yml + spec -> annotated spec, per target
                        (target-aware editions, skip/only, targets.<lang> -> x-ferrule-target)
crates/sdk-model        NEW: language-neutral SdkModel + adapter (the only crate that imports Forge)
crates/python-plugin    namer + emitters + printer for Python (adapter moves out)
crates/ts-plugin        typescript and node layouts
crates/go-plugin        v1 / v2
crates/jvm-plugin       java + kotlin flavours
crates/ruby-plugin      rb + rbi + rbs
crates/ferrule-gen      runs converter, then forge::run per requested target
runtime/<lang>/         template runtime per language (python/ today's runtime/, ts-node/,
                        ts/, go-v1/, go-v2/, jvm/, ruby/), each with NOTICE and tests in its
                        own toolchain
```

- **Adapter rule (ADR 0002).** It moves to `crates/sdk-model`, so the rule "only the adapter
  imports Forge" holds once for all languages.
- **Plugin crates.** Each depends on `sdk-model` and holds three pieces:
  - a `Namer`, the only place spellings are made;
  - emitters, with one mode flag per variant, the same DRY rule as sync/async;
  - a printer.
  The "type mapping in one function" principle becomes one type-mapping function per language.
- **Runtimes.** Copy each template from an Apache-2.0 oracle:
  - TS-node, Go v1, JVM and Ruby from OneBusAway;
  - TS and Go v2 from knock, isaacus or imagekit.

  The templates rename the package or module (Go module path, JVM `reverse_domain`, Ruby module,
  TS package name). The stagehand, opencode and ogx runtimes are MIT: usable as oracles, but
  copying their runtime needs the plan's source list extended (ADR) first.
- **Formatters.** Post-generation formatting runs in the CLI "finish" stage, as ruff does today:
  prettier, gofmt, palantir-java-format, ktfmt, syntax_tree/rubocop. Byte-exact printers come
  later, where they matter.

### How verify and adopt generalise

- **`adopt` needs only small changes.**
  - `.stats.yml`, spec rescue from `openapi_spec_url` and the hash checks are identical in every
    language.
  - The target is detected from the repo's files (`package.json`, `go.mod`, `settings.gradle.kts`,
    `*.gemspec`, `composer.json`, `*.sln`) or from `production_repo`.
  - Runtime copy-from-customer becomes a per-language list of runtime paths, as in §3.
  - The hand-edit diff can use the repo's `generated` branch as the base.
- **`verify` becomes a driver over three checks:**
  - `ApiDiff` per language (griffe today; then api-extractor or a TS assignability check,
    apidiff/gorelease, japicmp/BCV, RBS diff, roave, ApiCompat);
  - the repo's own `./scripts/test` against Steady, which is language-agnostic;
  - HTTP replay and normalized text diff, which are already language-agnostic.

  Toolchains should run in the SDK's own `.devcontainer`, which Stainless repos ship, or in pinned
  containers, so users do not need every toolchain installed.
- **`merge`.** The libcst AST fallback is Python-only. Other languages start with a plain
  three-way merge on the `generated` branch, with a tree-sitter fallback later.
- **`ferrule-gen`.** It takes `--target <lang>` and defaults to every enabled target that has a
  plugin.
- **Parity harness.** It gets per-language oracle entries in `oracles/manifest.yaml`:
  `kind: sdk`, `language: go`, the same `pairs_with` config, and the pins in the table in §2.
