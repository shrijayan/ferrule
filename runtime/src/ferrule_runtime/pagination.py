# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Generate an SDK's ``pagination.py`` from its pagination schemes.

Class shapes and method bodies reproduce the text of Stainless-generated SDKs (Apache-2.0):
``cursor`` (knock-python, knock-mgmt-python, ogx-client-python), ``cursor_id`` and
``fake_page`` (lithic-python), ``page_number`` (honcho-python-core). ``offset``,
``cursor_url`` and ``page_number`` without a total-pages field have no pinned oracle yet;
their text is unverified (see runtime/README.md).
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import TYPE_CHECKING, Literal

if TYPE_CHECKING:
    from collections.abc import Sequence

__all__ = [
    "PageField",
    "PageKind",
    "PageModel",
    "PageScheme",
    "PaginationError",
    "render_pagination",
]

PageKind = Literal["cursor", "cursor_id", "cursor_url", "offset", "page_number", "fake_page"]

HEADER = "# File generated from our OpenAPI spec by Stainless. See CONTRIBUTING.md for details.\n"
_TYPING = ("Any", "Dict", "List", "Union", "Generic", "TypeVar", "Optional", "cast")
_TYPING_EXT = ("Protocol", "override", "runtime_checkable")
_BASE_CLIENT = ("BasePage", "PageInfo", "BaseSyncPage", "BaseAsyncPage")


class PaginationError(ValueError):
    """Raised for an incomplete or inconsistent pagination scheme."""


@dataclass(frozen=True)
class PageField:
    """A response field declared on the page class, in response-schema order."""

    name: str
    annotation: str
    """Python annotation as Stainless prints it, e.g. ``List[_T]``, ``bool``, ``Optional[int]``."""
    optional: bool = False
    """Renders ``= None`` (Stainless: a field that is not required in the response)."""

    def line(self) -> str:
        """Return the class-body line declaring this field."""
        return f"    {self.name}: {self.annotation}" + (" = None" if self.optional else "")


@dataclass(frozen=True)
class PageModel:
    """A nested response model (e.g. ``EntriesCursorPageInfo``), emitted before its scheme."""

    name: str
    fields: tuple[PageField, ...]


@dataclass(frozen=True, kw_only=True)
class PageScheme:
    """One entry of ``pagination:`` in the Stainless config, resolved against the spec.

    ``name`` is the class stem: ``CursorPage`` gives ``SyncCursorPage`` / ``AsyncCursorPage``.
    """

    name: str
    kind: PageKind
    items: str
    fields: tuple[PageField, ...]
    models: tuple[PageModel, ...] = ()
    has_more: str | None = None
    """Response field that gates ``has_next_page`` (``has_more``)."""
    param: str | None = None
    """Request parameter of the next page: cursor, ``starting_after``, offset or page."""
    before_param: str | None = None
    """``cursor_id`` only: the backwards parameter (``ending_before``)."""
    item_id: str | None = None
    """``cursor_id`` only: the item field holding the cursor (``id``, ``token``)."""
    path: tuple[str, ...] = ()
    """Response path of the next cursor / next URL / current page number."""
    total_path: tuple[str, ...] = ()
    """Response path of the total page count (``page_number``) or item count (``offset``)."""


_REQUIRED: dict[str, tuple[str, ...]] = {
    "cursor": ("param", "path"),
    "cursor_id": ("param", "before_param", "item_id"),
    "cursor_url": ("path",),
    "offset": ("param",),
    "page_number": ("param", "path"),
    "fake_page": (),
}


def _validate(scheme: PageScheme) -> None:
    missing = [attr for attr in _REQUIRED[scheme.kind] if not getattr(scheme, attr)]
    if missing:
        msg = f"pagination scheme {scheme.name!r} ({scheme.kind}) needs {', '.join(missing)}"
        raise PaginationError(msg)
    names = {f.name for f in scheme.fields}
    if scheme.items not in names:
        msg = f"pagination scheme {scheme.name!r}: items field {scheme.items!r} is not declared"
        raise PaginationError(msg)
    if scheme.has_more is not None and scheme.has_more not in names:
        msg = (
            f"pagination scheme {scheme.name!r}: has_more field {scheme.has_more!r} is not declared"
        )
        raise PaginationError(msg)


