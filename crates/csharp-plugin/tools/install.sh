#!/usr/bin/env bash
# `mise run csharp:tools`: installs the csharpier that dotnet-tools.json pins into
# <repo>/.cache/csharp-tools (the network once), then checks that bin/csharpier runs.
set -euo pipefail
# shellcheck source=env.sh
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"
mkdir -p "$CSHARP_TOOLS"
installed="$("$CSHARP_TOOLS/csharpier" --version 2>/dev/null || true)"
if [ "$installed" != "$csharpier_version" ]; then
  if [ -n "$installed" ]; then
    dotnet tool uninstall csharpier --tool-path "$CSHARP_TOOLS" >/dev/null
  fi
  dotnet tool install csharpier --version "$csharpier_version" --tool-path "$CSHARP_TOOLS"
fi
"$tools/bin/csharpier" --version
