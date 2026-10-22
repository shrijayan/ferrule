#!/usr/bin/env bash
# Emit `specs=<json array>` of Tier 1 spec ids from corpus/manifest.yaml for the corpus.yml matrix.
# Prints `specs=[]` when the manifest does not exist yet.
set -euo pipefail
manifest="${1:-corpus/manifest.yaml}"
if [ ! -f "$manifest" ]; then
  echo "specs=[]"
  exit 0
fi
specs=$(uvx --quiet --with pyyaml python -I -c '
import json, sys, yaml
data = yaml.safe_load(open(sys.argv[1])) or {}
items = data.get("specs", data) if isinstance(data, dict) else data
ids = [s["id"] for s in (items or []) if isinstance(s, dict) and s.get("tier") == 1 and "id" in s]
print(json.dumps(ids))
' "$manifest")
echo "specs=${specs}"
