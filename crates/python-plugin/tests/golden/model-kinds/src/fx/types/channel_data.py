# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import Dict, List, Optional
from typing_extensions import TypeAlias

from .._models import BaseModel

__all__ = ["ChannelData", "ChannelDataItem"]


class ChannelDataItem(BaseModel):
    tokens: Optional[List[str]] = None


ChannelData: TypeAlias = Dict[str, ChannelDataItem]
