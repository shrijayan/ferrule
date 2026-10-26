---
title: Overview
description: What Ferrule is, how far along it is, and where to start reading.
---

Ferrule is an open-source toolkit for teams leaving Stainless. It turns an OpenAPI spec and a
`stainless.yml` into the same SDKs Stainless would have generated, in every major Stainless
language, then proves that nothing public changed. Forge, Cloudflare's Apache-2.0 generator
engine, does the parsing; Ferrule adds the config converter, a language-neutral model, one plugin
per language that writes Stainless-shaped code, and the migration tools. The core is Rust: one
`ferrule-gen` binary of 8 MB with Forge built in, so generation needs no Node and no
WebAssembly. Everything runs on your machine and in your CI. There is no hosted service and no
account.

## Status

Ferrule is **pre-alpha**. All eight SDK languages are on `main`. In Python the converter, plugin
and runtime are complete, and the OneBusAway SDK regenerates with 0 breaking changes while all
1,110 of its own tests pass. Adoption is being hardened against more real SDKs; the
[fidelity board](/docs/fidelity/) shows where each Python SDK stands tonight.

Each language is proven against real Stainless SDKs; numbers are identical files out of the total.

| Language | Status | Proof |
| --- | --- | --- |
| Python | on `main` | onebusaway 80/80, 0 breaking, 1,110 tests pass; the Phase 4 gate adopts five real SDKs with 0 breaking, every test passing and custom code unchanged |
| TypeScript | on `main` | isaacus-typescript 84/84, its lint and 216 jest tests pass; onebusaway-typescript 108/109 |
| Go | on `main` | onebusaway-go runtime 25/25, generated 65/69, builds and tests pass; imagekit-go runtime 54/54, generated 17/45 |
| Java, Kotlin | on `main` | onebusaway-java 328/343, onebusaway-kotlin 282/338, imagekit-java 186/291 |
| Ruby | on `main` | onebusaway-ruby 326/329, imagekit-ruby 349/432; both gems' tests pass |
| PHP | on `main` | imagekit-php runtime 48/48, generated 271 of 340 shared files (319 of 388 with the runtime; 82 more under other names); 87 pest tests pass |
| C# | on `main` | imagekit-dotnet runtime 38/38, generated 109/184 (38 more differ only in doc wrapping); builds with dotnet |
| CLI, Terraform | not started | no public Stainless-generated pair with a public config, so no oracle to prove against |

Ferrule is not on PyPI yet. Until the 0.1 release you run it from a clone of this repository, as
the [quickstart](/docs/quickstart/) shows.

## Where to go next

| If you want to | Read |
| --- | --- |
| Move an existing Stainless Python SDK to Ferrule | [Quickstart](/docs/quickstart/) |
| Look up a command or a flag | [Commands](/docs/commands/) |
| Keep the SDK regenerating on every spec change | [GitHub Action](/docs/action/) |
| Understand the pipeline | [Architecture](/docs/architecture/) |
| See how close each public SDK is | [Fidelity board](/docs/fidelity/) |
| Contribute | [CONTRIBUTING.md](https://github.com/shrijayan/ferrule/blob/main/CONTRIBUTING.md) |
