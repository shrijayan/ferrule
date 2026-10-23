# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import libcst as cst

from ferrule.merge.ast_merge import merge_python, user_changes_survive

# A generated resource class in the style of the oracle SDKs (illustrative, not copied).
BASE = '''\
"""Models resource."""

from __future__ import annotations

import httpx

__all__ = ["Models"]


class Models:
    def retrieve(self, model: str) -> str:
        """Get a model."""
        return self._get(f"/models/{model}")

    def list(self) -> list[str]:
        """List models."""
        return self._get("/models")
'''

USER_METHOD = '''
    def retrieve_or_none(self, model: str) -> str | None:
        """Hand-written helper."""
        try:
            return self.retrieve(model)
        except KeyError:
            return None
'''

GEN_METHOD = '''
    def delete(self, model: str) -> None:
        """Delete a model."""
        self._delete(f"/models/{model}")
'''


def test_methods_added_on_both_sides_merge() -> None:
    ours = BASE.replace("import httpx\n", "import httpx\nimport functools\n") + USER_METHOD
    theirs = BASE.replace('"/models")\n', '"/v2/models")\n') + GEN_METHOD
    merged = merge_python(BASE, ours, theirs)
    assert merged is not None
    assert "def retrieve_or_none" in merged
    assert "def delete" in merged
    assert '"/v2/models"' in merged
    assert "import functools" in merged
    cst.parse_module(merged)
    assert merged.index("def delete") < merged.index("def retrieve_or_none")


def test_docstring_edited_on_both_sides_is_not_merged() -> None:
    ours = BASE.replace('"""Get a model."""', '"""Get a model (cached)."""')
    theirs = BASE.replace('"""Get a model."""', '"""Retrieve a model by id."""')
    assert merge_python(BASE, ours, theirs) is None


def test_module_docstring_edited_on_both_sides_is_not_merged() -> None:
    ours = BASE.replace("Models resource.", "Ours.")
    theirs = BASE.replace("Models resource.", "Theirs.")
    assert merge_python(BASE, ours, theirs) is None


def test_class_header_edits() -> None:
    ours = BASE.replace("class Models:", "class Models(Mixin):") + USER_METHOD
    theirs = BASE + GEN_METHOD
    merged = merge_python(BASE, ours, theirs)
    assert merged is not None
    assert "class Models(Mixin):" in merged
    both = BASE.replace("class Models:", "class Models(Base):") + GEN_METHOD
    assert merge_python(BASE, ours, both) is None


def test_class_added_on_both_sides_merges_members() -> None:
    extra = "\n\nclass Extra:\n    def a(self) -> None:\n        pass\n"
    other = "\n\nclass Extra:\n    def b(self) -> None:\n        pass\n"
    merged = merge_python(BASE, BASE + extra, BASE + other)
    assert merged is not None
    assert "def a" in merged
    assert "def b" in merged


def test_deleted_on_one_side_changed_on_other_fails() -> None:
    without_list = BASE[: BASE.index("    def list")].rstrip() + "\n"
    changed_list = BASE.replace('"/models")', '"/models", page=1)')
    assert merge_python(BASE, without_list, changed_list) is None
    assert merge_python(BASE, changed_list, without_list) is None


def test_deletions_apply_when_other_side_untouched() -> None:
    without_list = BASE[: BASE.index("    def list")].rstrip() + "\n"
    ours = BASE + USER_METHOD
    merged = merge_python(BASE, ours.replace("import httpx\n", ""), without_list + GEN_METHOD)
    assert merged is not None
    assert "def list" not in merged
    assert "import httpx" not in merged


def test_unparseable_and_unkeyable_inputs_fail() -> None:
    assert merge_python(BASE, "def (:\n", BASE + GEN_METHOD) is None
    dup = BASE + "\n\ndef f() -> None: ...\n\n\ndef f() -> None: ...\n"
    assert merge_python(BASE, dup, BASE + "\nX = 1\n") is None
    one_line = "class A: pass\n"
    assert merge_python(one_line, "class A: x = 1\n", "class A: y = 2\n") is None


def test_generic_statements_changed_on_both_sides_fail() -> None:
    ours = BASE + "\nif True:\n    A = 1\n"
    theirs = BASE + "\nif True:\n    B = 1\n"
    assert merge_python(BASE, ours, theirs) is None


def test_assignments_are_keyed_by_name() -> None:
    base = "X = 1\nY: int = 2\n"
    merged = merge_python(base, "X = 10\nY: int = 2\nZ = 0\n", "X = 1\nY: int = 3\nW = 0\n")
    assert merged == "X = 10\nY: int = 3\nW = 0\nZ = 0\n"


def test_survival_check_rejects_lost_user_code() -> None:
    ours = BASE + USER_METHOD
    assert user_changes_survive(BASE, ours, ours)
    assert not user_changes_survive(BASE, ours, BASE)
    assert not user_changes_survive(BASE, ours, "def (:\n")
