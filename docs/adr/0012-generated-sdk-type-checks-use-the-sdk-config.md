# 0012. Generated SDKs are type-checked with their own configuration

- Status: Accepted
- Date: 2026-10-23
- Refines: plan §11 ("Generated SDKs must pass ruff, pyright strict and mypy strict")

## Context

Stainless SDKs ship their own `[tool.ruff]`, `[tool.pyright]` (strict) and
`[tool.mypy]` settings. The oracle's `_client.py` imports names that
`_types.py` and `_base_client.py` re-export implicitly; bare `mypy --strict`
(which adds `--no-implicit-reexport`) rejects that, while the SDK's own mypy
configuration accepts it. Our output must be byte-identical to the oracle.

## Decision

The compile gate runs ruff, pyright and mypy on generated code with the
generated SDK's own configuration (which we emit identically to Stainless's:
pyright strict, mypy strict options as configured there). Ferrule's own code
stays under our stricter root settings.

## Consequences

"Passes the compile gate" means "passes the checks the SDK's maintainers
run", which is what users of the SDK see. Tightening beyond that would make
output differ from Stainless.
