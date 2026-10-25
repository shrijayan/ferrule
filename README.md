<p align="center">
  <a href="https://shrijayan.github.io/ferrule/">
    <img src="site/public/favicon.svg" width="72" height="72" alt="Ferrule">
  </a>
</p>

<h1 align="center">Ferrule</h1>

<p align="center">
  <strong>Change the engine. Not the SDK.</strong><br>
  The open-source way off Stainless: the same Python SDK your users already import,<br>
  regenerated from the same spec and <code>stainless.yml</code>, with proof that nothing public changed.
</p>

<p align="center">
  <a href="https://shrijayan.github.io/ferrule/"><img alt="Website" src="https://img.shields.io/badge/site-shrijayan.github.io%2Fferrule-d6a660?style=flat-square&labelColor=0b0b0d"></a>
  <a href="https://github.com/shrijayan/ferrule/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/shrijayan/ferrule/ci.yml?branch=main&style=flat-square&labelColor=0b0b0d&label=ci"></a>
  <a href="./LICENSE"><img alt="Apache-2.0" src="https://img.shields.io/badge/licence-Apache--2.0-d6a660?style=flat-square&labelColor=0b0b0d"></a>
  <img alt="Pre-alpha" src="https://img.shields.io/badge/status-pre--alpha-8a5c2c?style=flat-square&labelColor=0b0b0d">
  <a href="./CONTRIBUTING.md"><img alt="DCO, no CLA" src="https://img.shields.io/badge/DCO-never%20a%20CLA-d6a660?style=flat-square&labelColor=0b0b0d"></a>
</p>

<p align="center">
  <a href="https://shrijayan.github.io/ferrule/docs/quickstart/"><b>Quickstart</b></a> ·
  <a href="https://shrijayan.github.io/ferrule/docs/">Docs</a> ·
  <a href="https://shrijayan.github.io/ferrule/docs/commands/">Commands</a> ·
  <a href="https://shrijayan.github.io/ferrule/docs/fidelity/">Fidelity board</a> ·
  <a href="./docs/plan.html">Build plan</a>
</p>

<p align="center">
  <a href="https://shrijayan.github.io/ferrule/">
    <img src=".github/assets/hero.jpg" alt="Ferrule landing page: your config unchanged, the engine replaced, your users' code unchanged" width="100%">
  </a>
</p>

---

## Your generator is going away. Your users aren't.

Thousands of developers import the SDK you shipped. Every class name, every keyword argument,
every `with_raw_response` is a promise. When the generator behind it goes away, you have three
options:

| | Option | What happens to your users |
| :-: | --- | --- |
| ✕ | Switch to another generator | It has its own house style. Names move, params change shape, and your users find out from a stack trace. |
| ✕ | Freeze the SDK and hand-maintain it | Fine until the API ships a new endpoint. Then the SDK drifts away from the spec it came from. |
| ✓ | **Ferrule: keep the SDK, swap the engine** | **Nothing changes for them. That's the point.** |

## Not "compatible". Identical.

