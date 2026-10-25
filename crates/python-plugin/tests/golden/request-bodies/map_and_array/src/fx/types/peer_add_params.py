# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import Dict
from typing_extensions import Required, TypedDict

__all__ = ["PeerAddParams"]


class PeerAddParams(TypedDict, total=False):
    body: Required[Dict[str, str]]
    """Peers to add"""
