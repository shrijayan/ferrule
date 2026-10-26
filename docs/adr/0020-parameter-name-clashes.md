---
status: Proposed
date: 2026-10-26
deciders: Ferrule maintainers
---

# 0020. Parameter names that clash in a generated method

## Context and problem statement

A generated method takes its path, query, header and body parameters as Python arguments, next to
the per-request options `extra_headers`, `extra_query`, `extra_body` and `timeout`, after `self`.
Real specs give two of these the same name: github's path `name` and body `name`, kubernetes'
query `dryRun` and `DeleteOptions.dryRun`, docker's query `timeout`, twilio's body `Timeout`, a
jira body field `self`. Python rejects a function with a duplicate argument, so the whole module
fails to import (Tier 1 corpus run, 2026-10-26: about 24,000 failing tests). Which names does the
SDK use instead?

## Decision drivers

- Stainless output is the spec: its SDKs show the rule wherever it has one.
- Keep the wire name: the request on the wire must not change.

## Considered options

1. Stainless's rules, with `self` treated like a request option.
2. Fail with a FER diagnostic and ask for an `openapi.transforms` rename.
3. Suffix with `_` (`timeout_`), as for Python keywords.

## Decision outcome

Chosen option 1.

- A name two locations share is prefixed with each location, `path_`, `query_`, `header_` or
  `body_`; the path template and `TypedDict` alias use the wire name. Evidence: gcore-python
  `dns.zones.replace(path_name, *, body_name, ...)` (template `/dns/v2/zones/{path_name}`,
  `body_name: Required[Annotated[str, PropertyInfo(alias="name")]]`); cloudflare-python
  `resource_sharing.recipients.create(share_id, *, path_account_id, body_account_id, ...)`.
- A parameter named like a request option is `api_<name>`. Evidence:
  `api_timeout: Annotated[int, PropertyInfo(alias="timeout")]` for a body field in
  browserbase-python `session_create_params.py` and for a query parameter in parallel-sdk-python
  `task_run_result_params.py`; telnyx-python `v1_messages_params.py`.
- `header_` and `self` are Ferrule's choice: no oracle SDK shows either. `self` is a name the
  method already binds, like the request options, so it is `api_self`.
- An `allOf` that declares a property in several members gives one parameter, typed by the last
  member (openai `CreateChatCompletionRequest.top_logprobs`); that is a fix, not a choice.

### Consequences

- Good: every Tier 1 spec that hit this imports; names match the Stainless SDKs we can see.
- Bad: the `header_` prefix and `api_self` may differ from Stainless if it has its own rule; a
  user can still rename with an `openapi.transforms` entry.

## More information

`crates/python-plugin/tests/location_clash/`, `request_option_clash/` and `allof_merge/`, pinned
by `crates/python-plugin/tests/param_clashes.rs`; `site/src/content/docs/docs/corpus-findings.md`
item 1.
