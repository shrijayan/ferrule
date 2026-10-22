# 0009. Which oracle configs Phase 2 converts

- Status: Accepted
- Date: 2026-10-23

## Context

Phase 2's gate is "all oracle configs convert". Of the Tier 0 oracles only
OneBusAway publishes its `stainless.yml` (OneBusAway/sdk-config). groq-python,
lithic-python, perplexity-py and opencode-sdk-python ship generated output and
`.stats.yml` (with a `config_hash`) but not the config.

Other projects do publish a Stainless config next to a public Python SDK
(found with GitHub code search for `stainless.yml`).

## Decision

- Phase 2 oracle configs are OneBusAway plus every public config whose
  matching Python SDK is public and Apache-2.0 or MIT. These are added to
  `oracles/manifest.yaml` as `config` entries pinned by commit, paired with
  their SDK at the commit whose `.stats.yml` `config_hash` equals the md5 of
  the config, when that can be shown. They are fetched into `oracles/cache/`
  and never committed; only converter snapshots of our output are committed.
- Configs for groq, lithic, perplexity and opencode are written by us in
  Phase 3–4 (from their `api.md`, generated code and public docs) under
  `oracles/configs/`, as an adopter would, and must reproduce the output.
  They are our own work, not recovered Stainless files.

## Consequences

The Phase 2 gate covers real-world config variety without guessing configs.
Configs we write can't be proven identical to the originals (`config_hash`
won't match); verification for those oracles is output parity, not config
equality.
