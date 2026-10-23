---
title: GitHub Action
description: Regenerate, verify, label the semver bump and open a PR on every spec change.
---

The Ferrule action keeps an adopted SDK up to date. Each time your spec or config changes, it:

1. **Generates** the SDK with `ferrule generate` in adopt mode. The runtime comes from your repository, and hand-written files (`lib/`, `keep_files`) are never deleted.
2. **Verifies** the new public API with `ferrule verify --json` against the previous release (the latest tag, or `HEAD`).
3. **Labels** the bump: any breaking change is `major`, any additive change is `minor`, anything else is `patch`.
4. **Opens or updates a PR** from the `ferrule/regenerate` branch with that label, when `open-pr: true`.
5. **Fails** on breaking changes when `verify: true`, unless the PR carries `ferrule:allow-breaking`.

## Usage

Pin the action to a full commit SHA: `shrijayan/ferrule/action@<sha>`.

```yaml
jobs:
  regenerate:
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          fetch-depth: 0
          persist-credentials: false
      - uses: shrijayan/ferrule/action@<sha>
        with:
          spec: openapi.json
          config: stainless.yml
          verify: true    # fails on unlabeled breaking change
          open-pr: true   # labels major / minor / patch
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

## Inputs and outputs

Inputs: `spec`, `config`, `verify` (default `true`), `open-pr` (default `false`), `against`, `python-version` (default `3.12`), `ferrule-version` (an exact release, or `source` for dogfooding from this repository), and `github-token`.

Outputs: `label` (`major`, `minor` or `patch`), `breaking` and `additive` (change counts).

## Permissions

The job needs `contents: write` and `pull-requests: write` only to push the branch and manage the PR. Without `open-pr`, `contents: read` is enough. Inputs are validated and passed to scripts through environment variables, never interpolated into shell.

## Try it locally

`action/scripts/dry_run.sh [report.json] [labels]` runs the label and PR logic against a fixture verify report and prints the PR plan, with no network access. The full reference is in [`action/README.md`](https://github.com/shrijayan/ferrule/blob/main/action/README.md).
