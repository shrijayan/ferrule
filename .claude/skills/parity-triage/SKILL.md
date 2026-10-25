---

name: parity-triage
description: Triage a failing oracle parity or nightly corpus run: classify the diff, find the owning part, and open or update an issue. Use when parity.yml or corpus.yml fails.
---

# Parity triage

1. Read the report: griffe API diff, the repo's own test results, HTTP replay, normalized text diff.
2. Classify each difference:
   - **breaking public API** (removed or renamed symbol, signature, default, exception class): highest priority;
   - **behavior** (replay mismatch: retries, pagination, headers);
   - **cosmetic** (formatting, comments, ordering): fix only if the text diff matters.
3. Attribute to a part: config interpretation to converter (`crates/converter`); file content to python-plugin (`crates/python-plugin`); runtime behavior to runtime; custom-code loss to migrate; Forge parsing upstream.
4. Check for a Forge version bump or oracle pin change first; those explain most sudden regressions.
5. Open or update one issue per root cause with the FER code, oracle, file path and minimal diff.
6. Hand off to `reduce-fixture` before any fix. Deviations we intend to keep need an ADR (Stainless output is the spec).
