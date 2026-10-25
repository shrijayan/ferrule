# ferrule-runtime

The runtimes copied into every Ferrule-generated SDK, one per target language, plus the code
that renders them. Each runtime is the runtime of a Stainless-generated SDK (Apache-2.0) with
per-SDK names replaced by tokens; rendering is pure and byte-identical to the oracle's files.

| Language | Template | Renderer | Source (Apache-2.0) |
|---|---|---|---|
| Python | `template/` | `render_files`, `render` | `OneBusAway/python-sdk` @ `4c20fa83` |
| TypeScript (`typescript` target, current layout) | `typescript/template/` | `render_typescript_files` | `isaacus-dev/isaacus-typescript` @ `c56af182`, `src/internal/qs/` from `knocklabs/knock-mgmt-node` @ `ca88fdba` |
| Go (v1) | `go/template/` | `render_go_files` | `OneBusAway/go-sdk` @ `d115209e` |
| JVM (`java` and `kotlin` targets) | `java/template/` | `render_java_files` | `OneBusAway/java-sdk` @ `c29c48ad`, `OneBusAway/kotlin-sdk` @ `69234467`, `OkHttpClient.kt` and the webhook files from `imagekit-developer/imagekit-java` @ `f96c35e8` |

- Templates are third-party code: not linted or formatted by our rules (excluded in the root
  `pyproject.toml` and `biome.json`).
- `src/ferrule_runtime/`: the renderers, the shared engine `render_tree` and the Python
  `pagination.py` generator. Python's template stays at `template/` (not `python/template/`)
  because the wheel layout and ADRs 0004 and 0016 refer to that path.

All languages share one engine, `render_tree(root, *, replacements, flags, comment, slots,
gated, line_length)`: whole-line directives `<comment>ferrule:if <flag>` / `else` / `endif` and
`<comment>ferrule:slot <name>` (`#` for Python, `//` for TypeScript, Go and the JVM), then
token replacement in the text and in the path (a directory named `__ferrule_pkg_path__`
becomes `org/acme`), then a check that no `__ferrule_*__` / `__Ferrule*__` / `__FERRULE_*__`
token is left. `gated` maps a template path prefix to whether files under it are rendered. A
slot line is replaced by text the caller supplies (empty removes it); a malformed directive is
an error.

