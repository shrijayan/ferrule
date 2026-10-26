---
title: Commands
description: Every ferrule command, its flags and its exit codes.
---

`ferrule` is a Python CLI. Until the 0.1 release, run it from a Ferrule checkout with
`uv run ferrule …` (see the [quickstart](/docs/quickstart/)).

`adopt` and `generate` run the `ferrule-gen` binary, which holds the converter, Forge and the
Ferrule plugin. Build it with `cargo build --release -p ferrule-gen`. Pass it as `--gen PATH`,
or set `FERRULE_GEN`; the default is `target/release/ferrule-gen` in the Ferrule checkout.

## adopt

Switch an existing Stainless-generated SDK repository to Ferrule.

```bash
ferrule adopt [REPO] [--config STAINLESS_YML] [--spec SPEC] [--dry-run] [--gen PATH]
```

| Flag | Default | Meaning |
| --- | --- | --- |
| `REPO` | `.` | The SDK checkout, with full git history. |
| `--config` | `REPO/stainless.yml` | Your Stainless config. |
| `--spec` | rescued | Use this OpenAPI spec instead of rescuing the one `.stats.yml` names. |
| `--dry-run` | off | Rescue, generate and find hand edits in a temporary worktree, then stop. |
| `--gen` | `$FERRULE_GEN` | The `ferrule-gen` binary. |

Rescues the spec, regenerates in a temporary worktree, records hand edits since the last
Stainless commit as `.ferrule/patches/`, writes `ferrule.lock` and `.github/workflows/sdk.yml`,
and commits everything on the local branch `ferrule/adopt`. Nothing is pushed. Then runs
`verify` against the last Stainless commit.

## rescue

Save the OpenAPI spec that `.stats.yml` points to, checking its md5.

```bash
ferrule rescue [REPO] [--out PATH]
```

Run it today even if you aren't switching yet: the spec lives in a bucket you don't control.
The default output is `openapi.<ext>` in the repository.

## generate

Generate an SDK from a spec and a config into a new directory.

```bash
ferrule generate --spec SPEC --config STAINLESS_YML --out DIR [--runtime-from SDK_CHECKOUT] [--gen PATH]
```

`--out` must be new or empty. `--runtime-from` copies the runtime (`_base_client.py` and
friends) from an existing SDK, which is what adopt mode does. The command prints the package
name, the file count and a sha256 of the whole tree: the same inputs always give the same digest.

## verify

Compare the public API of two SDK versions.

```bash
ferrule verify --against OLD --new NEW_DIR [--repo REPO] [--package NAME] [--json REPORT]
```

| Flag | Meaning |
| --- | --- |
| `--against` | The old SDK: a directory, or a git ref in `--repo`. |
| `--new` | The new SDK directory. |
| `--package` | The import name, when it can't be detected. |
| `--json` | Also write the full report, with every change, to this file. |

```text
public API: 0 breaking · 6 additive
text closeness: 97.8%
```

`verify` reads the code with griffe and never imports it, so it is safe on untrusted trees.

## merge

Three-way merge of a new generation with your custom changes.

```bash
ferrule merge --base OLD_GEN --ours CUSTOM --theirs NEW_GEN [--repo REPO] [--out DIR] [--report JSON] [--keep GLOB]
```

Each side is a directory or a git ref. Files under any `lib/` directory and anything matching `--keep`
(repeatable, like `keep_files` in `stainless.yml`) are never overwritten. Conflicts are printed
with their file and reason.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success. |
| 1 | A diagnostic was reported, `verify` found a breaking change, or `merge` left a conflict. |
| 2 | The command isn't implemented in this version. |

## Errors

Every error is printed as `ferrule <command>: FER<code> <message>`, with a pointer into your spec
or config and a suggested fix.

| Codes | Area |
| --- | --- |
| `FER11xx` to `FER15xx` | `stainless.yml`: schema, transforms, client, streaming, pagination |
| `FER201` | A spec or config feature Ferrule can't generate yet |
| `FER40xx` | Inputs and tools: missing files, `ferrule-gen` not found, unsafe paths |
| `FER41xx` | Rescue and adopt: hash mismatch, bad `.stats.yml`, patch conflicts, git failures |
