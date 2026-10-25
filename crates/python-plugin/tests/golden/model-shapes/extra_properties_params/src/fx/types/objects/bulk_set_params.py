# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import Dict, Union, Iterable
from typing_extensions import Required, TypeAlias, TypedDict

__all__ = ["BulkSetParams", "Object"]


class BulkSetParams(TypedDict, total=False):
    objects: Required[Iterable[Object]]
    """A list of objects."""


class ObjectTyped(TypedDict, total=False):
    id: Required[str]
    """Unique identifier for the object."""


Object: TypeAlias = Union[ObjectTyped, Dict[str, object]]