`<comment>ferrule:if fits` is computed per block, never passed as a flag: it holds when the one
line of its branch, with the tokens replaced, is at most `line_length(path)` columns (the
formatter's line length for that file), and its `else` branch holds the formatter's wrapped
form. That is how a line that wraps or not with the length of a name (formatters such as ktfmt
wrap at a fixed width) renders right for any name.

## Python

### API

```python
from ferrule_runtime import render, render_files, PageScheme, PageField, PageModel

render("acme", out / "src" / "acme")  # entry point used by `ferrule generate`
files = render_files("acme", "Acme", timeout=60)  # pure: {"_base_client.py": "...", ...}
```

`render(package, package_dir, /, *, client_name=None, pagination=(), **options)` writes the
files and returns their paths. `render_files(package_name, client_name, *, error_name,
log_env_var, repo_url, default_branch, timeout, max_retries, min_python, webhooks,
line_length, pagination)` returns `{relative path: text}`; `pagination.py` is included when
`pagination` schemes are given (`render_pagination` emits it alone).

Pagination kinds: `cursor`, `cursor_id`, `cursor_url`, `offset`, `page_number`, `fake_page`.
Class names are `Sync<name>` / `Async<name>`.

### Template format

Placeholders: `__ferrule_pkg__` (package), `__FerruleClient__` (client class; also
`Async__FerruleClient__`), `__FerruleError__` (base exception, default `<Client>Error`),
`__FERRULE_LOG_ENV__` (default `<CLIENT_SCREAMING_SNAKE>_LOG`), `__FERRULE_TIMEOUT__`,
`__FERRULE_TIMEOUT_COMMENT__`, `__FERRULE_MAX_RETRIES__`, `__FERRULE_FILE_UPLOADS_HINT__`.
Config-dependent blocks use whole-line directives `#ferrule:if <flag>` / `#ferrule:else` /
`#ferrule:endif`, removed on render. Flags: `py39` (`min_python < 3.10`), `webhooks`,
`base_model_hint_fits` (ruff-format line length depends on the package name length).

### File classification

Decided by comparing the same file across oracles after normalising package, client and error
names (onebusaway, groq, lithic, isaacus, perplexity, knock-mgmt, imagekit and others).

| File | Class | Notes |
|---|---|---|
| `_base_client.py`, `_compat.py`, `_models.py`, `_qs.py`, `_resource.py`, `_types.py`, `py.typed` | runtime | identical across oracles |
| `_utils/*` (14 files) | runtime | `_logs.py`: env var; `_compat.py`: py3.9 branch |
| `_exceptions.py` | runtime | generic part; `APIWebhookValidationError` only with webhooks |
| `_files.py` | runtime | README file-upload link only when the repo URL is known |
| `_constants.py` | runtime | timeout / max retries from `client_settings` |
| `_response.py` | runtime | one line wraps by package-name length |
| `_streaming.py` | runtime | default variant only (see drift) |
| `pagination.py` | generated | `render_pagination` from config schemes |
| `__init__.py`, `_client.py`, `_version.py`, `resources/`, `types/`, `lib/` | generated | python-plugin (`crates/python-plugin`) |

### Version decision and drift

The newest Stainless oracles (onebusaway 2026-07-18, groq 2026-08-26, lithic 2026-09-18,
perplexity, knock-mgmt, isaacus, imagekit) share one runtime modulo config, so there is no
version drift among them: the OneBusAway copy is the newest text. Tests assert byte equality
for all seven. Known differences, each a config feature not yet templated:

- `_streaming.py`: groq, perplexity and ogx use custom SSE handling (`[DONE]`, `error` events).
- lithic: `_legacy_response.py` (legacy raw responses) touches `_base_client.py`, `_types.py`,
  `_response.py`.
- perplexity `_compat.py`: older `model_dump` text.
- Older SDKs (knock, honcho, papr, opencode 2025; lumaai 2026-04) are older Stainless versions;
  stagehand and ogx carry hand edits. Not used.

`pagination.py` is byte-identical for lithic (`cursor_id`, `fake_page`), honcho
(`page_number`), knock, knock-mgmt and ogx (`cursor`). `offset`, `cursor_url` and
`page_number` without a total field have no pinned oracle; their text is unverified but
behaviour-tested.

### Testing fake server

`render_files(..., testing=True)` adds `testing/__init__.py` and `testing/_fake.py`: an
in-process fake server (ADR 0016). Not in Stainless output; off by default, so parity is
unaffected.

```python
from acme.testing import fake, http_client

with fake() as client:  # Acme(base_url=fake, max_retries=0, http_client=...)
    client.users.retrieve("u1")  # deterministic example from the response schema
client = Acme(api_key="x", http_client=http_client(overrides={"GET /users/{id}": {"id": "u1"}}))
```

`FakeTransport` / `AsyncFakeTransport` match method and path template (literal paths before
templated ones; any base URL path prefix accepted) and never open a socket. Overrides map
`"METHOD /path/{param}"` to an `httpx.Response`, a callable taking the request, or a JSON body.
Unmatched requests get a 404. Bodies: `example`, `examples[0]`, `default`, `const`, `enum[0]`,
`$ref`, `allOf` (merged), first of `oneOf`/`anyOf`, then type defaults (`"string"` or a fixed
value per `format`, `0`, `0.0`, `false`, one array item). Optional properties without an
example are omitted, so cursors are absent and pagination stops; `$ref` cycles end there.

The plugin emits `<pkg>/testing/_routes.json` (missing file = no routes):

```json
{
  "version": 1,
  "client_options": {"api_key": "fake-api-key"},
  "schemas": {"User": {"type": "object", "required": ["id"], "properties": {"id": {"type": "string"}}}},
  "routes": [
    {"method": "GET", "path": "/users/{user_id}", "status": 200,
     "content_type": "application/json", "response": {"$ref": "#/schemas/User"}}
  ]
}
```

- `routes[]`: one per operation; `path` relative to the base URL with `{param}` templates;
  `status` (default 200) is the first 2xx response; `response` omitted for no body (204);
  `content_type` defaults to `application/json`.
- `schemas`: component schemas (OpenAPI 3.1 JSON Schema subset); refs are `#/schemas/<Name>`.
- `client_options`: keyword arguments the client needs to construct without the environment
  (required auth arguments with placeholder values, and `base_url` only if needed).

## TypeScript

For SDKs on the `typescript` target (Stainless's current layout, TypeScript 5.8). The legacy
`node` layout (`src/core.ts`, `src/_shims/**`: OneBusAway js-sdk, lumaai, honcho, ogx) is a
different runtime and is not templated.

```python
from ferrule_runtime import TypeScriptOptions, render_typescript_files

files = render_typescript_files(TypeScriptOptions(client_name="Acme"))
# {"src/core/error.ts": "...", "src/internal/utils/log.ts": "...", ...}: paths from the SDK root
```

`TypeScriptOptions(client_name, *, error_name=None, max_retries=2, query_array_format=None,
redacted_headers=("api-key", "x-api-key"), pagination=False)`:

- `error_name`: base error class, default `<client_name>Error`.
- `query_array_format`: `None` renders the basic `stringifyQuery` (primitives only, throws on
  nested values); `comma` / `brackets` / `repeat` / `indices` renders the vendored `qs`
  stringifier (`src/internal/qs/**`) with that `arrayFormat`.
- `redacted_headers`: headers `src/internal/utils/log.ts` masks besides `authorization`,
  `cookie` and `set-cookie`. Every 2026 oracle has `api-key`, `x-api-key` (their auth is bearer
  or basic); papr's 2025 generation listed its own auth headers (`x-session-token`,
  `x-api-key`) and opencode's none. Which headers the newest generator adds for a header-based
  API key is unverified.
- `pagination`: re-export `PagePromise` from `src/index.ts`.

Tokens: `__FerruleClient__`, `__FerruleError__`, `__FERRULE_MAX_RETRIES__` (the `@default`
in `request-options.ts`; no oracle uses a non-default value, so the text for one is
unverified), `__FERRULE_QS_ARRAY_FORMAT__`. Flags: `qs`, `pagination`. Slot:
`redacted_headers` (rendered from the option).

### File classification

Decided by comparing every file of the six current-layout oracles after normalising client,
error and package names and the env prefix.

| Files | Class | Notes |
|---|---|---|
| `src/core/{api-promise,error,resource,uploads}.ts`, `src/core/README.md` | runtime | |
| `src/internal/**` (22 files) | runtime | `utils/log.ts`: redacted headers; `utils/query.ts`: basic or `qs` |
| `src/internal/qs/**` (7 files) | runtime, with `query_array_format` | knock-mgmt-node's copy; a port of `qs` (BSD-3-Clause, its `LICENSE.md` is shipped) |
| `src/{api-promise,error,resource,resources,uploads}.ts` | runtime | re-export shims |
| `src/index.ts` | runtime | `PagePromise` export with pagination |
| `src/client.ts` | generated | request machinery mixed with config (auth, env vars, base URL, environments, resources); see below |
| `src/version.ts`, `src/resources/**`, `src/lib/**` | generated / hand-written | like Python's `_version.py`, `resources/`, `lib/` |
| `src/core/pagination.ts`, `src/pagination.ts` | generated | page classes from the config's schemes (knock, knock-mgmt) |
| `src/core/streaming.ts`, `src/streaming.ts`, `src/internal/decoders/line.ts` | not templated | SSE: only opencode-sdk-js (MIT) has them |
| `package.json`, `tsconfig*.json`, `scripts/`, `tests/**`, `packages/mcp-server/**` | not runtime | project files and tests, like Python's in `crates/python-plugin` |

Most of `src/client.ts` is runtime too: from the `stringifyQuery` method to the end of
`buildBody` (about 500 lines) the text is identical across isaacus and knock-mgmt-node, except
for the `getAPIList` / `requestAPIList` methods pagination inserts. The constructor, options,
auth, environments and resources around it are generated. That block belongs in the plugin's
client emitter as a fixed fragment, not in this template; imagekit-nodejs (2026-05) has an
older `buildBody`.

### Oracles

Checkouts at `oracles/cache/<name>` (`git clone`, then check out the commit); the tests skip
without them. Only Apache-2.0 repositories feed the template; opencode-sdk-js (MIT) is compared
only.

| Oracle | Commit | Options | Identical |
|---|---|---|---|
| isaacus-typescript | `c56af182d21e32ea472def11c3af17641d8f89c3` (0.23.0, 2026-06-17) | defaults | 33/33 |
| imagekit-typescript | `9161f99f561250b11b4535a328469bb5737cb4b8` (2026-05-17) | defaults | 33/33 |
| knock-mgmt-typescript | `ca88fdba703717f355ad3e18e850a0bb23e548f6` (0.32.0, 2026-07-09) | `comma`, pagination | 40/40 |
| knock-typescript | `e2c0f18ad31244906b8b7885d6d52bc51e35586b` (1.7.0, 2025-06-06) | `brackets`, pagination, no extra headers | 24/40 |
| papr-typescript | `41dbb3b3432a3b7d08bb1a09527a2e83db93fd39` (1.20.0, 2025-12-07) | papr's headers | 27/33 |
| opencode-sdk-js (MIT) | `7149e6a0575bc4e5b90489f31f509af7b226e2ec` (2025-09-03) | no extra headers | 25/33 |

The three 2026 oracles are byte-identical. The other differences are older Stainless
generations (missing `/* @__PURE__ */` annotations, older `env.ts`, `parse.ts`, `types.ts`,
no `utils/query.ts`), listed per file as `known_drift` in
`tests/test_render_oracles_typescript.py`. At knock-node's last Stainless commit
(`b7cdf1a4`, 2026-07-14, not pinned) 38 of 40 files are identical; the other two are knock's
hand edits (`utils/query.ts` re-exports `src/lib/stringifyQuery.ts`, `index.ts` exports
`src/lib`).

## Go

For SDKs on Stainless's Go v1 layout. v2 (`enable_v2`: imagekit-go, knock-mgmt-go, ogx,
stagehand) adds `internal/encoding/`, `internal/paramutil/`, `packages/param/`,
`packages/respjson/`, `shared/constant/` and `backcompat.go` and is not templated yet.

