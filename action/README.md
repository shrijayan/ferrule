# Ferrule GitHub Action

A composite action. It regenerates a Python SDK adopted with `ferrule adopt` and merges the new generation into a `ferrule/regenerate` branch with `ferrule regenerate`, so custom code (in its own files or in generated ones) and `keep_files` survive. It checks the public API with `ferrule verify` against the previous release, labels the semver bump and, if you ask it to, opens or updates a PR.

Reference it as `shrijayan/ferrule/action@<sha>`, pinned to a full commit SHA.

## Usage

```yaml
# .github/workflows/sdk.yml
name: sdk
on:
  push:
    paths: [openapi.json, stainless.yml]
  workflow_dispatch:

permissions: {}

jobs:
  regenerate:
    runs-on: ubuntu-latest
    permissions:
      contents: write        # push the ferrule/regenerate branch
      pull-requests: write   # create/update the PR and its label
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          fetch-depth: 0           # verify needs the previous tag
          persist-credentials: false
      - uses: shrijayan/ferrule/action@<sha>
        with:
          spec: openapi.json
          config: stainless.yml
          verify: true    # fails on unlabeled breaking change
          open-pr: true   # labels major / minor / patch
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

## Bring your own App token

`secrets.GITHUB_TOKEN` is enough when the spec lives in the SDK repository and a person merges
the PR. It falls short in two cases:

- **CI on the PR.** Events caused by `GITHUB_TOKEN` start no workflow runs, so the PR the action
  opens gets no checks.
- **Another repository.** `GITHUB_TOKEN` is scoped to the repository its workflow runs in, so an
  API repository can't use it to tell the SDK repository its spec changed.

Use a token from a GitHub App you own instead. Create the App with **Contents: read & write** and
**Pull requests: read & write** on the SDK repository (add **Contents** on the API repository for
the cross-repository flow below), install it on both, and store its ID and private key as the
`SDK_APP_ID` variable and `SDK_APP_PRIVATE_KEY` secret. Then mint a short-lived token in the job:

```yaml
    steps:
      - uses: actions/create-github-app-token@64ab729a6294000f5aa4a4bc0b73d1f32bc45510 # v3.3.0
        id: app-token
        with:
          app-id: ${{ vars.SDK_APP_ID }}
          private-key: ${{ secrets.SDK_APP_PRIVATE_KEY }}
      # ... checkout as above ...
      - uses: shrijayan/ferrule/action@<sha>
        with:
          spec: openapi.json
          config: stainless.yml
          open-pr: true
          github-token: ${{ steps.app-token.outputs.token }}
```

### Regenerate when another repository changes its spec

When the spec is published by a separate API repository, that repository sends a
`repository_dispatch` event to the SDK repository; the SDK repository copies the spec in and
commits it, and that push runs the regeneration workflow above (its `on: push: paths:` matches
the spec). Each side checks what it receives and moves no secrets between repositories.

In the API repository:

```yaml
# .github/workflows/notify-sdk.yml
name: notify-sdk
on:
  push:
    branches: [main]
    paths: [openapi.json]

permissions: {}

jobs:
  dispatch:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/create-github-app-token@64ab729a6294000f5aa4a4bc0b73d1f32bc45510 # v3.3.0
        id: app-token
        with:
          app-id: ${{ vars.SDK_APP_ID }}
          private-key: ${{ secrets.SDK_APP_PRIVATE_KEY }}
          owner: acme
          repositories: acme-python
      - name: Tell the SDK repository the spec changed
        env:
          GH_TOKEN: ${{ steps.app-token.outputs.token }}
        run: >-
          gh api repos/acme/acme-python/dispatches
          -f event_type=spec-updated
          -f "client_payload[sha]=$GITHUB_SHA"
```

In the SDK repository:

```yaml
# .github/workflows/sync-spec.yml
name: sync-spec
on:
  repository_dispatch:
    types: [spec-updated]
  workflow_dispatch:

permissions: {}

jobs:
  sync:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/create-github-app-token@64ab729a6294000f5aa4a4bc0b73d1f32bc45510 # v3.3.0
        id: app-token
        with:
          app-id: ${{ vars.SDK_APP_ID }}
          private-key: ${{ secrets.SDK_APP_PRIVATE_KEY }}
          owner: acme
          repositories: acme-api,acme-python
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          token: ${{ steps.app-token.outputs.token }} # its push starts sdk.yml; GITHUB_TOKEN's would not
      - name: Copy the spec from the API repository
        env:
          GH_TOKEN: ${{ steps.app-token.outputs.token }}
          SHA: ${{ github.event.client_payload.sha || 'main' }}
        run: |
          # The payload is input: accept a full commit SHA (or main for a manual run), nothing else.
          [[ "$SHA" =~ ^([0-9a-f]{40}|main)$ ]] || { echo "::error::bad sha $SHA"; exit 1; }
          gh api "repos/acme/acme-api/contents/openapi.json?ref=$SHA" \
            -H "Accept: application/vnd.github.raw" > openapi.json
          git diff --quiet -- openapi.json && { echo "spec unchanged"; exit 0; }
          git -c user.name="acme-sdk[bot]" -c user.email="sdk-bot@acme.invalid" \
            commit -s -m "chore: sync openapi.json from acme-api@${SHA:0:12}" -- openapi.json
          git push origin HEAD:main
