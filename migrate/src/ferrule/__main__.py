# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Allow ``python -m ferrule``."""

import sys

from ferrule.cli import main

sys.exit(main())
