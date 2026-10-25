# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import Union, Optional
from typing_extensions import Required, TypeAlias, TypedDict

from .._types import SequenceNotStr

__all__ = [
    "WorkflowRunParams",
    "Recipient",
    "RecipientObjectRecipientReference",
    "Actor",
    "ActorObjectRecipientReference",
]


class WorkflowRunParams(TypedDict, total=False):
    recipients: Required[SequenceNotStr[Recipient]]
    """A list of recipients to run the workflow for."""

    actor: Optional[Actor]
    """
    A recipient reference, used when referencing a recipient by either their ID (for
    a user), or by a reference for an object.
    """

    tenant: str
    """The tenant to associate the workflow run with."""


class RecipientObjectRecipientReference(TypedDict, total=False):
    """An object reference."""

    id: Required[str]
    """The ID of the object."""

    collection: Required[str]
    """The collection of the object."""


Recipient: TypeAlias = Union[str, RecipientObjectRecipientReference]


class ActorObjectRecipientReference(TypedDict, total=False):
    """An object reference."""

    id: Required[str]
    """The ID of the object."""

    collection: Required[str]
    """The collection of the object."""


Actor: TypeAlias = Union[str, ActorObjectRecipientReference]
