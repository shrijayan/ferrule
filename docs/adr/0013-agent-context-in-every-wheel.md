# 0013. Ship agent context in every generated SDK

- Status: Accepted
- Date: 2026-10-23

## Context

Plan §5 item 4: generated SDKs carry agent context (`SKILL.md`, `AGENTS.md`,
`llms.txt` and tested recipes) inside the wheel, locked to the package version.
Stainless output has none of these files, so emitting them deviates from the
oracle output, which needs an ADR.

## Decision

`python-plugin/src/emit/agent/` emits, from SdkModel only:

- `src/<pkg>/_agent/SKILL.md`, `AGENTS.md` and `llms.txt`. They live inside the
  package, so hatch's `packages = ["src/<pkg>"]` puts them in the wheel without
  a `pyproject.toml` change.
- With project metadata, the repository copies from the plan §10 layout
  (`skills/<pkg>/SKILL.md`, `AGENTS.md`, `llms.txt`), one runnable recipe per
  plain example in `examples/agent/`, and `tests/test_agent_recipes.py`, which
  runs each recipe against a respx mock.

The files are deterministic: the version comes from project metadata, and
nothing is written from a clock or an LLM.

## Consequences

The files are additive. No existing module or public name changes, so
`ferrule verify` still reports 0 breaking changes for OneBusAway. Text
closeness counts the new files as unmatched lines, which lowers OneBusAway's
score from 94.07% to 91.46%. Paginated and
streaming snippets are only parsed, not run, because an empty mock response is
not a valid page or stream. A repository that has its own `AGENTS.md` or
`llms.txt` gets ours instead, so adopt mode should treat those paths as user
files if that comes up.
