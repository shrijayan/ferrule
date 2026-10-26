# ferrule-jvm-plugin

The JVM plugin (ADR 0017): the neutral model of `ferrule-sdk-model` to a Java or Kotlin SDK in
Stainless's Gradle layout. Stainless writes both its `java` and `kotlin` targets in Kotlin with
the same layout (`<artifact>-core` with `models/`, `services/{blocking,async}/`, `client/`;
`<artifact>-client-okhttp`; `<artifact>`; `-example`, `-lib`, `-proguard-test`), so one crate
writes both, with `Flavor` as the one mode flag. It never imports Forge; the model comes from
`ferrule_sdk_model::forge_to_model`.

```text
annotated spec --sdk-model adapter--> model::Client --for_target("java"|"kotlin")-->
  namer + emitters --> Kotlin text + Gradle files + runtime options
  --ktfmt 0.61 --kotlinlang-style, ferrule_runtime.render_java_files--> SDK
```

- `naming.rs`: Stainless's JVM spelling: classes `PascalCase` and members `camelCase` with every
  word capitalised as a word (`agencyID` -> `agencyId`), `x-ferrule-casings`, enum constants
  `SCREAMING_SNAKE`, packages as lower-case words run together.
- `types.rs`: the one type mapper (`Jt`) and the nested classes Stainless declares for inline
  objects and enums, named by position (field name, array items singular). Field order is
  Stainless's: `allOf` bases first, then required before optional, `id` first, by wire name
  ignoring case.
- `kt.rs`: file header, imports and `KDoc`. The emitters write Stainless's tokens, comments and
  blank lines but not its line breaks: ktfmt lays the text out (in `ferrule generate` and the
  tests), sorts the imports and drops unused ones. Text ktfmt keeps as written (the `fromEnv`
  table, code fences) is written in its final layout.
- `emit/`: models (`model.rs`); params with builders, path, query and body params
  (`params.rs`: a JSON body as the nested `Body`, a model body as one param, additional body
  properties otherwise); services and their `WithRawResponse` views, blocking and async
  (`service.rs`); the clients, `ClientOptions.kt` and the OkHttp clients (`client.rs`); the tests
  (`tests.rs`); the Gradle and project files (`project.rs`); the runtime template's options
  (`runtime.rs`). `templates/` holds the fixed texts (`tmpl.rs` renders them; provenance in
  `NOTICE`, branding in `templates/BRANDING.md`).
- `meta.rs`: the one accessor for JVM settings, `jvm_target` (`targets.<java|kotlin>`:
  `reverse_domain`, `production_repo` and its `#branch`), overridden by `JvmMeta`, which also
  carries what no config key holds yet (the release version, the README example, the LICENSE
  year, `organization.docs` and `contact`, `disable_mock_tests`).

`ferrule-gen generate --target java|kotlin` writes the files and `.ferrule/jvm-runtime.json`;
`ferrule generate --target java|kotlin` runs ktfmt over the plugin's Kotlin files, renders the
runtime from the options, removes them and makes the scripts executable.

## Proof

`tests/oracle.rs` compares the formatted output with the Stainless SDKs, byte for byte after the
branding substitutions:

| Oracle | Identical | Recorded differences |
|---|---|---|
| onebusaway-java (`c29c48ad`, Apache-2.0, same generation as the Python pin) | 328 of 343 | 14 tests: Forge gap G18 (parameter and field `example`s, `maximum` not in the model); `StopsForAgencyListResponse.kt`: Stainless spelled one KDoc type in full in this file only |
| onebusaway-kotlin (`69234467`, Apache-2.0, same generation) | 282 of 338 | the same G18 tests; 43 files carry hand edits of the Kotlin repo (`limitExceeded`/`outOfRange` and the `Situation.Reason` enum), which the Java repo of the same generation does not have |
| imagekit-java (`f96c35e8`, Apache-2.0, same generation as the imagekit-python pin) | 187 of 291 (a ratchet) | unions, multipart uploads, webhooks, G18 examples, path params' schema descriptions, imagekit's custom code (`lib/Helper`) |

It also checks that every generated OneBusAway file is emitted, but `README.md` and
`.stats.yml`. The runtime is proven by `runtime/tests/test_render_oracles_java.py`.
`tests/golden.rs` pins a synthetic API that covers JSON bodies, an empty response, two path
params, subresources, enums and a resource model.

## Not done yet

- `README.md`, `.stats.yml`; the binary `gradle/wrapper/gradle-wrapper.jar`.
- Unions (mapped to `JsonValue`), multipart uploads, pagination (`AutoPager`/`Page`), SSE
  streaming, header params, webhooks (`UnwrapWebhookParams`, the `standardwebhooks` dependency),
  `JsonValue` test values, `baseUrlOverridden()`.
- Model gaps: parameter and field `example`s and `maximum` (Forge gap G18); a path param's schema
  description apart from its own (Stainless documents path params with the schema's); a resource
  model's resource path (`x-ferrule-models` writes Python's module), so models are placed by use.
- The 2025 Stainless generation (knock-java).
- Not built with Gradle here: the sandbox could not download Gradle 8.12 or the Maven
  dependencies. ktfmt parses every generated file.
