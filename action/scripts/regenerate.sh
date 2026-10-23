#!/usr/bin/env bash
# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
# Generate the SDK (runtime taken from the repo itself), verify it against the previous
# release, then copy it over the working tree. Writes the verify report to $REPORT.
#
# Env: INPUT_SPEC, INPUT_CONFIG, INPUT_AGAINST (optional git ref), FERRULE_VERSION,
# WORKSPACE, OUT, REPORT. FERRULE_FORGE / FERRULE_PLUGIN pass through to `ferrule generate`.
set -euo pipefail

: "${WORKSPACE:?}" "${OUT:?}" "${REPORT:?}"
rm -rf "$OUT"

if [[ "${FERRULE_VERSION:-}" == "source" ]]; then
  # Dogfooding: the converter comes from the same checkout as the action.
  FERRULE_CONVERTER="$(cd "$GITHUB_ACTION_PATH/.." && pwd)/converter/src/cli.ts"
  export FERRULE_CONVERTER
fi

against="${INPUT_AGAINST:-}"
if [[ -z "$against" ]]; then
  # The latest release tag if the repo has one, else the commit being regenerated.
  against="$(git -C "$WORKSPACE" describe --tags --abbrev=0 2>/dev/null || echo HEAD)"
fi
echo "verifying against $against"

ferrule generate \
  --spec "$WORKSPACE/$INPUT_SPEC" \
  --config "$WORKSPACE/$INPUT_CONFIG" \
  --out "$OUT" \
  --runtime-from "$WORKSPACE"

# verify exits 1 on breaking changes; the label step owns that decision.
status=0
ferrule verify --repo "$WORKSPACE" --against "$against" --new "$OUT" --json "$REPORT" || status=$?
if [[ "$status" -gt 1 || ! -s "$REPORT" ]]; then
  echo "::error::ferrule verify failed (exit $status)" >&2
  exit 1
fi

# Adopt mode: overlay generated files; never delete hand-written ones (lib/, keep_files).
(cd "$OUT" && tar --exclude=.git -cf - .) | (cd "$WORKSPACE" && tar -xf -)
