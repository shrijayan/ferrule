#!/usr/bin/env bash
# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
# Run the action's scripts against a fixture verify report, offline: validate inputs,
# compute the label, print the PR plan. `act` is not required.
#
#   action/scripts/dry_run.sh [report.json] [labels]
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
report="${1:-$here/tests/fixtures/additive.json}"
labels="${2:-}"
out="$(mktemp)"
trap 'rm -f "$out"' EXIT

INPUT_SPEC=openapi.json INPUT_CONFIG=stainless.yml INPUT_VERIFY=true INPUT_OPEN_PR=true \
  INPUT_PYTHON_VERSION=3.12 INPUT_FERRULE_VERSION=source INPUT_AGAINST="" GH_TOKEN=dry-run \
  bash "$here/validate.sh"

echo "--- label ($report)"
status=0
python3 "$here/label.py" "$report" --enforce true --labels "$labels" --github-output "$out" || status=$?

label="" breaking="" additive=""
# shellcheck disable=SC1090
source <(grep -E '^(label|breaking|additive)=' "$out")
[[ "$status" -le 1 ]] || exit "$status"

echo "--- PR plan"
LABEL="$label" BREAKING="$breaking" ADDITIVE="$additive" WORKSPACE="$PWD" DRY_RUN=1 \
  bash "$here/open_pr.sh"

if [[ "$status" -eq 1 ]]; then
  echo "--- job fails after opening the PR: unlabeled breaking change"
  exit 1
fi
echo "--- job passes"
