# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing_extensions import Required, TypedDict

__all__ = ["PeerCardParams"]


class PeerCardParams(TypedDict, total=False):
    workspace_id: Required[str]
    """ID of the workspace"""

    target: str
    """The peer whose card to get."""