Here is the complete diff between a file from the last Stainless release of the
[OneBusAway Python SDK](https://github.com/OneBusAway/python-sdk) and Ferrule's output, generated
from the same spec and the same config:

```diff
  # src/onebusaway/resources/current_time.py
- # File generated from our OpenAPI spec by Stainless. See CONTRIBUTING.md for details.
+ # File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.
```

The only line that differs is the one that names the generator. Across the whole SDK:

| **0** | **1,110** | **80 / 80** | **byte-for-byte** |
| :-: | :-: | :-: | :-: |
| breaking changes to the public API | of the SDK's own tests pass on Ferrule's output | generated code files identical | the same output on every run |

## How it works

Ferrule doesn't build another generator. [Forge](https://github.com/cloudflare/forge),
Cloudflare's Apache-2.0 engine, parses the spec. Ferrule adds everything Forge doesn't do: reading
your config, writing Stainless-shaped Python, and proving the result.

```mermaid
flowchart LR
  C["<b>01 Convert</b><br/>read stainless.yml"] --> F["<b>02 Forge</b><br/>parse the spec"]
  F --> E["<b>03 Emit</b><br/>write the Python"]
  E --> FI["<b>04 Finish</b><br/>runtime, ruff, patches"]
  FI --> V["<b>05 Verify</b><br/>public API diff"]

  classDef step fill:#17171b,stroke:#d6a660,color:#f6e2b4
  classDef forge fill:#2a1f12,stroke:#f6e2b4,color:#f6e2b4
  class C,E,FI,V step
  class F forge
```

**In:** `openapi.yml`, `stainless.yml`, your hand edits, your last release.
**Out:** the SDK your users already import, plus `ferrule.lock` and a verify report.

## Quickstart

> [!IMPORTANT]
> Ferrule is **pre-alpha** and not on PyPI yet. The 0.1 release is planned to make this
> `uvx ferrule adopt .`; until then, build it from source.

```bash
git clone https://github.com/shrijayan/ferrule && cd ferrule
mise install && mise run install            # toolchain and dependencies
mise run forge:setup                        # build the pinned Forge commit
pnpm --filter @ferrule/python-plugin build  # build plugin.wasm
export FERRULE_FORGE="$PWD/.cache/forge/target/release/forge"
export FERRULE_PLUGIN="$PWD/python-plugin/dist/plugin.wasm"

uv run ferrule adopt ~/code/acme-python --config ~/code/acme-config/stainless.yml
```

```text
✓ read .stats.yml → spec URL + hash
✓ rescued spec to ./openapi.json (hash ok)
✓ converted stainless.yml → overlays
✓ copied runtime from your repo (you own it)
✓ found 3 hand edits → .ferrule/patches/
✓ generated via forge + ferrule-python
✓ committed 'Switch SDK generation to Ferrule' on local branch ferrule/adopt (not pushed)
ferrule verify --against 1a2b3c4
public API: 0 breaking · 6 additive
```

The full walkthrough, from prerequisites to what to do on an error, is in the
[quickstart](https://shrijayan.github.io/ferrule/docs/quickstart/).

## What happens next

1. **Adopt locally.** `adopt` works in a temporary git worktree, so your checkout is never touched.
2. **Review one commit.** The branch `ferrule/adopt` holds the rescued spec, your patches,
   `ferrule.lock` and a workflow. Generated files differ by one header line.
3. **Push and let CI test it.** Your existing test suite runs against the regenerated code.
4. **Merge and release.** Nothing your users import changed, so it ships as a patch release.
5. **Stay current.** The workflow regenerates on every spec change and opens a PR labelled
   `major`, `minor` or `patch`. A breaking change needs a human's label to pass.

## What you get

- **Proof that nothing broke.** `ferrule verify` diffs the public API against your last release,
  reading the code with griffe and never importing it.
- **Your spec, rescued.** Every Stainless repo's `.stats.yml` points at a spec in a bucket you
  don't control. `ferrule rescue` saves it and checks the hash. Worth running today.
- **Output you own.** No licence obligations on generated code, and the SDK never depends on
  Ferrule or Forge. In adopt mode the runtime is copied from your own repository.
- **Hand edits that survive.** Custom changes become patches, re-applied three-way on every
  regeneration. `lib/` and `keep_files` are never touched.
- **A GitHub Action that keeps it current.** It regenerates, verifies and opens a labelled PR.
- **A public [fidelity board](https://shrijayan.github.io/ferrule/docs/fidelity/).** Real public
  Stainless SDKs, regenerated every night, scores published, including the ones that aren't green
  yet.
- **Coming in phase 5:** agent docs (`SKILL.md`, `AGENTS.md`, `llms.txt`) inside every wheel, and a
  fake server in every SDK so code runs without an API key.

## Compared with the alternatives

| | **Ferrule** | Open-core generator (free tier) | Hosted platform | Freeze and hand-maintain |
| --- | --- | --- | --- | --- |
| Reads your `stainless.yml` as-is | ✓ | ✕ | Partly | n/a |
| Same Python your users import | ✓ identical | ✕ its own style | Close | ✓ until the spec changes |
| Proof nothing broke | ✓ `ferrule verify` | ✕ | ✕ | ✕ |
| Licence of generated code | ✓ yours | ✕ copyleft on the free path | Vendor terms | ✓ yours |
| Account, token or login | ✓ none | ✕ required | ✕ required | ✓ none |
| Runs offline, in your CI | ✓ | With a key | ✕ hosted | ✓ |
| Keeps up with the spec | ✓ labelled PRs | ✓ | ✓ | ✕ |

<sub>Summary of public terms as of October 2026. Check each vendor's current terms before deciding.</sub>

## Open by design

Ferrule will never ask for ~~hosted generation~~, ~~licence tokens~~, ~~logins~~,
~~telemetry~~, ~~a CLA~~ or ~~relicensing~~.

Everything that runs on your machine stays open. Contributors sign off with a
[DCO](./CONTRIBUTING.md) and keep their copyright, so nobody can take the project closed. Not even
us.

## The repository

| Part | What it does |
| --- | --- |
| [`converter/`](./converter) | TypeScript: `stainless.yml` → annotated OpenAPI spec (overlays and `x-ferrule-*`) |
| [`python-plugin/`](./python-plugin) | TypeScript → WebAssembly: the Forge plugin that emits the Python SDK |
| [`runtime/`](./runtime) | The Python runtime copied into every generated SDK |
| [`migrate/`](./migrate) | The `ferrule` CLI: `generate`, `adopt`, `rescue`, `verify`, `merge` |
| [`action/`](./action) | The GitHub Action that keeps adopted SDKs regenerating |
| [`oracles/`](./oracles), [`corpus/`](./corpus) | Real SDKs and specs every change is tested against |
| [`site/`](./site) | The website and docs, deployed to GitHub Pages |

Decisions live in [`docs/adr/`](./docs/adr/) and the full plan in [`docs/plan.html`](./docs/plan.html).

## Contributing

```bash
mise install        # Node, pnpm, Python, uv, lefthook, gitleaks
mise run install    # JS and Python dependencies
mise run check      # everything CI runs
```

Bugs are fixed test-first, commits use Conventional Commits with a DCO sign-off (`git commit -s`),
and there is never a CLA. Start with [CONTRIBUTING.md](./CONTRIBUTING.md); see also
[GOVERNANCE.md](./GOVERNANCE.md) and [SECURITY.md](./SECURITY.md).

## Licence

Apache-2.0. See [LICENSE](./LICENSE) and [NOTICE](./NOTICE).

<sub>Ferrule is independent and not affiliated with Stainless or Cloudflare. A ferrule is the metal
collar that joins two parts so firmly you forget there's a joint. That's the job.</sub>
