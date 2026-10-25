# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import List, Union
from typing_extensions import TypeAlias

from .static_audience import StaticAudience
from .dynamic_audience import DynamicAudience

__all__ = ["WorkflowListAudiencesResponse", "WorkflowListAudiencesResponseItem"]

WorkflowListAudiencesResponseItem: TypeAlias = Union[StaticAudience, DynamicAudience]

WorkflowListAudiencesResponse: TypeAlias = List[WorkflowListAudiencesResponseItem]
