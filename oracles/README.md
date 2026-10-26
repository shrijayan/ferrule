# Oracles (Tier 0)

Real Stainless-generated Python SDKs that Ferrule's output is checked against (plan §4, §12, §13),
the public Stainless configs that produced them (ADR 0009), and the SDKs in other languages
generated from those same configs ([below](#sdk-oracles-in-other-languages)).
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
python3 oracles/fetch.py                        # every Python sdk oracle and every config
python3 oracles/fetch.py --kind config          # every config oracle (and its spec)
python3 oracles/fetch.py --kind sdk             # every Python sdk oracle
python3 oracles/fetch.py onebusaway             # a group (onebusaway-config + onebusaway-python)
python3 oracles/fetch.py groq-python            # one oracle
python3 oracles/fetch.py --language typescript  # every TypeScript sdk and the configs they pair with
python3 oracles/fetch.py --language go --group knock   # knock-config + knock-go
python3 oracles/fetch.py --language all onebusaway     # the config and all six onebusaway SDKs
python3 oracles/fetch.py onebusaway-ruby        # an oracle named exactly, whatever --language says
```

`--language` (one of `python`, `typescript`, `go`, `java`, `kotlin`, `ruby`, `php`, `csharp`, or
`all`; default `python`) narrows groups and "everything" to that language's SDKs plus the
configs they pair with, so the commands that predate it fetch exactly what they did before.
`--group` (repeatable) is a group given by flag; it must be a group, not an oracle name.

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
turns off LFS smudge filters and submodule recursion, and forbids `file://` transports. Clone
URLs must be `https://github.com/<repo>.git` and are checked again before cloning. Spec
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
| `language` | sdk | `python` (the default when absent), `typescript`, `go`, `java`, `kotlin`, `ruby`, `php` or `csharp` |
| `layout` | sdk | required for `typescript` (`node` or `typescript`) and `go` (`v1` or `v2`), absent otherwise |
| `config_path` | config | path of the Stainless config in the repo; its md5 must be in `verify_md5` |
| `spec` | config | `{"path": ...}` (also in `verify_md5`) or `{"url": ..., "md5": ...}` (https) |
| `paired` | config | `proven` or `nearest` (below), for the Python SDK it names |
| `paired` | non-Python sdk | `proven` or `nearest`, for this SDK against the config it names |
| `pairs_with` | config | required: the Python `sdk` entry this config produced |
| `pairs_with` | non-Python sdk | the `config` entry this SDK was generated from (same group) |

The Python entries predate `language` and keep their shape: no `language`, and the pairing lives
on the config. `fetch.py` refuses a manifest where a `proven` pair's md5s differ from the SDK's
`config_hash`/`openapi_spec_hash`, where a `nearest` pair actually matches both, or where a
config has two SDKs in one language.

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

## SDK oracles in other languages

Ferrule is extending past Python (ADR 0017). Each config oracle's `stainless.yml` also enables
other targets, and Stainless generated those SDKs from the same config and spec. Those repos are
the oracles for the other languages. Each entry is named `<group>-<language>` and points at the
group's config with `pairs_with`. All are Apache-2.0 or MIT.

| Name | Repo | Language (layout) | Pinned SHA | Date | Commit | Branch | Paired | Licence |
|------|------|-------------------|-----------|------|--------|--------|--------|---------|
| onebusaway-typescript | OneBusAway/js-sdk | typescript (`node`) | `f3ca3f47613e` | 2026-07-18 | release: 1.20.0 | main | proven | Apache-2.0 |
| onebusaway-go | OneBusAway/go-sdk | go (`v1`) | `d115209ec633` | 2026-07-18 | release: 1.15.0 | main | proven | Apache-2.0 |
| onebusaway-java | OneBusAway/java-sdk | java | `c29c48ad95f0` | 2026-08-04 | release: 0.1.0-alpha.60 | main | proven | Apache-2.0 |
| onebusaway-kotlin | OneBusAway/kotlin-sdk | kotlin | `6923446778f9` | 2026-07-18 | release: 0.1.0-alpha.81 | main | proven | Apache-2.0 |
| onebusaway-ruby | OneBusAway/ruby-sdk | ruby | `46a574ddc699` | 2026-07-18 | release: 1.12.0 | main | proven | Apache-2.0 |
| opencode-go | anomalyco/opencode-sdk-go | go (`v1`) | `6a22ce6df155` | 2025-08-29 | chore(internal): codegen related update | main | proven | MIT |
| isaacus-typescript | isaacus-dev/isaacus-typescript | typescript (`typescript`) | `c56af182d21e` | 2026-06-17 | release: 0.23.0 | main | proven | Apache-2.0 |
| lumaai-typescript | lumalabs/lumaai-node | typescript (`node`) | `4dbfae0fcaba` | 2026-04-07 | release: 1.21.0 | main | proven | Apache-2.0 |
| honcho-typescript | plastic-labs/honcho-node-core | typescript (`node`) | `91d258060f78` | 2025-10-08 | feat(api): api update | main | proven | Apache-2.0 |
| ogx-typescript | ogx-ai/ogx-client-typescript | typescript (`node`) | `029aa5f8d709` | 2026-06-11 | release: 1.1.0-alpha.6 (#59) | main | proven | MIT |
| ogx-go | ogx-ai/ogx-client-go | go (`v2`) | `c66deeffe544` | 2026-06-10 | codegen metadata | generated | proven | MIT |
| imagekit-typescript | imagekit-developer/imagekit-nodejs | typescript (`typescript`) | `9161f99f5612` | 2026-05-17 | build: change dotnet package name | main | proven | Apache-2.0 |
| imagekit-go | imagekit-developer/imagekit-go | go (`v2`) | `bc22fec79994` | 2026-05-17 | codegen metadata | main | proven | Apache-2.0 |
| imagekit-java | imagekit-developer/imagekit-java | java | `f96c35e8d1e5` | 2026-05-17 | codegen metadata | main | proven | Apache-2.0 |
| imagekit-ruby | imagekit-developer/imagekit-ruby | ruby | `65401086b818` | 2026-05-17 | codegen metadata | main | proven | Apache-2.0 |
| imagekit-php | imagekit-developer/imagekit-php | php | `9f9309d5b8a6` | 2026-05-17 | codegen metadata | next | proven | Apache-2.0 |
| knock-typescript | knocklabs/knock-node | typescript (`typescript`) | `e2c0f18ad312` | 2025-06-06 | release: 1.7.0 (#100) | main | proven | Apache-2.0 |
| knock-go | knocklabs/knock-go | go (`v1`) | `4b86a0d68f0a` | 2025-06-06 | release: 1.5.0 (#49) | main | proven | Apache-2.0 |
| knock-ruby | knocklabs/knock-ruby | ruby | `5bc0a747b8dc` | 2025-06-06 | release: 1.6.0 (#34) | main | proven | Apache-2.0 |
| knock-java | knocklabs/knock-java | java | `6e0df2347f66` | 2025-06-06 | feat(api): api update | generated | proven | Apache-2.0 |
| knock-mgmt-typescript | knocklabs/knock-mgmt-node | typescript (`typescript`) | `ca88fdba7037` | 2026-07-09 | release: 0.32.0 (#72) | main | nearest | Apache-2.0 |
| knock-mgmt-go | knocklabs/knock-mgmt-go | go (`v2`) | `85ae774cbff4` | 2026-07-09 | release: 0.2.0 (#3) | main | nearest | Apache-2.0 |
| papr-typescript | Papr-ai/papr-TypescriptSDK | typescript (`typescript`) | `41dbb3b3432a` | 2025-12-07 | release: 1.20.0 | main | nearest | Apache-2.0 |

Full SHAs, `.stats.yml` values and the pairing evidence for each entry are in
[manifest.yaml](manifest.yaml). Dates are committer dates in UTC. Release-please rebases make
several SHAs on one branch share a date.

### How the pairs were proven

The method is the same for every language. Each target's `.stats.yml` records `config_hash` and
`openapi_spec_hash`, just as the Python SDK's does.

1. Blobless-clone the SDK repo, then walk `main`, `next` and `generated` in topological order.
   For every commit, read the `.stats.yml` blob (the tree is local; only that blob is fetched).
2. **Proven**: `config_hash` equals md5 of the pinned config file and `openapi_spec_hash`
   equals md5 of the pinned spec. That spec is the in-repo file, or the downloaded `spec.url`
   for opencode, imagekit and knock. These are the `verify_md5` / `spec.md5` values of the
   config entry. `fetch.py` checks every pair again on each run.
3. **Pin**: the newest commit authored by `stainless-app[bot]` that carries those hashes, taken
   from the default branch. That is usually the `release:` commit on top. Human merges and
   `stlc` commits that keep the same `.stats.yml` are skipped. Only when the default branch has
   no such commit is the pin taken from `generated` (knock-java, ogx-go) or `next`
   (imagekit-php). The clone fetches every branch, so the SHA resolves.
4. **Nearest**: knock-mgmt and papr have no proven Python pair, so no config commit has their
   `config_hash`. Their TypeScript and Go SDKs carry exactly the same two hashes as the paired
   Python SDK (`= python`). They are therefore paired with the same config as `nearest`, and
   `fetch.py` refuses them as `nearest` if both hashes ever match. papr-typescript is pinned at
   release 1.20.0 (2025-12-07), the release closest to papr-config. That mirrors papr-python
   2.18.0, released 4 minutes later. Its later Stainless commits keep the hashes until
   2026-02-07.

Per-language notes:

- **TypeScript has two layouts**, recorded as `layout`. `node` is the legacy layout from the
  `node` config key: `src/core.ts`, `src/_shims/`, `src/index.ts` as the client. The OneBusAway,
  lumaai, honcho and ogx SDKs use it. `typescript` is the current layout from the `typescript`
  key: `src/client.ts`, `src/core/`, `src/internal/`. The isaacus, imagekit, knock, knock-mgmt
  and papr SDKs use it. Check `src/core.ts` vs `src/client.ts` at the pin.
- **Go** is `v1` (OneBusAway, knock, opencode) or `v2` (`enable_v2`: imagekit, ogx, knock-mgmt).
  `v2` adds `packages/param/` and `packages/respjson/`; imagekit's module path ends in `/v2`.
- **Java and Kotlin** are separate targets. Both are written in Kotlin, with near-identical
  runtimes, and each has its own oracle.
- **Same inputs, different counts**: imagekit's TypeScript, Java, Ruby and PHP SDKs report 47
  `configured_endpoints`; Python and Go report 48.
- The OneBusAway pins come from the same codegen run as onebusaway-python 1.30.0 (2026-07-18).
  The exception is java: alpha.60 (2026-08-04) regenerated from the same inputs and is pinned
  instead of alpha.59. Every OneBusAway repo moved to stlc from 2026-08-11. `next` and
  `generated` hold later `stainless-app[bot]` commits with the same hashes that were never
  released on main; they are not used.

### Skipped

| Repo(s) | Licence | Why |
|---------|---------|-----|
| browserbase/stagehand-{go,java,kotlin,ruby,php,net} | MIT | Proven only on `next`. Their last generation was never merged to the default branch, which is deprecated, and java, kotlin, php and net are archived. No release to pin. They can be added from the `next` SHAs if stagehand becomes a target. Stagehand's TypeScript target has `production_repo: null`. |
| anomalyco/opencode-sdk-js | MIT | Not proven. Six commits on main carry the pinned `config_hash`, with five different specs, but none has the pinned `openapi_spec_hash` 4ff9376c…. |
| imagekit-developer/imagekit-dotnet | Apache-2.0 | Unpaired. Its `.stats.yml` hashes come from July 2026 configs that were never published. |
| imagekit cli, terraform, kotlin targets | — | `production_repo: null` in the config, so no repo exists. |
| groq, lithic, perplexity | — | No public config, so their other-language SDKs have nothing to pair with. |

No candidate was dropped for its licence: every repo that pairs is Apache-2.0 or MIT
(`gh api repos/O/R --jq .license.spdx_id`). The SDK repos are fetched into `oracles/cache/` as
data and are not copied into Ferrule (see NOTICE).
