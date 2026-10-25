# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import Dict, Optional
from typing_extensions import TypedDict

__all__ = ["WorkspaceListParams"]


class WorkspaceListParams(TypedDict, total=False):
    filters: Optional[Dict[str, object]]
