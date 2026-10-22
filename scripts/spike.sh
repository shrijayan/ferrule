#!/usr/bin/env bash
# Phase 1 exit gate (docs/phases/phase-1.md): generate OneBusAway's
# `agencies_with_coverage` resource from the pinned oracle pair through
# converter -> Forge -> plugin, then check it the way the plan's compile gate
# does (ruff, pyright strict, mypy strict) inside a copy of the oracle SDK, and
# diff it against the oracle's own files.
#
# Oracle code is data: it is copied and type-checked, never installed or run.
# Only the oracle's declared third-party dependencies are installed, by name.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cfg_dir="$root/oracles/cache/onebusaway-config"
sdk_dir="$root/oracles/cache/onebusaway-python"
out="$root/.cache/spike"
forge="$root/.cache/forge/target/release/forge"
wasm="$root/python-plugin/dist/plugin.wasm"

python3 "$root/oracles/fetch.py" onebusaway >/dev/null
[ -x "$forge" ] || bash "$root/scripts/forge-setup.sh"
pnpm --dir "$root" --filter @ferrule/python-plugin build >/dev/null

rm -rf "$out"
mkdir -p "$out"

# 1. The spike's slice of stainless.yml: one resource plus $shared and the python target.
uv run --quiet --with pyyaml==6.0.3 python - "$cfg_dir/stainless-config.yml" "$out/stainless.yml" <<'PY'
import sys

import yaml

with open(sys.argv[1]) as f:
    config = yaml.safe_load(f)
keep = ("agencies_with_coverage", "$shared")
config["resources"] = {k: v for k, v in config["resources"].items() if k in keep}
with open(sys.argv[2], "w") as f:
    yaml.safe_dump(config, f, sort_keys=False)
PY

# 2. Convert, 3. Forge + plugin.
node "$root/converter/src/cli.ts" "$cfg_dir/openapi.yml" "$out/stainless.yml" -o "$out/annotated.json"
"$forge" "$out/annotated.json" --target "wasm://$wasm" --out "$out/generated"

# 4. Drop the generated files into a copy of the oracle SDK (its runtime and tool configs).
mkdir -p "$out/sdk"
(cd "$sdk_dir" && git ls-files -z | xargs -0 tar cf -) | tar xf - -C "$out/sdk"
generated=()
while IFS= read -r -d '' f; do
  rel="${f#"$out/generated/"}"
  mkdir -p "$(dirname "$out/sdk/$rel")"
  cp "$f" "$out/sdk/$rel"
  generated+=("$rel")
done < <(find "$out/generated" -name '*.py' -print0 | sort -z)
echo "generated ${#generated[@]} files"

# Third-party dependencies only, by name; the oracle package itself is not installed.
uv venv --quiet --python 3.12 "$out/venv"
uv run --quiet --no-project python - "$sdk_dir/pyproject.toml" >"$out/requirements.txt" <<'PY'
import sys
import tomllib

with open(sys.argv[1], "rb") as f:
    print("\n".join(tomllib.load(f)["project"]["dependencies"]))
PY
uv pip install --quiet --python "$out/venv/bin/python" -r "$out/requirements.txt"

status=0
cd "$out/sdk"
echo "== ruff (oracle's config)"
uvx ruff@0.17.1 check "${generated[@]}" || status=1
uvx ruff@0.17.1 format --check "${generated[@]}" || status=1
echo "== pyright strict"
PYTHONPATH=src uvx pyright@1.1.414 --pythonpath "$out/venv/bin/python" "${generated[@]}" || status=1
echo "== mypy strict"
MYPYPATH=src uvx mypy@2.4.0 --strict --python-executable "$out/venv/bin/python" \
  --follow-imports=silent --explicit-package-bases "${generated[@]}" || status=1

# 5. Diff against the oracle, ignoring only the generated-by header line.
echo "== diff vs oracle (header line excluded)"
# The slice's client and package files list one resource, so they differ from the full SDK by
# design; `mise run parity -- onebusaway` checks the whole tree. Compare the spike's files only.
for rel in "${generated[@]}"; do
  case "$rel" in
    */resources/__init__.py | */types/__init__.py) continue ;;
    src/*/resources/*.py | src/*/types/*) ;;
    *) continue ;;
  esac
  if diff -q <(tail -n +2 "$sdk_dir/$rel") <(tail -n +2 "$out/generated/$rel") >/dev/null; then
    echo "identical  $rel"
  else
    echo "DIFFERENT  $rel"
    diff -u <(tail -n +2 "$sdk_dir/$rel") <(tail -n +2 "$out/generated/$rel") | head -40
    status=1
  fi
done

[ "$status" -eq 0 ] && echo "spike: OK"
exit "$status"
