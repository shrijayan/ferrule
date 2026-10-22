#!/usr/bin/env bash
# Fail if any non-merge commit in BASE..HEAD lacks a Signed-off-by trailer matching its author.
# Usage: check-dco.sh <base-sha> <head-sha>
set -euo pipefail
base="$1"
head="$2"
status=0
for sha in $(git rev-list --no-merges "${base}..${head}"); do
  author_email=$(git log -1 --format='%ae' "$sha")
  if ! git log -1 --format='%(trailers:key=Signed-off-by,valueonly)' "$sha" | grep -qiF "<${author_email}>"; then
    echo "::error::commit ${sha} has no Signed-off-by for ${author_email} (use git commit -s)"
    status=1
  fi
done
[ "$status" -eq 0 ] && echo "All commits are signed off."
exit "$status"
