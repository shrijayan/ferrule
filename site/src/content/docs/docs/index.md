---
title: Overview
description: What Ferrule is, how far along it is, and where to start reading.
---

Ferrule is an open-source toolkit for teams leaving Stainless. It turns an OpenAPI spec and a
`stainless.yml` into the same Python SDK Stainless would have generated, then proves that nothing
public changed. Forge, Cloudflare's Apache-2.0 generator engine, does the parsing; Ferrule adds
the config converter, a Python plugin that writes Stainless-shaped code, and the migration tools.
Everything runs on your machine and in your CI. There is no hosted service and no account.

## Status

Ferrule is **pre-alpha**. The converter, plugin and runtime are complete, and the OneBusAway SDK
regenerates with 0 breaking changes while all 1,110 of its own tests pass. Adoption is being
hardened against more real SDKs; the [fidelity board](/ferrule/docs/fidelity/) shows where each
one stands tonight.

Ferrule is not on PyPI yet. Until the 0.1 release you run it from a clone of this repository, as
the [quickstart](/ferrule/docs/quickstart/) shows.

## Where to go next

| If you want to | Read |
| --- | --- |
| Move an existing Stainless Python SDK to Ferrule | [Quickstart](/ferrule/docs/quickstart/) |
| Look up a command or a flag | [Commands](/ferrule/docs/commands/) |
| Keep the SDK regenerating on every spec change | [GitHub Action](/ferrule/docs/action/) |
| Understand the pipeline | [Architecture](/ferrule/docs/architecture/) |
| See how close each public SDK is | [Fidelity board](/ferrule/docs/fidelity/) |
| Contribute | [CONTRIBUTING.md](https://github.com/shrijayan/ferrule/blob/main/CONTRIBUTING.md) |
