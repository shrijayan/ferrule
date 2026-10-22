# Stainless research for Ferrule (as of 2026-10-22)

## 1. Status
- **Confirmed:** Anthropic announced the acquisition on 18 May 2026. Price undisclosed [2]. Stainless is "winding down all hosted Stainless products, including our SDK generator", and new signups, projects and SDKs closed that day [1][3].
- **Existing customers:** they own the SDKs they generated and can modify them freely. They are pointed to app.stainless.com/transition and transition@stainless.com [1][5].
- **Deadline:** no official end date has been published [3]. One outlet claims shutdown on 1 Sep (single-source) [4]. That is contradicted by GitHub: stainless-app[bot] pushed "feat(api)" commits on 22–23 Sep and releases on 16 Oct.
- **stlc (key finding):** eligible customers get a self-hosted generator CLI.
  - It lives in the private repos `stainless/stlc` and `stlc-python` etc., installs via npm, and needs an org admin to accept an "stlc license" [6] (single source: Knock's mirror of Stainless transition docs).
  - It runs locally and in CI, supports custom code and uses plain release-please [5][6].
  - Groq, Orb, Lithic, Kernel and Runway use it, per their public workflow files [7].
- **Config format and open source:** no statement on either, but the config JSON Schema is public [9].
- **Landscape:**
  - stainful: MIT, Python-only, reads stainless.yml, inactive since 22 May [17].
  - Cloudflare Forge: Apache-2.0; Python is only a "future target" [18].
  - Speakeasy: generator released under AGPL-3.0 [19].
  - OpenAI: moved off Stainless on 12 Aug to "Castiron" (in-house, inferred) [15].

## 2. stainless.yml [8][9]
The schema requires `organization`, `environments`, `resources` and `readme`.
- `edition`: dated config edition (latest is `2026-05-06`). Per-target editions also exist; `python.2025-11-20` switches from Rye to uv [10].
- `organization`: name, docs, contact, github_org.
- `targets.python`: package_name, project_name, production_repo, publish.pypi, keep_files, skip, and options (python_version_range, use_uv, websockets/standardwebhooks extras).
- `environments`: maps a name to a base URL; the first entry is the default.
- `resources`: the namespace tree, with special keys `$client` and `$shared`.
  - `models`: schema refs, deduplicate, alias.
  - `methods`: shorthand `get /x`, or an object with paginated, streaming, positional_params, body_param_name, unwrap_response.
  - `subresources`.
- `client_settings`: opts (read_env, auth), default_client_name, default_env_prefix, default_headers, default_timeout, default_retries, idempotency, omit_stainless_headers.
- `pagination`: named schemes (cursor, cursor_id, cursor_url, offset, page_number, fake_page) with request/response mappings. `list_*` methods are matched automatically [11].
- `streaming.on_event`: SSE/JSONL event handlers.
- `readme`: example_requests, example_types, include_stainless_attribution.
- `settings`: license, file_header, unwrap_response_fields.
- `openapi`: transforms (update, append, merge, remove, copy, move) and code_samples.
- Also: query_settings, multipart_settings, security, security_schemes, unspecified_endpoints, custom_casings, constants, codeflow, diagnostics.

**Public examples:** OneBusAway/sdk-config [21], which pairs with the public OneBusAway/python-sdk and makes an ideal golden fixture.

## 3. Python SDK anatomy [13][14]
- **License:** all 10 repos checked are Apache-2.0. The copyright line names the customer, for example "Copyright 2026 Groq".
- **Package layout:** `src/<pkg>/` holds `_client`, `_base_client`, `_models`, `_types`, `_response`, `_streaming`, `_exceptions`, `_files`, `_qs`, `_compat`, `_constants`, `pagination.py`, `_utils/`, `resources/` (Sync and Async pairs), `types/` (one file per model or params type) and `lib/.keep`.
- **Repo files:** `tests/api_resources`, `api.md`, `.stats.yml`, release-please config and manifest, `scripts/{bootstrap,lint,mock,test}`, `bin/publish-pypi`, `.devcontainer`, Brewfile, noxfile, lockfiles.
- **File header:** generated files start with "# File generated from our OpenAPI spec by Stainless. See CONTRIBUTING.md for details." Core runtime files have none.
- **Dependencies:** httpx<1, pydantic v1 and v2, typing-extensions, anyio, distro, sniffio, plus an optional aiohttp extra. Python >=3.9 (Groq requires 3.10). Built with hatchling.
- **Tests:** pytest (xdist, asyncio), respx, time-machine, dirty-equals. The mock server is Prism in older repos and Steady (MIT) [23] in newer ones.
- **Lint:** ruff, strict pyright and mypy.
- **Conventions that make it look hand-written:**
  - Sentinels: `NotGiven`/`not_given`/`NOT_GIVEN` and `Omit`.
  - Per-call `extra_headers`/`extra_query`/`extra_body`/`timeout`.
  - `.with_raw_response` and `.with_streaming_response` on every resource.
  - Params as TypedDicts; responses as pydantic models; enums as `Literal` [10].
  - Auto-iterating `Sync/Async*Page` classes.
  - Retries: 2 by default, 0.5–8s backoff, honours `retry-after`, retries on 408/409/429/5xx.
  - 60s timeout, `X-Stainless-*` headers, typed exception hierarchy, `<PREFIX>_API_KEY` env var.

## 4. Regeneration and custom code
- **Hosted [12]:**
  - The `generated` branch holds pure output; the integrated branch is `main`, or `next` once a production repo is linked.
  - Each generated commit is cherry-picked onto the integrated branch with a semantic three-way merge, falling back to an AST-based merge.
  - A failed merge opens a conflict PR and pauses updates.
  - The generator never touches `lib/` or `examples/`.
- **Release:** release-please opens "release: x.y.z" PRs from `next` into `main` [13]. "codegen metadata" commits update `.stats.yml` (configured_endpoints, openapi_spec_url, openapi_spec_hash, config_hash) [22].
- **stlc:**
  - Each generated commit carries a `Stainless-Generated-From: <sha>` trailer [16].
  - Custom code is "sealed" into JSON tracking files in a config repo.
  - Code flows config → staging → promote → prod `next` → release → back-sync [6][7].

## 5. Beyond SDKs
- **Targets:** TS, Go, Java, Kotlin, Ruby, PHP, C#, CLI, Terraform, SQL and an OpenAPI target with code samples [8].
- **Also:** MCP servers (a TypeScript subpackage) [20], a hosted Docs Platform and a local docs preview.
- **Gap:** no maintained open-source tool targets byte-level Python parity.

## What Ferrule must replicate to reach output parity
1. Schema-validated config, applying edition defaults.
2. `openapi.transforms`.
3. Resource tree → Sync/Async classes, including `$client`/`$shared` and subresources.
4. Model naming, dedup and alias rules, one-file-per-type layout and re-exports.
5. TypedDict params, `Literal` enums, unions, `SequenceNotStr`.
6. A model layer compatible with pydantic v1 and v2.
7. A byte-identical runtime (`_base_client`, `_response`, `_streaming`, `_types`, `_qs`, `_utils`).
8. Retry, timeout and connection defaults; idempotency and `X-Stainless-*` headers.
9. Pagination schemes and the `list_*` heuristic.
10. SSE/JSONL streaming.
11. Client opts, env prefix, environments and security schemes.
12. Query and multipart serialization.
13. File uploads, webhooks and websocket extras.
14. Spec docstrings and the file header (configurable).
15. README sections, example requests and `api.md`.
16. Generated tests plus the mock and test scripts.
17. `pyproject.toml` in Rye or uv flavour, chosen by edition.
18. ruff formatting, so whitespace is identical.
19. CI and publish workflows plus release-please files.
20. `.stats.yml`.
21. Custom-code preservation (generated branch plus three-way merge, `lib/`, `keep_files`).
22. Deterministic output, checked by golden diffs against OneBusAway, Groq and Lithic at their last Stainless commit.

## Legal angle
- The generated repos are Apache-2.0, and Stainless says customers own them [1]. Reusing the runtime is therefore permitted if Ferrule keeps the license and notices, marks files it modifies, and does not use Stainless trademarks.
- The residual risk is chain of title: Stainless authored the runtime. I did not review Stainless's terms of service (unverified).
- **Best option, "adopt" mode:** copy the runtime from the migrating customer's own repo, which they own. That gives exact parity with no third-party licensing question. As a default, ship a runtime derived from one Apache-2.0 repo, with a NOTICE.
- **Clean-room runtime:** only needed to relicense the code, and it is unlikely to come out byte-identical.
- **Hard rule:** keep anyone who has seen stlc source out of Ferrule; stlc has its own license [6].
- Not legal advice.

## Sources
[1] https://www.stainless.com/blog/stainless-is-joining-anthropic
[2] https://techcrunch.com/2026/05/18/anthropic-has-acquired-the-dev-tools-startup-used-by-openai-google-and-cloudflare/
[3] https://scalar.com/resources/stainless-wind-down
[4] https://byteiota.com/cloudflare-forge-is-open-source-replace-stainless-now/
[5] https://news.ycombinator.com/item?id=48182281
[6] https://github.com/knocklabs/stainless-workflows/tree/main/docs/transition-guide
[7] https://github.com/kernel/hypeman/blob/main/docs/runbooks/sdk-generation-stlc.md
[8] https://www.stainless.com/docs/reference/config/
[9] https://app.stainless.com/config.schema.json
[10] https://www.stainless.com/docs/sdks/python/
[11] https://www.stainless.com/docs/sdks/configure/pagination/
[12] https://www.stainless.com/docs/sdks/configure/custom-code/
[13] https://github.com/groq/groq-python
[14] https://github.com/lithic-com/lithic-python
[15] https://github.com/openai/openai-python/commit/a1eeab58
[16] https://github.com/anthropics/anthropic-sdk-python/blob/main/CLAUDE.md
[17] https://github.com/stainlu/stainful
[18] https://github.com/cloudflare/forge
[19] https://github.com/speakeasy-api/openapi-generation
[20] https://www.stainless.com/docs/mcp
[21] https://github.com/OneBusAway/sdk-config
[22] https://github.com/perplexityai/perplexity-py/blob/main/.stats.yml
[23] https://github.com/dgellow/steady