def _field(scheme: PageScheme, name: str) -> PageField:
    return next(f for f in scheme.fields if f.name == name)


def _get_items(items: str) -> list[str]:
    return [
        "    @override",
        "    def _get_page_items(self) -> List[_T]:",
        f"        {items} = self.{items}",
        f"        if not {items}:",
        "            return []",
        f"        return {items}",
    ]


def _has_next(scheme: PageScheme) -> list[str]:
    if scheme.has_more is None:
        return []
    name = scheme.has_more
    head = [
        "",
        "    @override",
        "    def has_next_page(self) -> bool:",
        f"        {name} = self.{name}",
    ]
    if _field(scheme, name).optional:
        return [
            *head,
            f"        if {name} is not None and {name} is False:",
            "            return False",
            "",
            "        return super().has_next_page()",
        ]
    return [*head, f"        return {name} and super().has_next_page()"]


def _read_path(path: tuple[str, ...], var: str) -> list[str]:
    """Read a (possibly nested) optional response path into ``var``."""
    if len(path) == 1:
        return [f"        {var} = self.{path[0]}"]
    lines = [f"        {var} = None"]
    indent = "        "
    for depth in range(1, len(path) + 1):
        lines.append(f"{indent}if self.{'.'.join(path[:depth])} is not None:")
        indent += "    "
    lines.append(f"{indent}{var} = self.{'.'.join(path)}")
    return lines


def _next_cursor(scheme: PageScheme) -> list[str]:
    var = scheme.path[-1]
    return [
        *_read_path(scheme.path, var),
        f"        if not {var}:",
        "            return None",
        "",
        f'        return PageInfo(params={{"{scheme.param}": {var}}})',
    ]


def _next_cursor_id(scheme: PageScheme) -> list[str]:
    item_cls = f"{scheme.name}Item"
    items = scheme.items

    def branch(index: str, param: str | None, indent: str) -> list[str]:
        return [
            f"{indent}item = cast(Any, {index})",
            f"{indent}if not isinstance(item, {item_cls}) or item.{scheme.item_id} is None:",
            f"{indent}    # TODO emit warning log",
            f"{indent}    return None",
            "",
            f'{indent}return PageInfo(params={{"{param}": item.{scheme.item_id}}})',
        ]

    return [
        f'        is_forwards = not self._options.params.get("{scheme.before_param}", False)',
        "",
        f"        {items} = self.{items}",
        f"        if not {items}:",
        "            return None",
        "",
        "        if is_forwards:",
        *branch(f"{items}[-1]", scheme.param, "            "),
        "        else:",
        *branch(f"self.{items}[0]", scheme.before_param, "            "),
    ]


def _next_page_number(scheme: PageScheme) -> list[str]:
    lines = [
        *_read_path(scheme.path, "current_page"),
        "        if current_page is None:",
        "            current_page = 1",
        "",
    ]
    if scheme.total_path:
        lines += [
            *_read_path(scheme.total_path, "total_pages"),
            "        if total_pages is not None and current_page >= total_pages:",
            "            return None",
            "",
        ]
    return [*lines, f'        return PageInfo(params={{"{scheme.param}": current_page + 1}})']


def _next_offset(scheme: PageScheme) -> list[str]:
    param = scheme.param
    lines = [
        f'        offset = self._options.params.get("{param}") or 0',
        "        if not isinstance(offset, int):",
        (
            "            raise ValueError("
            f"f'Expected \"{param}\" param to be an integer but got {{offset}}')"
        ),
        "",
        "        length = len(self._get_page_items())",
        "        current_count = offset + length",
        "",
    ]
    if not scheme.total_path:
        return [*lines, f'        return PageInfo(params={{"{param}": current_count}})']
    return [
        *lines,
        *_read_path(scheme.total_path, "total_count"),
        "        if total_count is None:",
        "            return None",
        "",
        "        if current_count < total_count:",
        f'            return PageInfo(params={{"{param}": current_count}})',
        "",
        "        return None",
    ]


def _next_cursor_url(scheme: PageScheme) -> list[str]:
    return [
        *_read_path(scheme.path, "url"),
        "        if url is None:",
        "            return None",
        "",
        "        return PageInfo(url=httpx.URL(url))",
    ]


