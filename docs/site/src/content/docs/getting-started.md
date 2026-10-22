---

title: Getting started
description: Adopt Ferrule for an existing Stainless Python SDK.
---

:::caution
Ferrule is in Phase 0 (production baseline). The commands below describe the intended flow and
do not work yet.
:::

## For a team leaving Stainless

```bash
uvx ferrule adopt .
uvx ferrule verify --against v2.14.0
```

`adopt` reads `.stats.yml`, rescues the spec (checking its hash), converts `stainless.yml`, copies
the runtime from your own repository, records hand edits under `.ferrule/patches/`, generates via
Forge and the Ferrule Python plugin, and opens a pull request. `verify` reports public API changes
against your last Stainless release, runs your tests on the new output, and replays recorded HTTP
sessions.

## For contributors

```bash
mise install
mise run install
mise run check
```

See [CONTRIBUTING.md](https://github.com/ferrule-dev/ferrule/blob/main/CONTRIBUTING.md).
