#!/usr/bin/env bash
# `mise run php:tools`: installs the packages composer.lock pins into <repo>/.cache/php-tools (the
# network once), then checks that bin/php-cs-fixer runs.
set -euo pipefail
# shellcheck source=env.sh
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"
composer install --working-dir="$tools" --no-progress --quiet
"$tools/bin/php-cs-fixer" --version