def _next_page(scheme: PageScheme) -> list[str]:
    if scheme.kind == "fake_page":
        return [
            "",
            "    @override",
            "    def next_page_info(self) -> None:",
            '        """',
            (
                "        This page represents a response that isn't actually paginated"
                " at the API level"
            ),
            "        so there will never be a next page.",
            '        """',
            "        return None",
        ]
    body = {
        "cursor": _next_cursor,
        "cursor_id": _next_cursor_id,
        "cursor_url": _next_cursor_url,
        "offset": _next_offset,
        "page_number": _next_page_number,
    }[scheme.kind](scheme)
    return ["", "    @override", "    def next_page_info(self) -> Optional[PageInfo]:", *body]


def _page_class(scheme: PageScheme, mode: Literal["Sync", "Async"]) -> list[str]:
    return [
        f"class {mode}{scheme.name}(Base{mode}Page[_T], BasePage[_T], Generic[_T]):",
        *(f.line() for f in scheme.fields),
        "",
        *_get_items(scheme.items),
        *_has_next(scheme),
        *_next_page(scheme),
    ]


def _blocks(scheme: PageScheme) -> list[list[str]]:
    blocks: list[list[str]] = []
    if scheme.kind == "cursor_id":
        blocks.append(
            [
                "@runtime_checkable",
                f"class {scheme.name}Item(Protocol):",
                f"    {scheme.item_id}: Optional[str]",
            ]
        )
    blocks.extend(
        [f"class {m.name}(BaseModel):", *(f.line() for f in m.fields)] for m in scheme.models
    )
    blocks.append(_page_class(scheme, "Sync"))
    blocks.append(_page_class(scheme, "Async"))
    return blocks


def _sort_names(names: list[str]) -> list[str]:
    """Order imported names like Stainless's isort (ruff length-sort, classes before functions)."""
    return sorted(names, key=lambda n: (not n[0].isupper(), len(n), n))


def _wrap(
    head: str, names: list[str], brackets: tuple[str, str], line_length: int, *, bare: bool
) -> str:
    """One line if it fits, else one name per line with a magic trailing comma (ruff format)."""
    opening, closing = brackets
    one = f"{head}{', '.join(names)}" if bare else f"{head}{opening}{', '.join(names)}{closing}"
    if len(one) <= line_length:
        return one
    inner = "".join(f"    {n},\n" for n in names)
    return f"{head}{opening}\n{inner}{closing}"


def _used(body: str, candidates: tuple[str, ...]) -> list[str]:
    return [n for n in candidates if re.search(rf"\b{n}\b", body)]


def render_pagination(schemes: Sequence[PageScheme], *, line_length: int = 120) -> str:
    """Return the text of ``pagination.py`` for the given schemes, in config order."""
    if not schemes:
        msg = "at least one pagination scheme is required"
        raise PaginationError(msg)
    for scheme in schemes:
        _validate(scheme)

    exported: list[str] = []
    blocks: list[list[str]] = []
    for scheme in schemes:
        exported += [m.name for m in scheme.models]
        exported += [f"Sync{scheme.name}", f"Async{scheme.name}"]
        blocks += _blocks(scheme)
    body = "\n\n\n".join("\n".join(b) for b in blocks) + "\n"

    paren = ("(", ")")
    typing_names = _sort_names([*_used(body, _TYPING), "TypeVar"])
    ext_names = _sort_names(_used(body, _TYPING_EXT))
    imports = [
        _wrap("from typing import ", typing_names, paren, line_length, bare=True),
        _wrap("from typing_extensions import ", ext_names, paren, line_length, bare=True),
    ]
    third_party = "\nimport httpx\n" if re.search(r"\bhttpx\.", body) else ""
    local: list[str] = []
    if re.search(r"\bBaseModel\b", body):
        local.append("from ._models import BaseModel")
    local.append(
        _wrap(
            "from ._base_client import ", _used(body, _BASE_CLIENT), paren, line_length, bare=True
        )
    )
    all_line = _wrap(
        "__all__ = ", [f'"{n}"' for n in exported], ("[", "]"), line_length, bare=False
    )

    return (
        f"{HEADER}\n"
        + "\n".join(imports)
        + "\n"
        + third_party
        + "\n"
        + "\n".join(local)
        + f"\n\n{all_line}\n\n"
        + '_T = TypeVar("_T")\n\n\n'
        + body
    )
