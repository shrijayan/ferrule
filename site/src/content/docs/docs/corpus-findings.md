---
title: What the real-world specs taught us
description: Every problem the Tier 1 corpus run found in Ferrule, with the evidence for it, what was fixed and what is still open.
---

The [real-world specs](../real-world/) run puts fourteen large public specs through Ferrule end to
end. This page records what that run found, with the evidence behind each point: the specs it
affects, the counts from the run's reports, and an excerpt of the generated code. Items are
ordered by how much they cost, measured in generated tests that fail or operations that are
skipped. Each one names the part of Ferrule that owns it.

The numbers come from the run of **2026-10-26**
(`ferrule-gen 0.0.0`, Forge `91f52a40`). Re-running `mise run corpus -- --tier 1` reproduces them.
Every report records the most frequent causes of each failing stage, and the board shows them for
each spec.

## Fixed while running the corpus

Each fix was made test-first: a minimal fixture reduced from the failing spec, committed with
a failing test, then the fix.

| Problem | Evidence | Fix |
| --- | --- | --- |
| A freshly generated SDK did not import when its package name differs from its client name. The runtime defined `CorpusSlackError`; `__init__.py` imports `SlackError`. | slack: `ImportError: cannot import name 'SlackError'`. Any user whose `package_name` is not the client name in lower case would hit it. | `ferrule generate` reads the client class from the generated `_client.py` and names the runtime after it (migrate). |
| Operations that `stainless.yml` leaves out were still generated, then refused with FER103 (no `x-ferrule-resource`). | cloudflare: its spec carries `x-fern-sdk-group-name` on every operation and `x-forge-aliases` on 44, which Forge reads as "generate this" (gap G1). | The converter removes Fern grouping and spec-supplied Forge aliases from operations no method is bound to, so Forge drops them as Stainless does (converter). |
| The Jira spec failed its sha256 check. | Atlassian replaced the file in place: same size, new hash, `info.version` `1001.0.0-SNAPSHOT-12ca6ec5`. | Re-pinned in `corpus/manifest.yaml`. |

## Open

### 1. Generated methods with a parameter twice (python-plugin)

**Cost: by far the largest.** A Python function with a duplicate parameter is a `SyntaxError`.
Every test that imports the module fails, not just the tests of that method.

