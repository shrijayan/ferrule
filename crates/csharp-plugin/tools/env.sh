# The formatter's environment, sourced by install.sh and bin/csharpier: the .NET SDK that
# mise.toml pins (when mise is there) and csharpier installed into <repo>/.cache/csharp-tools
# (gitignored), with its NuGet packages kept there too.

tools="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$tools/../../.." && pwd)"
export CSHARP_TOOLS="$root/.cache/csharp-tools"
export NUGET_PACKAGES="$CSHARP_TOOLS/nuget"
export DOTNET_CLI_HOME="$CSHARP_TOOLS/home"
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_NOLOGO=1
export DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1

# The pinned .NET SDK first on the PATH (looked up from the repository; the command keeps its own
# working directory).
if command -v mise >/dev/null 2>&1; then
  if dotnet_home="$(cd "$root" && mise where dotnet 2>/dev/null)"; then
    export DOTNET_ROOT="$dotnet_home"
    export PATH="$dotnet_home:$PATH"
  fi
fi

# The csharpier version the oracles' .config/dotnet-tools.json pins.
csharpier_version="$(sed -n 's/.*"version": *"\([0-9][0-9.]*\)".*/\1/p' "$tools/dotnet-tools.json" | tail -n 1)"

ferrule_csharpier_installed() {
  if [ ! -x "$CSHARP_TOOLS/csharpier" ]; then
    echo "csharpier is not installed in .cache/csharp-tools: run \`mise run csharp:tools\`" >&2
    exit 127
  fi
}
