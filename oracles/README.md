# Oracles (Tier 0)

Real Stainless-generated Python SDKs that Ferrule's output is checked against (plan §4, §12, §13).
Each one is pinned to the **last commit Stainless generated** in that repo.

| Name | Repo | Pinned SHA | Date | Commit |
|------|------|-----------|------|--------|
| onebusaway-config | OneBusAway/sdk-config | `27cdbb11faf296d3eed63aa19148dd006e05b934` | 2026-07-11 | input of python-sdk 1.30.0 |
| onebusaway-python | OneBusAway/python-sdk | `4c20fa834c2a1237ef046e4894af60cd02fb3bcf` | 2026-07-18 | release: 1.30.0 |
| groq-python | groq/groq-python | `ac18a8cdffdd4189b3f9a7d7c083e15e8e67ce58` | 2026-08-26 | release: 1.7.0 (#283) |
| lithic-python | lithic-com/lithic-python | `a1e8c1861ecb7053ef97aad80dcb48aa8fda8b3f` | 2026-09-18 | release: 0.132.0 |
| perplexity-py | perplexityai/perplexity-py | `f068fb8e270a56440b604703e39750ba219aa0b3` | 2026-07-17 | release: 0.42.0 (#67) |
| opencode-sdk-python | anomalyco/opencode-sdk-python | `817f1a08163452fd7fcfbb1268b262cc78a9d629` | 2025-08-27 | release: 0.1.0-alpha.36 |

`.stats.yml` values at each SHA are in [manifest.yaml](manifest.yaml).

## Usage

```bash
python3 oracles/fetch.py                  # all oracles
python3 oracles/fetch.py onebusaway       # a group (onebusaway-config + onebusaway-python)
python3 oracles/fetch.py groq-python      # one oracle
```

Checkouts go in `oracles/cache/<name>` (gitignored). For each oracle the harness:

1. runs `git clone --filter=blob:none --no-checkout` (or reuses the clone if it already exists),
2. checks out the pinned SHA as a detached HEAD and verifies that `HEAD` equals it,
3. checks `.stats.yml` against the manifest (`configured_endpoints`, `openapi_spec_url`,
   `openapi_spec_hash`, `config_hash`), plus any `verify_md5` file digests.

Running it again is a no-op when the checkout is already clean and at the pinned SHA.

### Oracles are data

The harness never runs code from a checkout. Every git call sets `core.hooksPath=/dev/null`,
turns off LFS smudge filters and submodule recursion, and forbids `file://` transports. Nothing
gets installed, imported or built. Later phases that run an oracle's tests have to do that in a
sandbox.

## Manifest format

`manifest.yaml` starts with `#` comment lines followed by a single JSON document. JSON is valid
YAML 1.2, so the file stays YAML as the plan says. `fetch.py` drops the comment lines and reads
the rest with the stdlib `json` module, so it needs no PyYAML and runs with only Python 3.12.
Don't add YAML-only syntax.

## How "last Stainless-generated commit" was chosen

Several of these repos have since moved to a different generator ("stlc" commits, `Build SDK`,
syncs by human accounts), so the newest commit is often not Stainless output. The rule:

1. Walk the default branch's history (`gh api repos/<repo>/commits?per_page=100&page=N`), newest
   first.
2. Pick the first commit whose GitHub **author** login is `stainless-app[bot]`. For a Stainless
   repo this is always a release commit (`release: X.Y.Z`), which sits on top of the codegen
   commits (`feat(api): api update`, `chore(internal): codegen related update`, ...). That is
   the last tree Stainless fully produced.
3. Read `.stats.yml` at that SHA (`gh api repos/<repo>/contents/.stats.yml?ref=<sha>`) and copy
   its four fields into the manifest.

`scripts/oracles/find_stainless_commit.py <owner/repo>` repeats steps 1–3 so pins can be
re-derived and checked.

### OneBusAway golden pair

`OneBusAway/sdk-config` is not generated, so it has no Stainless commit. The plan asks for the
config that **produced** the pinned python-sdk, so we pin `27cdbb11` rather than upstream HEAD,
which is the last sdk-config commit before python-sdk `release: 1.30.0`. Its contents confirm the
pairing: `md5(openapi.yml)` equals the python-sdk `openapi_spec_hash` (`8df6b187…`), and
`md5(stainless-config.yml)` equals its `config_hash` (`c28ddf5b…`). `fetch.py` checks both on
every run.

Upstream HEAD (`6894347048548b3cf1393a10527858ac8151f930`, 2026-09-01) dropped
`stainless-config.yml` and moved to an stlc workspace, so it cannot serve as the golden input.

### opencode

`sst/opencode-sdk-python` redirects to `anomalyco/opencode-sdk-python`, the canonical name the
GitHub API reports. Its HEAD is still the Stainless release commit.
