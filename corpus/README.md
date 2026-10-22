# Spec corpus (Tiers 1–2)

Real-world OpenAPI specs that the converter and generator must survive (plan §12, §13). Every
entry is pinned: GitHub-hosted files use a raw URL at a fixed commit SHA, and all Tier 1 files
also record a sha256. Tier 0 (the oracles) is in [../oracles](../oracles).

| Tier | Contents | Runs |
|------|----------|------|
| 1 | Stripe, GitHub, OpenAI, Kubernetes, Twilio (API v2010), Cloudflare, Azure (Storage RM 2026-09-01), Slack, Plaid, Square, DigitalOcean, Discord, Docker (moby), Jira | Nightly |
| 2 | APIs.guru `openapi-directory` at a pinned commit | Weekly |

## Usage

```bash
python3 corpus/fetch.py                 # Tier 1, all specs (about 70 MB)
python3 corpus/fetch.py stripe jira     # selected specs
python3 corpus/fetch.py --tier 2        # APIs.guru: blobless, no-checkout clone (~20 MB)
```

Files go to `corpus/cache/tier1/<name>.<json|yaml>` and `corpus/cache/tier2/<name>/`, both
gitignored. A cached file that still matches its sha256 is not downloaded again. A mismatch
fails the run and leaves the cached copy alone. Specs are data: the harness hashes them and
never parses or executes them.

## Manifest format

`manifest.yaml` is `#` comment lines followed by one JSON document. That is valid YAML 1.2, and
fetch.py reads it with the stdlib `json` module, so it has no dependencies. See
[../oracles/README.md](../oracles/README.md).

## Re-pinning

* GitHub-hosted: choose a new commit, update `source.sha` and `url`, then download the file and
  record its `sha256` and `size`.
* Jira is served by Atlassian (`developer.atlassian.com`) and not by a git host, so its sha256
  is the only pin. Atlassian replaces the file in place, so a hash mismatch for Jira usually means
  upstream moved and the entry needs re-pinning, not that the download was corrupted.
* DigitalOcean's root document `$ref`s sibling files. Only the root file is pinned here; later
  phases that resolve refs should fetch from `source.repo` at `source.sha`.

## Layout

* `fixtures/`: small minimised specs reduced from corpus failures (committed).
* `baselines/`: expected results per corpus spec (committed).
