#!/usr/bin/env bash
# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
# Generate the SDK (runtime taken from the repo itself), verify it against the previous
# release, then copy it over the working tree. Writes the verify report to $REPORT.
#
# Env: INPUT_SPEC, INPUT_CONFIG, INPUT_AGAINST (optional git ref), WORKSPACE, OUT, REPORT.
# FERRULE_GEN (a ferrule-gen binary) and FERRULE_CONVERT pass through to `ferrule generate`.
set -euo pipefail

: "${WORKSPACE:?}" "${OUT:?}" "${REPORT:?}"
rm -rf "$OUT"

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

# The only paths open_pr.sh may commit.
(cd "$OUT" && find . -type f ! -path './.git/*' | sed 's#^\./##' | sort) > "${MANIFEST:?}"

# Adopt mode: overlay generated files; never delete hand-written ones (lib/, keep_files).
(cd "$OUT" && tar --exclude=.git -cf - .) | (cd "$WORKSPACE" && tar -xf -)
