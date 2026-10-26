#!/usr/bin/env bash
# `mise run php:tools`: installs the PHP env.sh pins (mise builds it from source, once), then the
# packages composer.lock pins into <repo>/.cache/php-tools (the network once), then checks that
# bin/php-cs-fixer runs.
set -euo pipefail
# shellcheck source=env.sh
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"
if command -v mise >/dev/null 2>&1 && ! mise where "php@$php_version" >/dev/null 2>&1; then
  mise install "php@$php_version"
  PATH="$(mise where "php@$php_version")/bin:$PATH"
fi
composer install --working-dir="$tools" --no-progress --quiet
"$tools/bin/php-cs-fixer" --version
