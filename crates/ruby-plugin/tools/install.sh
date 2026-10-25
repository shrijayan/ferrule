#!/usr/bin/env bash
# `mise run ruby:tools`: installs the gems Gemfile.lock pins into <repo>/.cache/ruby-tools (the
# network once), then checks that bin/stree and bin/rubocop run.
set -euo pipefail
# shellcheck source=bundle.sh
source "$(dirname "${BASH_SOURCE[0]}")/bundle.sh"
bundle install --quiet
"$tools/bin/stree" version
"$tools/bin/rubocop" --version
