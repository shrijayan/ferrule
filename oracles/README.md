# Oracles (Tier 0)

Real Stainless-generated Python SDKs that Ferrule's output is checked against (plan §4, §12, §13),
and the public Stainless configs that produced them (ADR 0009).
Each SDK is pinned to the **last commit Stainless generated** in that repo, or, when it is paired
with a public config, to the newest Stainless commit generated **from that config**.

| Name | Repo | Pinned SHA | Date | Commit |
|------|------|-----------|------|--------|
| onebusaway-config | OneBusAway/sdk-config | `27cdbb11faf296d3eed63aa19148dd006e05b934` | 2026-07-11 | input of python-sdk 1.30.0 |
| onebusaway-python | OneBusAway/python-sdk | `4c20fa834c2a1237ef046e4894af60cd02fb3bcf` | 2026-07-18 | release: 1.30.0 |
| groq-python | groq/groq-python | `ac18a8cdffdd4189b3f9a7d7c083e15e8e67ce58` | 2026-08-26 | release: 1.7.0 (#283) |
| lithic-python | lithic-com/lithic-python | `a1e8c1861ecb7053ef97aad80dcb48aa8fda8b3f` | 2026-09-18 | release: 0.132.0 |
| perplexity-py | perplexityai/perplexity-py | `f068fb8e270a56440b604703e39750ba219aa0b3` | 2026-07-17 | release: 0.42.0 (#67) |
| opencode-sdk-python | anomalyco/opencode-sdk-python | `817f1a08163452fd7fcfbb1268b262cc78a9d629` | 2025-08-27 | release: 0.1.0-alpha.36 |
| isaacus-python | isaacus-dev/isaacus-python | `c3a0e7dfc65dcecea7a6bc0452dc4a221f390b91` | 2026-06-01 | release: 0.22.0 |
| lumaai-python | lumalabs/lumaai-python | `513697a05d0e3187cceef996443b792f46d7033f` | 2026-04-01 | release: 1.21.0 |
| honcho-python-core | plastic-labs/honcho-python-core | `9be4f056dd866fde6102220904de433b65211370` | 2025-10-08 | feat(api): api update |
| stagehand-python | browserbase/stagehand-python | `9fe6865a103f382bd1d31a2dbd44438d3c9d20bb` | 2026-07-22 | release: 3.22.0 (#344) |
| ogx-client-python | ogx-ai/ogx-client-python | `e932a74d284f38116cfa15a9004d17a0afe1f2b2` | 2026-06-11 | release: 1.1.0-alpha.3 (#345) |
| imagekit-python | imagekit-developer/imagekit-python | `e75f6267fe641d84d0eb8b53aaf2ba108f0c07d4` | 2026-05-17 | codegen metadata |
| knock-python | knocklabs/knock-python | `248fa874915e543427ed44b899bf9151de726400` | 2025-06-06 | release: 1.6.0 (#48) |
| knock-mgmt-python | knocklabs/knock-mgmt-python | `50e5904329fb4db01f3a6e635e4a045de4e901c3` | 2026-07-09 | release: 0.1.0-alpha.2 (#4) |
| papr-python | Papr-ai/papr-pythonSDK | `94759183da64fbcf59545ebc46d4f07b61bfe6be` | 2025-12-07 | release: 2.18.0 |

The config oracles are listed in [Config oracles](#config-oracles-adr-0009). `.stats.yml` values
at each SHA, and the notes on why each pin was chosen, are in [manifest.yaml](manifest.yaml).

## Usage

```bash
python3 oracles/fetch.py                  # all oracles
python3 oracles/fetch.py --kind config    # every config oracle (and its spec)
python3 oracles/fetch.py --kind sdk       # every sdk oracle
python3 oracles/fetch.py onebusaway       # a group (onebusaway-config + onebusaway-python)
python3 oracles/fetch.py groq-python      # one oracle
```

Checkouts go in `oracles/cache/<name>` (gitignored). For each oracle the harness:

1. runs `git clone --filter=blob:none --no-checkout` (or reuses the clone if it already exists),
2. checks out the pinned SHA as a detached HEAD and verifies that `HEAD` equals it,
3. checks `.stats.yml` against the manifest (`configured_endpoints`, `openapi_spec_url`,
   `openapi_spec_hash`, `config_hash`), plus any `verify_md5` file digests,
4. for a `config` oracle, makes sure its spec is on disk with the pinned md5: either a file in
   the checkout, or a download from `spec.url` saved as `oracles/cache/specs/<name>.<ext>`.

Running it again is a no-op when the checkout is already clean and at the pinned SHA, and a
downloaded spec is reused while its md5 still matches.

### Oracles are data

The harness never runs code from a checkout. Every git call sets `core.hooksPath=/dev/null`,
turns off LFS smudge filters and submodule recursion, and forbids `file://` transports. Spec
downloads are https only, capped at 64 MiB, and written only after their md5 matches. Nothing
gets installed, imported or built. Later phases that run an oracle's tests have to do that in a
sandbox.

## Manifest format

`manifest.yaml` starts with `#` comment lines followed by a single JSON document. JSON is valid
YAML 1.2, so the file stays YAML as the plan says. `fetch.py` drops the comment lines and reads
the rest with the stdlib `json` module, so it needs no PyYAML and runs with only Python 3.12.
Don't add YAML-only syntax.

Every entry has `name`, `group`, `kind` (`sdk` or `config`), `repo`, `url`, `sha`,
`commit_date`, `pairs_with`, `stats` and `verify_md5`. On top of that:

| Field | Kind | Meaning |
|-------|------|---------|
| `license` | sdk | SPDX id, `Apache-2.0` or `MIT`; required when a config pairs with this SDK |
| `config_path` | config | path of the Stainless config in the repo; its md5 must be in `verify_md5` |
| `spec` | config | `{"path": ...}` (also in `verify_md5`) or `{"url": ..., "md5": ...}` (https) |
| `paired` | config | `proven` or `nearest` (below) |
| `pairs_with` | config | required: the `sdk` entry this config produced |

`fetch.py` refuses a manifest where a `proven` config's md5s differ from its SDK's
`config_hash`/`openapi_spec_hash`, or where a `nearest` config actually matches both.

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

## Config oracles (ADR 0009)

Phase 2 converts every public Stainless config whose Python SDK is public and Apache-2.0 or MIT.
Configs are fetched into `oracles/cache/` and never committed or copied into this repo (some
config repos have no licence or a copyleft one).

| Config | Repo | Config path | Pinned SHA | Date | SDK oracle (SHA, date) | Paired | SDK licence |
|--------|------|-------------|-----------|------|------------------------|--------|-------------|
| onebusaway-config | OneBusAway/sdk-config | `stainless-config.yml` | `27cdbb11faf2` | 2026-07-11 | onebusaway-python `4c20fa834c2a` 2026-07-18 | proven | Apache-2.0 |
| opencode-config | anomalyco/opencode | `packages/sdk/stainless/stainless.yml` | `33cef075d228` | 2025-07-31 | opencode-sdk-python `817f1a081634` 2025-08-27 | proven (spec from URL) | MIT |
| isaacus-config | isaacus-dev/openapi | `config/stainless.yaml` | `ee9809820d1c` | 2026-08-23 | isaacus-python `c3a0e7dfc65d` 2026-06-01 | proven | Apache-2.0 |
| lumaai-config | lumalabs/lumaai-api | `openapi.stainless.yml` | `5834633ba93c` | 2026-01-07 | lumaai-python `513697a05d0e` 2026-04-01 | proven | Apache-2.0 |
| honcho-config | plastic-labs/honcho-stainless-config | `openapi.stainless.yml` | `d0fe5dada0ac` | 2025-10-08 | honcho-python-core `9be4f056dd86` 2025-10-08 | proven | Apache-2.0 |
| stagehand-config | browserbase/stagehand | `stainless.yml` | `e0e5c3bc3c8f` | 2026-07-21 | stagehand-python `9fe6865a103f` 2026-07-22 | proven | MIT |
| ogx-config | ogx-ai/ogx (ex llama-stack) | `client-sdks/stainless/config.yml` | `b7fc2751f009` | 2026-06-10 | ogx-client-python `e932a74d284f` 2026-06-11 | proven | MIT |
| imagekit-config | imagekit-developer/sdk-generation | `stainless-config/main.yaml` | `28534c19366c` | 2026-05-17 | imagekit-python `e75f6267fe64` 2026-05-17 | proven (spec from URL) | Apache-2.0 |
| knock-config | knocklabs/docs | `data/specs/api/stainless.yml` | `f6eb88e11425` | 2025-06-04 | knock-python `248fa874915e` 2025-06-06 | proven (spec from URL) | Apache-2.0 |
| knock-mgmt-config | knocklabs/docs | `data/specs/mapi/stainless.yml` | `2a646bdfd8ec` | 2026-07-08 | knock-mgmt-python `50e5904329fb` 2026-07-09 | nearest | Apache-2.0 |
| papr-config | Papr-ai/memory-opensource | `openapi.stainless.yml` | `7871626834fa` | 2025-12-08 | papr-python `94759183da64` 2025-12-07 | nearest | Apache-2.0 |

Full SHAs, spec paths/URLs and per-pair notes are in [manifest.yaml](manifest.yaml).

### Method

1. **Find candidates.** `gh search code --filename stainless.yml` (and `stainless.yaml`,
   `stainless-config.yml`, `openapi.stainless.yml`, plus `production_repo --extension yml`) lists
   repos with a config. Keep those whose config has a `targets.python` with a `production_repo`
   that is a public repo licensed Apache-2.0 or MIT
   (`gh api repos/O/R --jq .license.spdx_id`). Skip forks and copies of other projects' configs,
   Stainless-owned demos and test fixtures, and SDKs that Stainless never generated (no
   `stainless-app[bot]` commit, e.g. kernel/hypeman-python).
2. **Pair by md5.** Stainless records `config_hash` (md5 of the config file) and
   `openapi_spec_hash` (md5 of the spec) in each SDK's `.stats.yml`. For every
   `stainless-app[bot]` SDK commit, newest first, look for a config-repo commit where
   md5(config) equals `config_hash` and, if the spec is in the repo, md5(spec) equals
   `openapi_spec_hash`. `python3 -m scripts.oracles.pair_config CONFIG_REPO CONFIG_PATH SDK_REPO
   --spec SPEC_PATH` does this on bare clones in `oracles/cache/pairing/`.
3. **Proven.** Both hashes match. The SDK is pinned at the newest Stainless commit with that
   `.stats.yml` (usually the release on top), the config at a commit with those exact bytes.
   When the spec is not in the config repo (opencode, imagekit) or the repo's copy never
   matches (knock), the spec is the SDK's `openapi_spec_url`, accepted only because its md5 is
   `openapi_spec_hash`. Where that URL has since changed (stagehand, ogx) the in-repo spec is
   the proof.
4. **Nearest.** No config-repo commit has the SDK's `config_hash`. The pair is the config commit
   and the Stainless SDK commit closest in time (dates in the table); conversion still runs, but
   differences from the SDK are not evidence of a converter bug.

The SDK pinned for a pair can be older than the SDK's last Stainless commit when later
generations used a config that was never published (honcho, imagekit, knock).

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
GitHub API reports. Its HEAD is still the Stainless release commit. Its config lives in the
opencode monorepo (`sst/opencode`, now `anomalyco/opencode`) and was removed from HEAD later.
