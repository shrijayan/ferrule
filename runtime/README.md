# ferrule-runtime

The runtimes copied into every Ferrule-generated SDK, one per target language, plus the code
that renders them. Each runtime is the runtime of a Stainless-generated SDK (Apache-2.0) with
per-SDK names replaced by tokens; rendering is pure and byte-identical to the oracle's files.

| Language | Template | Renderer | Source (Apache-2.0) |
|---|---|---|---|
| Python | `template/` | `render_files`, `render` | `OneBusAway/python-sdk` @ `4c20fa83` |
| TypeScript (`typescript` target, current layout) | `typescript/template/` | `render_typescript_files` | `isaacus-dev/isaacus-typescript` @ `c56af182`, `src/internal/qs/` from `knocklabs/knock-mgmt-node` @ `ca88fdba` |

- Templates are third-party code: not linted or formatted by our rules (excluded in the root
  `pyproject.toml` and `biome.json`).
- `src/ferrule_runtime/`: the renderers, the shared engine `render_tree` and the Python
  `pagination.py` generator. Python's template stays at `template/` (not `python/template/`)
  because the wheel layout and ADRs 0004 and 0016 refer to that path.

All languages share one engine, `render_tree(root, *, replacements, flags, comment, slots,
gated)`: whole-line directives `<comment>ferrule:if <flag>` / `else` / `endif` and
`<comment>ferrule:slot <name>` (`#` for Python, `//` for TypeScript), then token
replacement, then a check that no `__ferrule_*__` / `__Ferrule*__` / `__FERRULE_*__` token is
left. `gated` maps a path prefix to whether files under it are rendered. A slot line is
replaced by text the caller supplies (empty removes it); a malformed directive is an error.

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
| imagekit-nodejs | `9161f99f561250b11b4535a328469bb5737cb4b8` (2026-05-17) | defaults | 33/33 |
| knock-mgmt-node | `ca88fdba703717f355ad3e18e850a0bb23e548f6` (0.32.0, 2026-07-09) | `comma`, pagination | 40/40 |
| knock-node | `e2c0f18ad31244906b8b7885d6d52bc51e35586b` (1.7.0, 2025-06-06) | `brackets`, pagination, no extra headers | 24/40 |
| papr-typescript | `41dbb3b3432a3b7d08bb1a09527a2e83db93fd39` (1.20.0, 2025-12-07) | papr's headers | 27/33 |
| opencode-sdk-js (MIT) | `7149e6a0575bc4e5b90489f31f509af7b226e2ec` (2025-09-03) | no extra headers | 25/33 |

The three 2026 oracles are byte-identical. The other differences are older Stainless
generations (missing `/* @__PURE__ */` annotations, older `env.ts`, `parse.ts`, `types.ts`,
no `utils/query.ts`), listed per file as `known_drift` in
`tests/test_render_oracles_typescript.py`. At knock-node's last Stainless commit
(`b7cdf1a4`, 2026-07-14, not pinned) 38 of 40 files are identical; the other two are knock's
hand edits (`utils/query.ts` re-exports `src/lib/stringifyQuery.ts`, `index.ts` exports
`src/lib`).

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
  knock-mgmt-node's versions).

MIT-licensed oracles (opencode-sdk-js) are compared by the tests but never copied;
stagehand and ogx (MIT) are not used.
