# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import Optional

from .._models import BaseModel

__all__ = ["SessionSummariesResponse", "LongSummary"]


class LongSummary(BaseModel):
    content: str
    """The summary text"""


class SessionSummariesResponse(BaseModel):
    id: str

    long_summary: Optional[LongSummary] = None
    """The long summary if available"""
