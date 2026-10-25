# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing_extensions import Literal

from .._models import BaseModel

__all__ = ["StreamEvent"]


class StreamEvent(BaseModel):
    id: str
    """Unique identifier for this event"""

    type: Literal["system", "log"]
    """Type of stream event"""
