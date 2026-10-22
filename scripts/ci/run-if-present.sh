#!/usr/bin/env bash
# Run a mise task only if it is defined; used by CI gates that land in later phases
# (golden, determinism, parity, corpus). Usage: run-if-present.sh <task> [args...]
set -euo pipefail
task="$1"
shift
if mise tasks ls --no-header 2>/dev/null | awk '{print $1}' | grep -qx -- "$task"; then
  exec mise run "$task" "$@"
fi
echo "::notice::mise task '$task' not defined yet; skipping"
