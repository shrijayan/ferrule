# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing_extensions import Annotated, TypedDict

from .._utils import PropertyInfo

__all__ = ["ModelListParams"]


class ModelListParams(TypedDict, total=False):
    after_id: str
    """Return models after this model ID."""

    limit: int
    """Maximum number of models to return."""

    anthropic_version: Annotated[str, PropertyInfo(alias="anthropic-version")]

    x_goog_api_client: Annotated[str, PropertyInfo(alias="x-goog-api-client")]

    x_goog_api_key: Annotated[str, PropertyInfo(alias="x-goog-api-key")]

    x_goog_user_project: Annotated[str, PropertyInfo(alias="x-goog-user-project")]