| Spec | Duplicated name | Where both come from | Tests failing with this `SyntaxError` |
| --- | --- | --- | --- |
| github | `name` | path parameter and request-body field | 10,874 of 22,428 |
| openai | `top_logprobs` | the same property in two `allOf` members (one nested) | 7,190 of 7,710 |
| twilio | `timeout` | request-body field and the per-request `timeout` option every method has | 4,377 of 5,015 |
| kubernetes | `dry_run`, `grace_period_seconds`, … | query parameter and `DeleteOptions` body field | 540 of 1,262 (and 3,500 ruff errors) |
| docker | `timeout` | query parameter and the per-request `timeout` option | 1,423 of 2,666 |
| jira | `self`, `id` | a body field named `self` (the method's own `self`) and a path `id` with a body `id` | not run: the SDK does not import (item 5) |
| cloudflare | `account_id`, `act`, `name` | path parameter and body field, and `allOf` members | not run: the SDK does not import (item 5) |

```py
# openai, src/openai/resources/chat.py (generated)
        top_logprobs: Optional[int] | Omit = omit,
        top_logprobs: int | Omit = omit,
        top_logprobs: Optional[int] | Omit = omit,

# github, src/corpus_github/resources/actions.py (generated)
    def actions_update_environment_variable(
        self,
        name: str,
        *,
        ...
        name: str | Omit = omit,
```

Three different fixes are needed. An `allOf` property should be merged into one parameter. A
clash between a path or query parameter and a body field needs Stainless's renaming rule. So does
a clash with a request option such as `timeout`. The renaming rule has to be confirmed against the
oracle SDKs, and the change approved as a golden-output change.

### 2. Form-encoded request bodies (python-plugin and runtime; Forge gap G21)

The spec says `application/x-www-form-urlencoded`. The SDK either sends JSON or cannot generate
the operation at all.

- slack: all **1,670** failing tests are the mock server answering
  `Content-Type "application/json" is not accepted. Expected: application/x-www-form-urlencoded`.
- twilio: 20 tests fail the same way. The rest are hidden behind item 1.
- stripe: **298 of 612** operations are skipped with FER201,
  `a request body that is not an object is not generated yet`. Forge types every non-JSON body as
  `unknown` ([G21](https://github.com/shrijayan/ferrule/blob/main/docs/forge-gaps.md)).

### 3. Shapes the plugin does not generate yet (FER201, python-plugin)

The run skips these operations and counts them under their code. What it skipped, by the
plugin's own message:

| Message | Specs (operations) |
| --- | --- |
| a model with two properties that both become the same attribute (e.g. `id` and `id`) | cloudflare (611), openai (13), plaid (10) |
| a request body that is not an object (see item 2) | stripe (298), cloudflare (14), jira (14), github (13), docker (5), discord (4), openai (1) |
| an `allOf` member of kind primitive / union / literal / map / array | cloudflare (175 / 70 / 27 / 7 / 1), plaid (1 / 7 / 1 / 37 / 0), jira (0 / 10 / 0 / 0 / 0), github (0 / 3 / 0 / 0 / 0), openai and docker (1 union each) |
| HTTP method `HEAD` / `OPTIONS` | kubernetes (6 / 6), docker (2 / 0) |

Recursive schemas are no longer skipped. As its diagnostic suggests, the run names them as models
in the synthesized config: 34 for stripe, 13 for cloudflare, 4 for kubernetes and 2 for openai.

### 4. The same nested class emitted twice (python-plugin)

Two inline objects that get the same generated name are both emitted, and the second hides the
first. pyright reports `reportRedeclaration` and mypy reports `no-redef`. Counts (pyright): github
130, jira 75, cloudflare 55, openai 41, square 9, slack 6, docker 2.

### 5. Generated SDKs that do not import or collect (python-plugin)

**jira** and **cloudflare** do not import. A property whose name starts with a digit (jira
`16x16`, cloudflare `1.1.1.1`) becomes an attribute with a leading underscore (`_16x16`,
`_1_1_1_1`), and pydantic refuses those fields (`NameError: Fields must not use names with
leading underscores`). The name needs a prefix instead, which has to be checked against
Stainless's rule.

**stripe**'s tests do not collect.

pytest stops while collecting, with `RecursionError: maximum recursion depth exceeded` inside
pydantic's schema generation, so no stripe test runs. This needs reducing to a fixture: a
recursive model graph that pydantic cannot build eagerly.

### 6. Integer path parameters with value 0 are rejected (python-plugin)

github: 480 tests fail with `ValueError: Expected a non-empty value for 'pull_number' but received 0`;
discord: 20 the same way (`answer_id`).
The empty-value check that Stainless applies to string path parameters also runs on integers,
and the generated test uses `0`.

### 7. Responses the SDK rejects (needs triage)

`APIResponseValidationError: Data returned by API invalid for expected schema` appears for github
(429), square (99) and plaid (96). The mock server's response data and the generated models
disagree. Each case still has to be checked against the spec. In plaid, 25 more tests fail
with `TypeError: Object of type date is not JSON serializable`.

### 8. Specs split over several files (converter or CLI; gaps G29 and G32)

**azure-storage** stops in generate. Its 713 `$ref`s point into other files
(`../../../../../../common-types/...`), and reading other documents is not something Forge
does.

**digitalocean** looks complete but is not. All 858 of its operations are written as
`get: {$ref: 'resources/....yml'}`, a reference to a sibling file that is not fetched. Forge reads
each one as an operation with no parameters, body or responses, without a diagnostic (new gap
G32, upstream issue drafted). Generation succeeds with 858 empty methods. Each generated test
then makes the mock server fail (`Cannot read properties of undefined (reading '200')`, an
operation without `responses`). The SDK retries every one of those 500 errors, so the suite hit
the one-hour limit at 78 %.

A bundling step before convert fixes both: it resolves `$ref`s into other files from
`source.repo` at `source.sha`. The harness now counts external `$ref`s per spec
(`counts.external_refs`), so a run flags this automatically.

### 9. Tooling limits

pyright crashes (exit 250, no error count) on the kubernetes SDK: 2,472 files and 137 MB. The
board shows that check as failed with an unknown count.

## Comparison with official SDKs

These are the three APIs with a Stainless-generated Python SDK. Their `stainless.yml` files are
private, so the synthesized config cannot reproduce their names. The API diff is therefore large
by construction, and endpoint coverage is the meaningful number:

| API | Official SDK (pinned) | Official endpoints generated |
| --- | --- | --- |
| OpenAI | `openai/openai-python` @ `f83b1f4` | 260 of 275 (274 are in the public spec) |
| DigitalOcean (Gradient) | `digitalocean/gradient-python` @ `32b6f06` | 188 of 193 (188 are in the public spec); paths only, since the methods are empty (item 8) |
| Cloudflare | `cloudflare/cloudflare-python` @ `de12426` | 1,456 of 2,423 (2,313 are in the public spec); 905 of the spec's 3,767 operations are skipped (item 3) |

## Forge

Apart from G32, the open items above are Ferrule's own. The Forge limits the run hit are
recorded in [`docs/forge-gaps.md`](https://github.com/shrijayan/ferrule/blob/main/docs/forge-gaps.md),
and each has a workaround: Swagger 2.0 (G24), query styles (G25), equal names (G26), dangling
`$ref`s (G27), tuple `items` (G28), titled union members (G30), nullable-only members (G31) and
non-JSON bodies (G21). G29 (`$ref`s into other files) stops azure-storage. The run found
one new gap, G32: an Operation Object written as a `$ref` is read as empty without a
diagnostic, as for digitalocean. Its upstream issue is drafted in `docs/HUMAN-ACTIONS.md`.
