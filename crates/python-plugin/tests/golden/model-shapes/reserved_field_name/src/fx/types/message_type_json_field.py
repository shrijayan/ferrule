# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import Optional

from pydantic import Field as FieldInfo

from .._models import BaseModel

__all__ = ["MessageTypeJsonField"]


class MessageTypeJsonField(BaseModel):
    """A JSON field used in a message type."""

    key: str
    """The unique key of the field."""

    schema_: Optional[str] = FieldInfo(alias="schema", default=None)
    """A JSON schema used to validate the structure of the JSON provided.

    Must be a valid JSON schema.
    """
