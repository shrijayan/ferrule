# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import Optional

from pydantic import Field as FieldInfo

from .._models import BaseModel

__all__ = ["Tenant"]


class Tenant(BaseModel):
    """A tenant entity."""

    id: str
    """The unique identifier for the tenant."""

    api_typename: str = FieldInfo(alias="__typename")
    """The typename of the schema."""

    api_model_id: Optional[str] = FieldInfo(alias="model_id", default=None)
    """The model to use."""
