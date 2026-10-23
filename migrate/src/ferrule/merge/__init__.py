# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Custom-code preservation: three-way merge of regenerated SDK trees (plan §7 stage 04).

The pure core is :func:`merge_trees`; :mod:`ferrule.merge.gitio` holds the git and file I/O,
including :func:`~ferrule.merge.gitio.regenerate_branch` for adopt and CI.
"""

from __future__ import annotations

from ferrule.merge.core import (
    BINARY_CONFLICT,
    DELETED_BY_GENERATOR,
    DELETED_BY_USER,
    TEXT_CONFLICT,
    Conflict,
    MergeResult,
    Outcome,
    Tree,
    merge_trees,
)
from ferrule.merge.rules import KeepRules

__all__ = [
    "BINARY_CONFLICT",
    "DELETED_BY_GENERATOR",
    "DELETED_BY_USER",
    "TEXT_CONFLICT",
    "Conflict",
    "KeepRules",
    "MergeResult",
    "Outcome",
    "Tree",
    "merge_trees",
]
