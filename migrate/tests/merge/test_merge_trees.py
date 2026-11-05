# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
from hypothesis import given
from hypothesis import strategies as st

from ferrule.merge import (
    BINARY_CONFLICT,
    DELETED_BY_GENERATOR,
    DELETED_BY_USER,
    TEXT_CONFLICT,
    KeepRules,
    Outcome,
    merge_trees,
)
from ferrule.merge.rules import glob_to_regex

KEEP = KeepRules()
CLIENT = "src/acme/_client.py"
RES = "src/acme/resources/models.py"

paths = st.sampled_from(["a.py", "b.md", "src/acme/c.py", "src/acme/lib/d.py", "e.txt"])
blobs = st.binary(max_size=20)
trees = st.dictionaries(paths, blobs, max_size=5)


@given(trees)
def test_noop_regeneration_is_identity(tree: dict[str, bytes]) -> None:
    result = merge_trees(tree, tree, tree, keep=KEEP)
    assert result.files == tree
    assert not result.conflicts


@given(trees, trees)
def test_theirs_only_changes_always_apply(base: dict[str, bytes], theirs: dict[str, bytes]) -> None:
    result = merge_trees(base, base, theirs, keep=KeepRules(protect_lib=False))
    assert result.files == theirs


@given(trees, trees, st.dictionaries(st.sampled_from(["u1.py", "u2.txt"]), blobs))
def test_user_edits_in_untouched_files_survive(
    base: dict[str, bytes], theirs: dict[str, bytes], user: dict[str, bytes]
) -> None:
    ours = {**base, **user}
    result = merge_trees(base, ours, theirs, keep=KEEP)
    for path, data in user.items():
        assert result.files[path] == data


def test_lib_and_examples_are_never_touched() -> None:
    base = {"src/acme/lib/helpers.py": b"x\n", "examples/demo.py": b"y\n"}
    ours = {"src/acme/lib/helpers.py": b"custom\n"}
    theirs = {
        "src/acme/lib/helpers.py": b"gen\n",
        "examples/demo.py": b"gen\n",
        "src/acme/lib/n.py": b"n",
    }
    result = merge_trees(base, ours, theirs, keep=KEEP)
    assert result.files == ours
    assert result.outcomes["src/acme/lib/helpers.py"] == Outcome.KEPT
    assert not result.conflicts


def test_the_agent_recipes_ferrule_generates_under_examples_are_updated() -> None:
    recipe = "examples/agent/session_get.py"
    base = {recipe: b"client.session.get(session_id={})\n", "examples/demo.py": b"y\n"}
    ours = dict(base)
    theirs = {recipe: b'client.session.get(session_id="sessionID")\n', "examples/demo.py": b"gen\n"}
    result = merge_trees(base, ours, theirs, keep=KEEP)
    assert result.files[recipe] == theirs[recipe]
    assert result.files["examples/demo.py"] == b"y\n"


def test_keep_files_glob() -> None:
    keep = KeepRules.from_config(
        {
            "targets": {
                "python": {"keep_files": [".github/workflows/*.yml", "./scripts/**", "docs/"]}
            }
        }
    )
    base = {".github/workflows/ci.yml": b"gen\n", "scripts/a/b.sh": b"gen\n", "docs/x.md": b"g\n"}
    ours = {".github/workflows/ci.yml": b"mine\n", "scripts/a/b.sh": b"mine\n", "docs/x.md": b"m\n"}
    theirs = dict.fromkeys(base, b"new\n")
    assert merge_trees(base, ours, theirs, keep=keep).files == ours
    assert not keep.keeps(".github/workflows/sub/ci.yml")
    assert glob_to_regex("a?c**").fullmatch("abcdef/g")
    assert KeepRules.from_config({}).keep_files == ()
    assert KeepRules.from_config({"targets": {"python": {"keep_files": "x"}}}).keep_files == ()


def test_method_added_by_hand_while_generator_adds_another() -> None:
    base = b"class Models:\n    def a(self) -> None:\n        pass\n"
    ours = base + b"\n    def mine(self) -> int:\n        return 1\n"
    theirs = base + b"\n    def gen(self) -> int:\n        return 2\n"
    result = merge_trees({RES: base}, {RES: ours}, {RES: theirs}, keep=KEEP)
    assert result.outcomes[RES] == Outcome.AST
    assert b"def mine" in result.files[RES]
    assert b"def gen" in result.files[RES]
    assert not result.conflicts


def test_docstring_edited_vs_regenerated_conflicts() -> None:
    base = b'def get() -> None:\n    """Get it."""\n'
    ours = b'def get() -> None:\n    """Get it, cached."""\n'
    theirs = b'def get() -> None:\n    """Retrieve it."""\n'
    result = merge_trees({CLIENT: base}, {CLIENT: ours}, {CLIENT: theirs}, keep=KEEP)
    assert result.outcomes[CLIENT] == Outcome.CONFLICT
    (conflict,) = result.blocking
    assert conflict.code == TEXT_CONFLICT
    assert conflict.regions
    assert conflict.render().startswith(f"{TEXT_CONFLICT} at {CLIENT}:")
    assert b"<<<<<<< ours" in result.files[CLIENT]
    assert result.to_json()["clean"] is False


def test_file_deleted_by_generator_but_edited_is_kept_and_reported() -> None:
    result = merge_trees({"old.py": b"a\n"}, {"old.py": b"b\n"}, {}, keep=KEEP)
    assert result.files == {"old.py": b"b\n"}
    (conflict,) = result.conflicts
    assert conflict.code == DELETED_BY_GENERATOR
    assert not conflict.blocking
    assert not result.blocking


def test_file_deleted_by_user_but_regenerated_is_reported() -> None:
    result = merge_trees({"x.py": b"a\n"}, {}, {"x.py": b"b\n"}, keep=KEEP)
    assert result.files == {}
    assert result.blocking[0].code == DELETED_BY_USER


def test_binary_and_plain_text_cases() -> None:
    bin_result = merge_trees({"i.png": b"\0a"}, {"i.png": b"\0b"}, {"i.png": b"\0c"}, keep=KEEP)
    assert bin_result.conflicts[0].code == BINARY_CONFLICT
    assert merge_trees({"u": b"\xff"}, {"u": b"\xfe"}, {"u": b"\xfd"}, keep=KEEP).conflicts
    text = merge_trees(
        {"r.md": b"a\nb\nc\n"}, {"r.md": b"A\nb\nc\n"}, {"r.md": b"a\nb\nC\n"}, keep=KEEP
    )
    assert text.files["r.md"] == b"A\nb\nC\n"
    report = text.to_json()
    assert report["outcomes"] == {"r.md": "merged"}
    assert report["counts"] == {"merged": 1}
    added = merge_trees({}, {"n.md": b"x\n"}, {"n.md": b"y\n"}, keep=KEEP)
    assert added.outcomes["n.md"] == Outcome.CONFLICT
