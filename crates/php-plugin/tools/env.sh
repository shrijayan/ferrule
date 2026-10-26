# The formatter's environment, sourced by install.sh and bin/*: composer.json and composer.lock
# next to this file, installed into <repo>/.cache/php-tools (gitignored), with the PHP (and its
# bundled composer) pinned here when mise is there. It is not in mise.toml's tools: mise builds
# PHP from source, which `mise install` and every CI job would otherwise do.

# imagekit-php's CI and composer platform use PHP 8.3.
php_version="8.3.36"

tools="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$tools/../../.." && pwd)"
export COMPOSER="$tools/composer.json"
export COMPOSER_VENDOR_DIR="$root/.cache/php-tools"
# Composer's caches and global config go with the install, not into the user's home.
export COMPOSER_HOME="$root/.cache/php-tools/.composer"
export COMPOSER_NO_INTERACTION=1

# The pinned PHP first on the PATH (looked up from the repository; the command keeps its own
# working directory).
if command -v mise >/dev/null 2>&1; then
  if php_home="$(cd "$root" && mise where "php@$php_version" 2>/dev/null)"; then
    export PATH="$php_home/bin:$PATH"
  fi
fi

ferrule_tools_installed() {
  if [ ! -f "$COMPOSER_VENDOR_DIR/friendsofphp/php-cs-fixer/php-cs-fixer" ]; then
    echo "the PHP plugin's formatter is not installed in .cache/php-tools: run \`mise run php:tools\`" >&2
    exit 127
  fi
}
