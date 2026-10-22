# Governance

## Today: BDFL

Ferrule is started and led by a single maintainer acting as BDFL (benevolent dictator for life).
The BDFL owns the SDK model, any deviation from Stainless output, releases, and the relationship
with the Forge team. Decisions of lasting impact are written down as ADRs in `docs/adr/`.

## Next: maintainers

Contributors with a sustained record of reviewed contributions in one part (converter,
python-plugin, runtime, migrate, CI/docs) may be invited as maintainers for that part, listed in
`.github/CODEOWNERS`. Once there are at least three maintainers, decisions move to lazy consensus
among maintainers, with the BDFL as tie-breaker; the BDFL role may then be retired by a vote of the
maintainers.

## Contributions and licence

- Licence: Apache-2.0. Contributions are accepted under the same licence.
- Developer Certificate of Origin (DCO): every commit is signed off (`git commit -s`).
- **No CLA.** Contributors keep their copyright.
- **Never relicense.** Ferrule will not change its licence, and no contributor agreement allows it.
- Never: required hosted generation, licence tokens or login, telemetry.

## Upstream

If Cloudflare wants to take pieces of Ferrule into Forge, maintainers will help upstream them
under Apache-2.0. Ferrule continues independently for whatever is not upstreamed.

## Code of conduct

All spaces follow the [Code of Conduct](./CODE_OF_CONDUCT.md).
