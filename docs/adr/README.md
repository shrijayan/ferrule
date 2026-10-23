# Architecture Decision Records

Decisions are recorded in [MADR](https://adr.github.io/madr/) style. Copy
[template.md](./template.md) (or use the `add-adr` skill) and add a row below. Any deviation from
Stainless output needs an ADR.

| #                                         | Title                                                                                  | Status   | Date       |
| ----------------------------------------- | -------------------------------------------------------------------------------------- | -------- | ---------- |
| [0001](./0001-languages.md)               | Languages: TypeScript for converter and plugin, Python for runtime and migration tools | Accepted | 2026-10-22 |
| [0002](./0002-adapter-model.md)           | Adapter model: only adapter/ imports Forge; emitters see SdkModel                      | Accepted | 2026-10-22 |
| [0003](./0003-extension-names.md)         | Extension names: the x-ferrule-* namespace                                             | Accepted | 2026-10-22 |
| [0004](./0004-runtime-source.md)          | Runtime source: Apache-2.0 Stainless-generated SDK runtimes, with attribution          | Accepted | 2026-10-22 |
| [0005](./0005-pin-forge-by-git-commit.md) | Pin Forge by git commit and build from source                                          | Accepted | 2026-10-22 |
| [0006](./0006-forge-telemetry-check.md)   | Forge telemetry check                                                                  | Accepted | 2026-10-22 |
| [0007](./0007-phase-1-spike-resource.md) | Phase 1 spike resource and synthetic pagination | Accepted | 2026-10-23 |
| [0008](./0008-dependabot-instead-of-renovate.md) | Dependabot instead of Renovate | Accepted | 2026-10-23 |
| [0009](./0009-oracle-configs.md) | Which oracle configs Phase 2 converts | Accepted | 2026-10-23 |
| [0010](./0010-oracle-snapshot-digests.md) | Converter oracle snapshots store digests, not specs | Accepted | 2026-10-23 |
| [0011](./0011-keep-files-the-generator-no-longer-writes.md) | Keep files the generator no longer writes | Accepted | 2026-10-23 |
| [0012](./0012-generated-sdk-type-checks-use-the-sdk-config.md) | Generated SDKs are type-checked with their own configuration | Accepted | 2026-10-23 |
| [0013](./0013-sdk-testing-fake-server.md) | Ship a `<pkg>.testing` fake server in generated SDKs | Accepted | 2026-10-23 |
