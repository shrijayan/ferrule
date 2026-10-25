# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import Union
from typing_extensions import Annotated, TypeAlias

from .._utils import PropertyInfo
from .static_audience import StaticAudience
from .dynamic_audience import DynamicAudience

__all__ = ["Audience"]

Audience: TypeAlias = Annotated[Union[StaticAudience, DynamicAudience], PropertyInfo(discriminator="type")]
