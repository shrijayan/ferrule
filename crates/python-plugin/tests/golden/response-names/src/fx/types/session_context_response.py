# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import Optional

from .._models import BaseModel

__all__ = ["SessionContextResponse", "SessionContextResponseSummary"]


class SessionContextResponseSummary(BaseModel):
    content: str
    """The summary text"""


class SessionContextResponse(BaseModel):
    id: str

    summary: Optional[SessionContextResponseSummary] = None
    """The summary if available"""
