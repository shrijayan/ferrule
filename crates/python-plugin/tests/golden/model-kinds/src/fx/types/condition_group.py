# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import List, Union, Optional
from typing_extensions import TypeAlias

from .._models import BaseModel

__all__ = ["ConditionGroup", "ConditionGroupAllMatch", "ConditionGroupAnyMatch"]


class ConditionGroupAllMatch(BaseModel):
    """A group of conditions that must all be met."""

    all: Optional[List[str]] = None


class ConditionGroupAnyMatch(BaseModel):
    """A group of conditions that any must be met."""

    any: Optional[List[str]] = None


ConditionGroup: TypeAlias = Union[ConditionGroupAllMatch, ConditionGroupAnyMatch]
