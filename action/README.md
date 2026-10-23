# Ferrule GitHub Action

A composite action. It regenerates a Python SDK with Ferrule in adopt mode (the runtime comes from your own repo), checks the public API with `ferrule verify` against the previous release, labels the semver bump and, if you ask it to, opens or updates a PR.

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

`FERRULE_FORGE` and `FERRULE_PLUGIN` in the job environment are passed through to `ferrule generate`.

## Outputs

| Output | Meaning |
|---|---|
| `label` | `major` if anything is breaking, `minor` if anything is additive, otherwise `patch` |
| `breaking` | Number of breaking public API changes |
| `additive` | Number of additive public API changes |

## Breaking changes

When `verify: true` finds a breaking change and no PR label overrides it, the PR is still opened (if `open-pr` is on), and then the job fails. The override is read only from the PR that triggered the run, and only when that PR's head is in the same repository. To accept a change, run the workflow on `pull_request` (types `opened`, `synchronize`, `labeled`) for the `ferrule/regenerate` PR, and have a maintainer add `ferrule:allow-breaking` to it. Each force-push of new content removes that label, so every new breaking diff must be approved again.

- Only same-repository PRs are updated. A fork PR whose head branch is also named `ferrule/regenerate` is ignored.
- Only the paths the generator wrote are committed. If anything else in the working tree has changed, the action refuses to commit or push.

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

The dry run validates the inputs, runs `label.py` against the fixture report and prints the git and `gh` commands that `open_pr.sh` would run. It uses no network. It does not need [`act`](https://github.com/nektos/act), which this repo does not install.