```python
from ferrule_runtime import GoOptions, render_go_files

files = render_go_files(
    GoOptions(module_path="github.com/acme/acme-go", package_name="acme", client_name="Acme")
)  # {"internal/apijson/decoder.go": "...", "option/requestoption.go": "...", ...}: module root
```

`GoOptions(module_path, package_name, client_name, *, org_name=None, max_retries=2,
redacted_headers=("api-key", "x-api-key"), config_fields="", config_clone="",
client_options="")`:

- `module_path`: the `go.mod` module; every internal import and the pkg.go.dev README links.
- `package_name`: the root package of `field.go` and `default_http_client.go`.
- `client_name`: the User-Agent product (`<client_name>/Go <version>`).
- `org_name`: `organization.name`, named in the `RequestOption` doc comment (default
  `package_name`; OneBusAway's is `onebusaway-sdk`).
- `redacted_headers`: headers `option.WithDebugLog` masks besides `authorization` and cookies.
- Slots, generated by the plugin and passed gofmt-formatted: `config_fields` (client-setting
  fields of `RequestConfig`), `config_clone` (the same fields in `RequestConfig.Clone`) and
  `client_options` (the `WithEnvironment*` and client-setting options appended to
  `option/requestoption.go`, starting with a blank line). gofmt aligns a struct block to its
  longest field name; a client setting longer than `RequestTimeout` would realign the runtime's
  own lines, which the template does not do.

Tokens: `__ferrule_module__`, `__ferrule_pkg__`, `__FerruleClient__`, `__FERRULE_ORG__`,
`__FERRULE_MAX_RETRIES__` (`NewRequestConfig`'s `MaxRetries: 2`; a non-default value is
unverified), `__FERRULE_SENSITIVE_HEADERS__`. Flags `module_before_io` and
`module_before_sjson` keep gofmt's import order in `field.go` and `option/requestoption.go`,
where the module's packages share an import group with `io` or `github.com/tidwall/sjson`
(the reversed order is in no oracle; it follows gofmt's sort by import path).

### File classification

| Files | Class | Notes |
|---|---|---|
| `internal/{apierror,apiform,apijson,apiquery,param,requestconfig,testutil}/**` (21 files, with their `_test.go`) | runtime | `requestconfig.go`: two slots |
| `option/{middleware,requestoption}.go` | runtime | `requestoption.go`: one slot |
| `field.go`, `default_http_client.go` | runtime | root package |
| `internal/version.go` | generated | version, like `src/version.ts` |
| `client.go`, `aliases.go`, `shared/**`, resource files, `api.md`, `examples/` | generated | |
| `packages/pagination/pagination.go` | generated | page types from the config (knock-go) |
| `packages/ssestream/ssestream.go` | not templated | SSE: only opencode-sdk-go (MIT) has it |
| `go.mod`, `go.sum`, `scripts/`, `lib/` | not runtime | project files and hand-written code |

### Oracles

| Oracle | Commit | Identical |
|---|---|---|
| onebusaway-go | `d115209ec6337f8d7ee52ad0cc8866a0005d6e65` (1.15.0, 2026-07-18), slots from its config | 25/25 |
| knock-go | `4b86a0d68f0ac65c1b1ccca2cca5b4893ee572ac` (1.5.0, 2025-06-06) | 11/25 |
| opencode-sdk-go (MIT, compared only) | `04c05a5d98bed833f9e8ed91616e6153880c1873` (0.1.0-alpha.28, 2025-07-29) | 11/25 |

knock-go and opencode-sdk-go are 2025 generations (`api` struct tags, no header redaction in
`WithDebugLog`, no `default_http_client.go`, older request code); `known_drift` in
`tests/test_render_oracles_go.py` lists the files. At knock-go's last Stainless commit
(`a6cf779a`, 2026-07-14, not pinned) 23 of 25 files are identical; the other two hold slots,
and knock's also carry its `idempotency_header` (a `github.com/google/uuid` import and an
`Idempotency-Key` header in `NewRequestConfig` and `Clone`), which the template does not
cover because no pinned oracle uses it.

## JVM (Java and Kotlin)

Stainless writes both the `java` and the `kotlin` target in Kotlin, with the same Gradle layout
(`<artifact>-core`, `<artifact>-client-okhttp`, `<artifact>`, `-example`, `-proguard-test`). One
template serves both: the Java target's Java-facing API sits in `//ferrule:if java` blocks and
the Kotlin target's own text in `//ferrule:if kotlin` blocks.

```python
from ferrule_runtime import JavaOptions, render_java_files

files = render_java_files(
    JavaOptions(
        package_name="com.acme.api",
        client_name="Acme",
        artifact_name="acme-java",
        repo="acme/acme-java",
    )
)  # {"acme-java-core/src/main/kotlin/com/acme/api/core/Values.kt": "...", ...}: Gradle root
```

`JavaOptions(package_name, client_name, artifact_name, repo, *, target="java",
log_env_var=None, array_format="comma", webhooks=False, empty_responses=False,
mock_tests=True)`:

- `package_name`: `reverse_domain`; every `package` line and the source directories.
- `client_name`: `<client>Exception` and the other error classes.
- `artifact_name`: the Gradle project prefix (`onebusaway-sdk-java`, `image-kit-java`): module
  directories and the ProGuard rules file `META-INF/proguard/<artifact>-core.pro`.
- `repo`: GitHub `owner/name`, linked from the Jackson version check in `Check.kt`.
- `target`: `java` keeps the `Optional` overloads, `@JvmStatic` / `@JvmSynthetic` /
  `@JvmOverloads` / `@file:JvmName` and the Java-only files (`AsyncStreamResponse.kt`,
  `PhantomReachableClosingAsyncStreamResponse.kt`, their test and `OkHttpClientTest.kt`);
  `kotlin` renders the Kotlin SDK's text (nullable types instead of `Optional`, no `@Jvm*`).
- `log_env_var`: read by `LogLevel.fromEnv`; default `<CLIENT_SCREAMING_SNAKE>_LOG`
  (`ONEBUSAWAY_SDK_LOG`, `IMAGE_KIT_LOG`).
- `array_format`: `query_settings.array_format`. `comma` (the default; imagekit) joins arrays in
  `QueryParams` and multipart bodies; `repeat` (OneBusAway) repeats the key. Other formats are
  in no 2026 oracle.
- `webhooks`: `core/UnwrapWebhookParams.kt` and `errors/<client>WebhookException.kt`.
- `empty_responses`: `core/handlers/EmptyHandler.kt`, used by methods with no response body.
- `mock_tests`: `TestServerExtension.kt` (off with `disable_mock_tests`).

Tokens: `__ferrule_artifact__`, `__ferrule_pkg_path__` (a directory), `__ferrule_pkg__`,
`__FerruleClient__`, `__FERRULE_LOG_ENV__`, `__FERRULE_REPO__`. Flags: `java`, `kotlin`,
`comma`. ktfmt sorts each import block by path, so the package's own imports move around
`java.*` and `kotlin.*` with its name (`io.imagekit` before them, `org.onebusaway` after): the
renderer re-sorts every block after rendering, the order all 1,723 import blocks of the four
oracles follow. Five `fits` blocks (ktfmt's 100 columns) hold the constructors of
`<client>IoException`, `InvalidDataException`, `RetryableException` and `WebhookException`
and the `Unexpected JsonNode type` throw; both forms are in the oracles (ImageKit fits,
OnebusawaySdk wraps).

### File classification

Decided by comparing every file of the four oracles after normalising artifact, package and
client names and sorting imports.

| Files | Class | Notes |
|---|---|---|
| `<artifact>-core/src/main/kotlin/<pkg>/core/**` (39 files) | runtime | `QueryParams.kt`, `HttpRequestBodies.kt`: array format |
| `<artifact>-core/src/main/kotlin/<pkg>/errors/**` (14 files) | runtime | |
| `<artifact>-core/src/test/kotlin/<pkg>/core/**`, `TestServerExtension.kt` (12 files) | runtime | tests of the runtime, like Go's `_test.go` |
| `<artifact>-core/src/main/resources/META-INF/proguard/<artifact>-core.pro` | runtime | |
| `<artifact>-client-okhttp/.../OkHttpClient.kt`, `OkHttpClientTest.kt` | runtime | |
| `core/ClientOptions.kt`, `core/ClientOptionsTest.kt`, `client/okhttp/<Client>OkHttpClient.kt`, `<Client>OkHttpClientAsync.kt` | generated | client settings (auth, environment variables, base URL) run through the builders, and ktfmt rewraps their KDoc by package length; like TypeScript's `src/client.ts` |
| `client/*`, `models/**`, `services/**`, `ProGuardCompatibilityTest.kt` | generated | |
| `core/AutoPager*.kt`, `core/Page*.kt` | generated | page types from the config's schemes (knock-java) |
| `build.gradle.kts`, `buildSrc/**`, the Gradle wrapper, `scripts/`, `-example`, `-lib` | not runtime | project files |

### Oracles

| Oracle | Commit | Options | Identical |
|---|---|---|---|
| onebusaway-java | `c29c48ad95f08e9a6042bfb4b620bb93b7a5f0ab` (0.1.0-alpha.60, 2026-08-04) | `repeat` | 64/65 |
| onebusaway-kotlin | `6923446778f92f8867c591064019eec24c685c1d` (0.1.0-alpha.81, 2026-07-18) | `kotlin`, `repeat` | 61/61 |
| imagekit-java | `f96c35e8d1e552eeee326e2c0a25a51436b36e37` (2026-05-17) | webhooks, empty responses, no mock tests | 67/67 |
| knock-java | `6e0df2347f66bbe9c7c284c352cc3466e081ad46` (2025-06-06, `generated` branch) | defaults | 28/65 |

onebusaway-java's `OkHttpClient.kt` carries a hand edit (a `println` `LoggingInterceptor`
added below the generated class), so the template takes that file from imagekit-java, whose
copy is otherwise identical up to names. knock-java is a 2025 generation: 37 files have older
text or do not exist yet (`Sleeper`, `LogLevel`, `LoggingHttpClient`, `ProxyAuthenticator`,
`<client>RetryableException`, the ProGuard rules; Apache HttpClient multipart bodies), listed
as `known_drift` in `tests/test_render_oracles_java.py`. Unverified: the Kotlin text for names
other than OneBusAway's (one Kotlin oracle), and wrapping that depends on names shorter than
`ImageKit` or longer than `OnebusawaySdk` outside the five `fits` blocks.

## Adding a language

1. Pick the newest Apache-2.0 Stainless SDK in that language among the oracles and list every
   file outside the generated resources. For each, compare it across all oracles of the same
   layout after normalising names (client, error, package, module, env prefix). A file is
   runtime when it matches up to those names; a difference that tracks a config option becomes
   a flag, token or slot; a difference that tracks the generation date is drift, recorded per
   oracle.
2. Copy the runtime files to `runtime/<language>/template/` unchanged, replace names with
   tokens and config-dependent lines with `<comment>ferrule:` directives. Never edit upstream
   text to pass a test; a directive branch that no oracle shows (such as Go's reversed import
   order) must follow a stated rule and be documented as unverified.
3. Add the name to `Language` in `_render.py` and `src/ferrule_runtime/_<language>.py` with an
   options dataclass and a renderer that calls
   `render_tree(template_root("<language>"), comment=..., ...)`; add the template to
   `[tool.hatch.build.targets.wheel.force-include]` in `pyproject.toml` and to the lint
   excludes in the root `pyproject.toml` (ruff, pyright) and `biome.json` (JS/TS/JSON).
4. Add `tests/test_render_<language>.py` (options) and `tests/test_render_oracles_<language>.py`
   (byte equality for every oracle, `known_drift` still drifting, every runtime file covered),
   reading checkouts from `oracles/cache/<name>` and skipping when they are absent.
5. Credit the source repository, commit and files in `NOTICE` and the root `NOTICE` and record
   the modification mark below, in the same commit as the template.

## Licence and modification marks

Every template file keeps its upstream text, header line included. Placeholders, directives and
slots are a modification under Apache-2.0 §4(b); an in-file mark would break byte parity of the
rendered output, so the mark lives here and in `NOTICE` (ADR 0004 amendment):

**Modified by The Ferrule Authors**:

- all files under `template/` (package, client and error names, log env var, timeout, max
  retries and file-upload hint replaced by placeholders; `_utils/_compat.py`, `_exceptions.py`,
  `_response.py` gained `#ferrule:` directive lines). `_streaming.py` contains code from
  httpx-sse and `_utils/_datetime_parse.py` from pydantic, as noted in their upstream headers;
- all files under `typescript/template/` (client and error names, max retries and the `qs` array
  format replaced by placeholders; `src/index.ts`, `src/internal/utils/log.ts` and
  `src/internal/utils/query.ts` gained `//ferrule:` lines; `query.ts` combines isaacus's and
  knock-mgmt-node's versions);
- all files under `go/template/` (module path, package, client and organisation names, max
  retries and the redacted-header list replaced by placeholders; `field.go` and
  `option/requestoption.go` gained `//ferrule:` import-order lines; `requestconfig.go` and
  `requestoption.go` had their config-dependent lines replaced by slots);
- all files under `java/template/` (artifact, package and client names, log env var and
  repository replaced by placeholders, also in directory and file names; the Java and Kotlin
  texts merged with `//ferrule:if java` / `kotlin` blocks; `QueryParams.kt`,
  `HttpRequestBodies.kt` and `HttpRequestBodiesTest.kt` gained comma-format blocks from
  imagekit-java; the error classes and `HttpRequestBodies.kt` gained `fits` blocks;
  `OkHttpClient.kt`, `UnwrapWebhookParams.kt`, `EmptyHandler.kt` and
  `<client>WebhookException.kt` come from imagekit-java).

MIT-licensed oracles (opencode-sdk-js, opencode-sdk-go) are compared by the tests but never
copied; stagehand and ogx (MIT) are not used.
