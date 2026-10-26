---
title: GitHub Action
description: Regenerate, verify, label the semver bump and open a PR on every spec change.
---

The Ferrule action keeps an adopted SDK up to date. Each time your spec or config changes, it:

1. **Generates** the SDK with `ferrule generate` in adopt mode. The runtime comes from your repository, and hand-written files (`lib/`, `keep_files`) are never deleted.
2. **Verifies** the new public API with `ferrule verify --json` against the previous release (the latest tag, or `HEAD`).
3. **Labels** the bump: any breaking change is `major`, any additive change is `minor`, anything else is `patch`.
4. **Opens or updates a PR** from the `ferrule/regenerate` branch with that label, when `open-pr: true`.
5. **Fails** on breaking changes when `verify: true`, unless the PR that triggered the run carries `ferrule:allow-breaking`. That PR must be from the same repository. Each force-push of new content removes the label, so every new breaking diff needs a fresh approval.

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

## Bring your own GitHub App token

Ferrule runs no bot of its own. The action runs in your repository, and `GITHUB_TOKEN` is enough
to push the branch and open the PR. Two limits of `GITHUB_TOKEN` make a GitHub App token worth it:

- **CI does not run on the regenerate PR.** GitHub does not start other workflows for pushes and
  PRs made with `GITHUB_TOKEN`, so the PR arrives with no test results.
- **It cannot reach another repository.** When the spec lives in your API repo and the SDK in its
  own repo, `GITHUB_TOKEN` in one cannot write to the other.

Create a GitHub App in your own organization (Settings, Developer settings, GitHub Apps). It
needs no webhook. Give it the repository permissions **Contents: read and write** and **Pull
requests: read and write**, then install it on the SDK repo (and the API repo, for the cross-repo
setup below). Store its client ID as the variable `SDK_BOT_CLIENT_ID` and its private key as
the secret `SDK_BOT_PRIVATE_KEY`. Then mint a token in the job and pass it as `github-token`:

```yaml
    steps:
      - uses: actions/create-github-app-token@64ab729a6294000f5aa4a4bc0b73d1f32bc45510 # v3.3.0
        id: app-token
        with:
          client-id: ${{ vars.SDK_BOT_CLIENT_ID }}
          private-key: ${{ secrets.SDK_BOT_PRIVATE_KEY }}
          permission-contents: write
          permission-pull-requests: write
      # checkout as in Usage above, then:
      - uses: shrijayan/ferrule/action@<sha>
        with:
          spec: openapi.json
          config: stainless.yml
          open-pr: true
          github-token: ${{ steps.app-token.outputs.token }}
```

The PR is now opened by `<your-app>[bot]`, and your CI runs on it. The token expires when the job
ends. Commits are still authored as `github-actions[bot]` and signed off.

### Spec in one repo, SDK in another

The API repo tells the SDK repo that the spec changed, with a `repository_dispatch` event. The SDK
repo checks out the spec at that commit and runs the action. In the API repo:

```yaml
# acme-api: .github/workflows/sdk-dispatch.yml
name: sdk-dispatch
on:
  push:
    branches: [main]
    paths: [openapi.json, stainless.yml]

permissions: {}

jobs:
  dispatch:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/create-github-app-token@64ab729a6294000f5aa4a4bc0b73d1f32bc45510 # v3.3.0
        id: app-token
        with:
          client-id: ${{ vars.SDK_BOT_CLIENT_ID }}
          private-key: ${{ secrets.SDK_BOT_PRIVATE_KEY }}
          repositories: acme-python
          permission-contents: write   # repository_dispatch needs contents: write
      - env:
          GH_TOKEN: ${{ steps.app-token.outputs.token }}
          SHA: ${{ github.sha }}
        run: |
          gh api repos/acme/acme-python/dispatches \
            -f event_type=spec-updated -f "client_payload[sha]=$SHA"
```

In the SDK repo:

```yaml
# acme-python: .github/workflows/sdk.yml
name: sdk
on:
  repository_dispatch:
    types: [spec-updated]
  pull_request:            # re-runs verify when a maintainer adds ferrule:allow-breaking
    types: [opened, synchronize, labeled]
    branches: [main]
  workflow_dispatch:

permissions: {}

jobs:
  regenerate:
    if: github.event_name != 'pull_request' || github.head_ref == 'ferrule/regenerate'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/create-github-app-token@64ab729a6294000f5aa4a4bc0b73d1f32bc45510 # v3.3.0
        id: spec-token
        with:
          client-id: ${{ vars.SDK_BOT_CLIENT_ID }}
          private-key: ${{ secrets.SDK_BOT_PRIVATE_KEY }}
          repositories: acme-api
          permission-contents: read
      - uses: actions/create-github-app-token@64ab729a6294000f5aa4a4bc0b73d1f32bc45510 # v3.3.0
        id: sdk-token
        with:
          client-id: ${{ vars.SDK_BOT_CLIENT_ID }}
          private-key: ${{ secrets.SDK_BOT_PRIVATE_KEY }}
          permission-contents: write
          permission-pull-requests: write
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          fetch-depth: 0
          persist-credentials: false
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          repository: acme/acme-api
          ref: ${{ github.event.client_payload.sha || 'main' }}
          path: .api
          token: ${{ steps.spec-token.outputs.token }}
          persist-credentials: false
      # Keep the spec checkout out of the commit: the action refuses files it did not generate.
      - run: echo ".api/" >> .git/info/exclude
      - uses: shrijayan/ferrule/action@<sha>
        with:
          spec: .api/openapi.json
          config: .api/stainless.yml
          verify: true
          open-pr: true
          github-token: ${{ steps.sdk-token.outputs.token }}
```

One token per repository keeps each to least privilege: read on the API repo, write on the SDK
repo. A fine-grained personal access token also works in place of the App, but it acts as a
person and outlives the job, so prefer the App.

## Try it locally

`action/scripts/dry_run.sh [report.json] [labels]` runs the label and PR logic against a fixture verify report and prints the PR plan, with no network access. The full reference is in [`action/README.md`](https://github.com/shrijayan/ferrule/blob/main/action/README.md).
