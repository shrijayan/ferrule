#!/usr/bin/env bash
# PostToolUse hook: format the file an Edit/Write just touched.
# Never fails the tool call: missing tools or formatter errors are ignored.
set -u
input="$(cat)"
file="$(printf '%s' "$input" | python3 -c 'import json,sys
try: print(json.load(sys.stdin).get("tool_input",{}).get("file_path",""))
except Exception: print("")' 2>/dev/null)"
[ -n "$file" ] && [ -f "$file" ] || exit 0
root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$root" 2>/dev/null || exit 0
case "$file" in
  *.ts|*.tsx|*.js|*.mjs|*.cjs|*.json|*.jsonc|*.css)
    if [ -x node_modules/.bin/biome ]; then
      node_modules/.bin/biome format --write "$file" >/dev/null 2>&1 || true
    fi ;;
  *.py|*.pyi)
    if command -v uv >/dev/null 2>&1; then
      uv run --no-sync ruff format "$file" >/dev/null 2>&1 || true
    elif command -v ruff >/dev/null 2>&1; then
      ruff format "$file" >/dev/null 2>&1 || true
    fi ;;
esac
exit 0