```

The push to `main` needs the App to be allowed past branch protection; to review the spec first,
push to a branch and open a PR instead, and regeneration runs when it is merged. The `sdk.yml`
job then opens or updates the `ferrule/regenerate` PR with the verify result, and because the
App's token opened it, the SDK's own CI runs on it.

## Inputs

| Input | Default | Meaning |
|---|---|---|
| `spec` | (required) | OpenAPI spec, a path relative to the repo root |
| `config` | (required) | `stainless.yml`, a path relative to the repo root |
| `verify` | `true` | Fail on breaking API changes unless the PR has the `ferrule:allow-breaking` label |
| `open-pr` | `false` | Commit to `ferrule/regenerate` and create or update a PR with the semver label |
| `against` | latest tag, else `HEAD` | Git ref to verify against |
| `python-version` | `3.12` | Python version for Ferrule |
| `ferrule-version` | `0.1.0` | Exact Ferrule version from PyPI, or `source` to install from the action's own checkout (for dogfooding in this repo) |
| `github-token` | empty | Token for `gh`. Required when `open-pr: true` |

The SDK is generated by the `ferrule-gen` binary. Until it is released as a download, the action
builds it with `cargo build --release -p ferrule-gen` from its own checkout. That needs the Rust
toolchain GitHub-hosted runners already have, and takes a few minutes on a cold run. To skip the
build, set `FERRULE_GEN` in the job environment to a `ferrule-gen` binary; it is passed through
to `ferrule generate`.

## Outputs

| Output | Meaning |
|---|---|
| `label` | `major` if anything is breaking, `minor` if anything is additive, otherwise `patch` |
| `breaking` | Number of breaking public API changes |
| `additive` | Number of additive public API changes |

## How custom code survives

`ferrule adopt` keeps the generation it committed on a `generated` branch and names it in its
commit's `Ferrule-Generated-From` trailer. Each run commits the new generation on `generated`
and merges it three-way into `ferrule/regenerate`, from the last generation that branch took (the
newest trailer). Push `generated` along with the adopt branch, keep `fetch-depth: 0` on checkout
so the action sees it, and keep the trailer when you merge the PR (a squash merge keeps it in
the message body). A change that conflicts with custom code opens the PR with the conflict
markers, and the job fails until someone resolves them.

## Breaking changes

When `verify: true` finds a breaking change and no PR label overrides it, the PR is still opened (if `open-pr` is on), and then the job fails. The override is read only from the PR that triggered the run, and only when that PR's head is in the same repository. To accept a change, run the workflow on `pull_request` (types `opened`, `synchronize`, `labeled`) for the `ferrule/regenerate` PR, and have a maintainer add `ferrule:allow-breaking` to it. Each force-push of new content removes that label, so every new breaking diff must be approved again.

- Only same-repository PRs are updated. A fork PR whose head branch is also named `ferrule/regenerate` is ignored.
- Only the merge `ferrule regenerate` committed is pushed, with the `generated` branch it merged from.

## Security

- Least privilege: `contents: write` and `pull-requests: write` are needed only with `open-pr: true`. Otherwise `contents: read` is enough.
- No input is ever interpolated into a `run:` script. Inputs reach the scripts through `env:`, and `scripts/validate.sh` checks them first: paths must be relative and contain no `..`, booleans must be literal, versions must be exact.
- Use `persist-credentials: false` on checkout. The push authenticates with `gh auth setup-git` using `github-token`.
- Every action it uses is pinned by SHA. The action passes `zizmor` and `actionlint`.

## Local dry run

```bash
action/scripts/dry_run.sh                                        # additive -> minor, PR plan
action/scripts/dry_run.sh action/scripts/tests/fixtures/breaking.json                         # fails
action/scripts/dry_run.sh action/scripts/tests/fixtures/breaking.json ferrule:allow-breaking  # passes
```

The dry run validates the inputs, runs `label.py` against the fixture report and prints the git and `gh` commands that `open_pr.sh` would run (it assumes a new generation). It uses no network. It does not need [`act`](https://github.com/nektos/act), which this repo does not install.
