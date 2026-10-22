# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import Optional
from datetime import datetime
from typing_extensions import Literal

from .._models import BaseModel

__all__ = ["Item"]


class Item(BaseModel):
    created: datetime

    token: str
    """Globally unique identifier."""

    nickname: Optional[str] = None

    status: Optional[Literal["ACTIVE", "CLOSED"]] = None
