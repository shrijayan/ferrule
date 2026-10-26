#!/usr/bin/env bash
# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
# Regenerate the SDK on a ferrule/regenerate branch: `ferrule regenerate` merges the new
# generation three-way into it, from the last generation the branch took, so custom code and
# keep_files survive. Then verify the merged tree against the previous release; the verify
# report goes to $REPORT. A merge conflict is left on its own branch, checked out, for the PR.
#
# Env: INPUT_SPEC, INPUT_CONFIG, INPUT_AGAINST (optional git ref), WORKSPACE, REPORT,
# GITHUB_OUTPUT (changed, conflict). FERRULE_GEN (a ferrule-gen binary) passes through.
set -euo pipefail

: "${WORKSPACE:?}" "${REPORT:?}"
BRANCH="ferrule/regenerate"

if [[ -z "${FERRULE_GEN:-}" ]]; then
  # No released ferrule-gen binary yet: build it from the action's own checkout (the runner's
  # rustup picks the toolchain from rust-toolchain.toml). A cold release build takes minutes.
  src="$(cd "$GITHUB_ACTION_PATH/.." && pwd)"
  echo "building ferrule-gen in $src"
  (cd "$src" && cargo build --release --locked --quiet -p ferrule-gen)
  FERRULE_GEN="$src/target/release/ferrule-gen"
  export FERRULE_GEN
fi

against="${INPUT_AGAINST:-}"
if [[ -z "$against" ]]; then
  # The latest release tag if the repo has one, else the commit being regenerated.
  against="$(git -C "$WORKSPACE" describe --tags --abbrev=0 2>/dev/null || echo HEAD)"
fi
echo "verifying against $against"

cd "$WORKSPACE"
git config user.name >/dev/null || git config user.name "github-actions[bot]"
git config user.email >/dev/null \
  || git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
# The generations so far (pushed by earlier runs; `fetch-depth: 0` fetches the branch).
if git rev-parse --verify --quiet refs/remotes/origin/generated >/dev/null; then
  git branch --force generated refs/remotes/origin/generated
fi
start="$(git rev-parse HEAD)"
git switch --quiet --force-create "$BRANCH"

output() { if [[ -n "${GITHUB_OUTPUT:-}" ]]; then echo "$1" >> "$GITHUB_OUTPUT"; fi; }

status=0
ferrule regenerate \
  --repo "$WORKSPACE" \
  --spec "$WORKSPACE/$INPUT_SPEC" \
  --config "$WORKSPACE/$INPUT_CONFIG" || status=$?
if [[ "$status" -eq 1 ]]; then
  conflict="$(git for-each-ref --format='%(refname:short)' 'refs/heads/ferrule/conflict-*' | head -n 1)"
  [[ -n "$conflict" ]] || { echo "::error::ferrule regenerate failed" >&2; exit 1; }
  echo "::warning::the regeneration conflicts with custom code; the PR carries the markers"
  git switch --quiet "$conflict"
  output "changed=true"
  output "conflict=true"
  # Markers don't parse: the PR is for a person to resolve, so verify runs on its next push.
  echo '{"breaking_count": 0, "additive_count": 0, "breaking": [], "additive": []}' > "$REPORT"
  exit 0
elif [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

if [[ "$(git rev-parse HEAD)" == "$start" ]]; then output "changed=false"; else output "changed=true"; fi
output "conflict=false"

# verify exits 1 on breaking changes; the label step owns that decision.
status=0
ferrule verify --repo "$WORKSPACE" --against "$against" --new "$WORKSPACE" --json "$REPORT" || status=$?
if [[ "$status" -gt 1 || ! -s "$REPORT" ]]; then
  echo "::error::ferrule verify failed (exit $status)" >&2
  exit 1
fi
