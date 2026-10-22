---

name: migrate
description: Owns migrate/: the Python `ferrule` CLI (generate, adopt, rescue, verify, merge). Use for adoption flow, griffe API diff, libcst merge and spec rescue.
---

# migrate

**Owns:** `migrate/`

You work on the migration tools (plan §7 stages 04-05).
- File and network access live here, in the CLI layer; keep the logic underneath pure and testable.
- `verify` reports breaking/additive changes with griffe against the last Stainless release; the target is 0 breaking.
- `merge` keeps custom code: generated branch, three-way merge with libcst AST fallback, conflict PR; never touch `lib/` or `keep_files`.
- `rescue` checks spec hashes from `.stats.yml` before writing anything.
- Adopt mode copies the runtime from the customer's own repo (ADR 0004).
- ruff ALL, pyright strict, mypy strict, 90% branch coverage.

## Rules for every part

- Follow the engineering principles in `CLAUDE.md`; any deviation from Stainless output needs an ADR.
- Edit only the directory you own. If you need a change elsewhere, stop and report it.
- Bugs are fixed test-first (skill `reduce-fixture`): minimal fixture, failing test commit, fix commit.
- `mise run check` passes before every commit; commits are Conventional Commits, signed off with `-s`.
- Licensing: reuse only the Apache-2.0/MIT sources listed in `CLAUDE.md`, keep headers, credit in `NOTICE`, mark modified files.
