# ferrule-runtime

The Python runtime copied into every Ferrule-generated SDK, plus the code that renders it.

- `template/`: the runtime of the Stainless-generated OneBusAway Python SDK
  (`OneBusAway/python-sdk` @ `4c20fa834c2a1237ef046e4894af60cd02fb3bcf`, release 1.30.0,
  Apache-2.0), with names replaced by placeholders. Third-party code: not linted by our rules.
- `src/ferrule_runtime/`: `render`, `render_files` and the `pagination.py` generator.

## API

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

## Template format

Placeholders: `__ferrule_pkg__` (package), `__FerruleClient__` (client class; also
`Async__FerruleClient__`), `__FerruleError__` (base exception, default `<Client>Error`),
`__FERRULE_LOG_ENV__` (default `<CLIENT_SCREAMING_SNAKE>_LOG`), `__FERRULE_TIMEOUT__`,
`__FERRULE_TIMEOUT_COMMENT__`, `__FERRULE_MAX_RETRIES__`, `__FERRULE_FILE_UPLOADS_HINT__`.
Config-dependent blocks use whole-line directives `#ferrule:if <flag>` / `#ferrule:else` /
`#ferrule:endif`, removed on render. Flags: `py39` (`min_python < 3.10`), `webhooks`,
`base_model_hint_fits` (ruff-format line length depends on the package name length).

## File classification

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

## Version decision and drift

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

## Licence and modification marks

Every template file keeps its upstream text, header line included. The placeholders and
directives are a modification under Apache-2.0 §4(b); an in-file mark would break byte parity
of the rendered output, so the mark lives here and in `NOTICE` (ADR 0004 amendment):

**Modified by The Ferrule Authors**: all files under `template/` (package, client and error
names, log env var, timeout, max retries and file-upload hint replaced by placeholders;
`_utils/_compat.py`, `_exceptions.py`, `_response.py` gained `#ferrule:` directive lines).
`_streaming.py` contains code from httpx-sse and `_utils/_datetime_parse.py` from pydantic, as
noted in their upstream headers.

## Testing fake server

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
