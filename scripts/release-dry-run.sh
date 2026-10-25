#!/usr/bin/env bash
# Local rehearsal of release.yml: build Python dists and ferrule-gen, SBOM, workflow audit.
# Publishes nothing. Packages that do not exist yet are skipped with a note.
set -euo pipefail
cd "$(dirname "$0")/.."

note() { printf '  - %s\n' "$*"; }
step() { printf '\n==> %s\n' "$*"; }

rm -rf dist
mkdir -p dist

step "Python sdists + wheels (uv build)"
built_py=0
for pkg in runtime migrate; do
  if [ -f "$pkg/pyproject.toml" ]; then
    uv build --quiet --out-dir dist "$pkg"
    built_py=1
  else
    note "skip $pkg: $pkg/pyproject.toml not found"
  fi
done
ls -1 dist 2>/dev/null | sed 's/^/  /' || true

step "ferrule-gen (cargo build --release)"
cargo build --release --locked --quiet -p ferrule-gen
note "$(target/release/ferrule-gen --version)"

step "CycloneDX SBOM"
if [ "$built_py" -eq 1 ] && uvx --quiet cyclonedx-py --version >/dev/null 2>&1; then
  venv="$(mktemp -d)/sbom-venv"
  uv venv --quiet "$venv"
  uv pip install --quiet --python "$venv" dist/*.whl
  uvx --quiet cyclonedx-py environment --output-format JSON --output-file dist/sbom.cdx.json "$venv"
  rm -rf "$(dirname "$venv")"
  note "wrote dist/sbom.cdx.json"
elif [ "$built_py" -eq 0 ]; then
  note "skip SBOM: no Python wheels built"
else
  note "skip SBOM: 'uvx cyclonedx-py' is unavailable (offline?)"
fi

step "Workflow audit (zizmor)"
if [ -d .github/workflows ] && uvx --quiet zizmor --version >/dev/null 2>&1; then
  uvx --quiet zizmor --offline .github
else
  note "skip zizmor: unavailable or no workflows"
fi

step "Release dry run OK"
