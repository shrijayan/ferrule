# The formatter bundle's environment, sourced by install.sh and bin/*: the Gemfile and
# Gemfile.lock next to this file, installed into <repo>/.cache/ruby-tools (gitignored), with the
# Ruby that mise.toml pins when mise is there.

tools="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$tools/../../.." && pwd)"
export BUNDLE_GEMFILE="$tools/Gemfile"
export BUNDLE_PATH="$root/.cache/ruby-tools"
# Bundler's config goes with the bundle, not into tools/.bundle.
export BUNDLE_APP_CONFIG="$root/.cache/ruby-tools"
export BUNDLE_FROZEN=true
# syntax_tree reads the files in the locale's encoding; the generated files are UTF-8.
export RUBYOPT="--encoding=UTF-8${RUBYOPT:+ $RUBYOPT}"

# The pinned Ruby first on the PATH (looked up from the repository; the command keeps its own
# working directory).
if command -v mise >/dev/null 2>&1; then
  if ruby_home="$(cd "$root" && mise where ruby 2>/dev/null)"; then
    export PATH="$ruby_home/bin:$PATH"
  fi
fi

ferrule_bundle_installed() {
  if [ ! -d "$BUNDLE_PATH/ruby" ]; then
    echo "the Ruby plugin's formatters are not installed in .cache/ruby-tools: run \`mise run ruby:tools\`" >&2
    exit 127
  fi
}
