# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import List, Optional

from .._models import BaseModel
from .audience import Audience
from .channel_data import ChannelData
from .condition_group import ConditionGroup
from .shared.extensions import Extensions
from .shared.streaming_resolution import StreamingResolution

__all__ = ["Workflow"]


class Workflow(BaseModel):
    """A workflow."""

    key: str

    steps: List["WorkflowStep"]

    audience: Optional[Audience] = None
    """An audience."""

    channel_data: Optional[ChannelData] = None
    """Channel data keyed by channel id."""

    conditions: Optional[ConditionGroup] = None
    """A group of conditions to be evaluated."""

    extensions: Optional[Extensions] = None
    """Extensions to apply."""

    resolution: Optional[StreamingResolution] = None
    """A streaming resolution."""


from .workflow_step import WorkflowStep
