# Security policy

## Supported versions

Ferrule is pre-1.0. Only the latest release and `main` receive security fixes.

## Reporting a vulnerability

Please do **not** open a public issue. Report privately through
[GitHub private vulnerability reporting](https://github.com/shrijayan/ferrule/security/advisories/new)
or by email to `security@ferrule.dev` (placeholder until the domain is registered).

Include the affected component (converter, python-plugin, runtime, migrate, action), version or
commit, and a minimal reproduction. We acknowledge reports within 3 business days and aim to ship
a fix or mitigation within 30 days, coordinating disclosure with you.

## Scope notes

- The runtime is copied into every generated SDK; vulnerabilities there affect downstream SDKs and
  are handled with highest priority, including advisories for regenerated SDKs.
- Ferrule never sends telemetry and never needs a token or login. Anything that contacts the
  network outside `rescue`, `adopt` and Git operations is a bug.
- Releases are built in CI with provenance (PyPI trusted publishing, npm provenance, Sigstore) and
  ship a CycloneDX SBOM.
