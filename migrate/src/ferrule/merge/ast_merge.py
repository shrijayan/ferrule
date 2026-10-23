# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Structural three-way merge of Python modules with libcst, used when a text merge conflicts.

Statements are keyed: functions, classes and simple assignments by name, imports and other
statements by their source. Each key is merged three-way; classes changed on both sides are
merged member by member. The result is accepted only if it parses and every definition the
user added or changed is present verbatim.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

import libcst as cst

if TYPE_CHECKING:
    from collections.abc import Sequence

type Key = tuple[str, str]
type Stmt = cst.BaseStatement
_RENDER = cst.Module(body=[])


class _MergeConflictError(Exception):
    """Both sides changed the same definition in incompatible ways."""


@dataclass(frozen=True)
class _Item:
    key: Key
    node: Stmt

    @property
    def code(self) -> str:
        return _RENDER.code_for_node(self.node)


def _key(stmt: Stmt, index: int) -> Key:
    if isinstance(stmt, cst.FunctionDef):
        return ("def", stmt.name.value)
    if isinstance(stmt, cst.ClassDef):
        return ("class", stmt.name.value)
    if isinstance(stmt, cst.SimpleStatementLine) and len(stmt.body) == 1:
        small = stmt.body[0]
        if index == 0 and isinstance(small, cst.Expr) and isinstance(small.value, cst.SimpleString):
            return ("doc", "")
        if isinstance(small, cst.Import | cst.ImportFrom):
            return ("import", _RENDER.code_for_node(small))
        target = _assign_target(small)
        if target is not None:
            return ("assign", target)
    return ("stmt", _RENDER.code_for_node(stmt).strip())


def _assign_target(small: cst.BaseSmallStatement) -> str | None:
    if isinstance(small, cst.AnnAssign) and isinstance(small.target, cst.Name):
        return small.target.value
    if (
        isinstance(small, cst.Assign)
        and len(small.targets) == 1
        and isinstance(small.targets[0].target, cst.Name)
    ):
        return small.targets[0].target.value
    return None


def _items(body: Sequence[Stmt]) -> dict[Key, _Item]:
    out: dict[Key, _Item] = {}
    for index, stmt in enumerate(body):
        key = _key(stmt, index)
        if key in out:  # duplicate names (overloads, redefinitions): too subtle to key
            raise _MergeConflictError
        out[key] = _Item(key, stmt)
    return out


def _class_body(node: cst.ClassDef) -> Sequence[Stmt]:
    body = node.body
    if isinstance(body, cst.IndentedBlock):
        return body.body
    raise _MergeConflictError  # one-line class: nothing worth merging structurally


def _class_header(node: cst.ClassDef) -> str:
    stub = cst.IndentedBlock(body=[cst.SimpleStatementLine(body=[cst.Pass()])])
    return _RENDER.code_for_node(node.with_changes(body=stub))


def _merge_class(base: cst.ClassDef | None, ours: cst.ClassDef, theirs: cst.ClassDef) -> Stmt:
    base_body: Sequence[Stmt] = _class_body(base) if base is not None else ()
    body = _merge_body(base_body, _class_body(ours), _class_body(theirs))
    base_head = _class_header(base) if base is not None else None
    ours_head, theirs_head = _class_header(ours), _class_header(theirs)
    if ours_head in (base_head, theirs_head):
        shell = theirs
    elif theirs_head == base_head:
        shell = ours
    else:
        raise _MergeConflictError
    block = cst.ensure_type(shell.body, cst.IndentedBlock)
    return shell.with_changes(body=block.with_changes(body=body))


def _merge_item(base: _Item | None, ours: _Item, theirs: _Item) -> Stmt:
    base_code = base.code if base is not None else None
    if ours.code in (base_code, theirs.code):
        return theirs.node
    if theirs.code == base_code:
        return ours.node
    if isinstance(ours.node, cst.ClassDef) and isinstance(theirs.node, cst.ClassDef):
        base_node = base.node if base is not None else None
        return _merge_class(
            base_node if isinstance(base_node, cst.ClassDef) else None, ours.node, theirs.node
        )
    raise _MergeConflictError


def _check_generic_statements(base: dict[Key, _Item], *sides: dict[Key, _Item]) -> None:
    """Unkeyed statements are matched by source, so edits on both sides would duplicate."""
    base_stmts = {k for k in base if k[0] == "stmt"}
    changed = [{k for k in side if k[0] == "stmt"} != base_stmts for side in sides]
    if all(changed):
        raise _MergeConflictError


def _merge_body(base: Sequence[Stmt], ours: Sequence[Stmt], theirs: Sequence[Stmt]) -> list[Stmt]:
    b, o, t = _items(base), _items(ours), _items(theirs)
    _check_generic_statements(b, o, t)
    merged: dict[Key, Stmt] = {}
    for key in [*t, *(k for k in o if k not in t), *(k for k in b if k not in t and k not in o)]:
        bi, oi, ti = b.get(key), o.get(key), t.get(key)
        if oi is not None and ti is not None:
            merged[key] = _merge_item(bi, oi, ti)
        elif oi is not None:  # absent in theirs: added by the user, or deleted by the generator
            if bi is not None and oi.code != bi.code:
                raise _MergeConflictError
            if bi is None:
                merged[key] = oi.node
        elif ti is not None:  # absent in ours: added by the generator, or deleted by the user
            if bi is not None and ti.code != bi.code:
                raise _MergeConflictError
            if bi is None:
                merged[key] = ti.node
    return _order(merged, list(o), list(t))


def _order(merged: dict[Key, Stmt], ours: list[Key], theirs: list[Key]) -> list[Stmt]:
    """Theirs order; each user-only statement follows its predecessor in ours.

    Statements the generator just added stay together, ahead of the user's additions.
    """
    order = [k for k in theirs if k in merged]
    in_ours = set(ours)
    previous: Key | None = None
    for key in ours:
        if key in merged and key not in order:
            at = order.index(previous) + 1 if previous is not None else 0
            while previous is not None and at < len(order) and order[at] not in in_ours:
                at += 1
            order.insert(at, key)
        if key in order:
            previous = key
    return [merged[k] for k in order]


def _leaves(body: Sequence[Stmt], prefix: tuple[Key, ...] = ()) -> dict[tuple[Key, ...], str]:
    """Every definition, flattened through classes, with its source."""
    out: dict[tuple[Key, ...], str] = {}
    for key, item in _items(body).items():
        path = (*prefix, key)
        if isinstance(item.node, cst.ClassDef) and isinstance(item.node.body, cst.IndentedBlock):
            out[(*path, ("header", ""))] = _class_header(item.node)
            out.update(_leaves(item.node.body.body, path))
        else:
            out[path] = item.code
    return out


def user_changes_survive(base: str, ours: str, merged: str) -> bool:
    """Whether every definition ``ours`` added or changed relative to ``base`` is in ``merged``."""
    try:
        b, o, m = (_leaves(cst.parse_module(src).body) for src in (base, ours, merged))
    except (cst.ParserSyntaxError, _MergeConflictError):
        return False
    return all(m.get(path) == code for path, code in o.items() if b.get(path) != code)


def merge_python(base: str, ours: str, theirs: str) -> str | None:
    """Merge three versions of a module structurally; ``None`` when that is not safe."""
    try:
        b, o, t = (cst.parse_module(src) for src in (base, ours, theirs))
        body = _merge_body(b.body, o.body, t.body)
    except (cst.ParserSyntaxError, _MergeConflictError):
        return None
    code = t.with_changes(body=body).code
    if not user_changes_survive(base, ours, code):  # pragma: no cover - guard, by construction
        return None
    return code
