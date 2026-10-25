# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing_extensions import TypedDict

__all__ = ["ItemListParams"]


class ItemListParams(TypedDict, total=False):
    ending_before: str
    """A cursor: return items before this token."""

    page_size: int
    """Page size (for pagination)."""

    starting_after: str
    """A cursor: return items after this token."""
