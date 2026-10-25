# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import Union
from datetime import datetime
from typing_extensions import Required, Annotated, TypeAlias, TypedDict

from .._utils import PropertyInfo

__all__ = ["GuideActivateParams", "GuideBooleanActivationParams", "GuideScheduledActivationParams"]


class GuideBooleanActivationParams(TypedDict, total=False):
    environment: Required[str]
    """The environment slug."""

    status: Required[bool]
    """Whether to activate or deactivate the guide."""

    branch: str
    """The slug of a branch to use."""


class GuideScheduledActivationParams(TypedDict, total=False):
    environment: Required[str]
    """The environment slug."""

    branch: str
    """The slug of a branch to use."""

    from_: Annotated[Union[str, datetime], PropertyInfo(alias="from", format="iso8601")]
    """When to activate the guide."""

    until: Annotated[Union[str, datetime], PropertyInfo(format="iso8601")]
    """When to deactivate the guide."""


GuideActivateParams: TypeAlias = Union[GuideBooleanActivationParams, GuideScheduledActivationParams]
