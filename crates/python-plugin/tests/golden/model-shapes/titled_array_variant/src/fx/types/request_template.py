# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import List, Union

from .._models import BaseModel

__all__ = ["RequestTemplate", "HeadersRequestTemplateHeadersArray"]


class HeadersRequestTemplateHeadersArray(BaseModel):
    key: str
    """The key of the header."""

    value: str
    """The value of the header."""


class RequestTemplate(BaseModel):
    """A request template for a fetch function step."""

    url: str
    """The URL of the request."""

    headers: Union[str, List[HeadersRequestTemplateHeadersArray], None] = None
    """The headers of the request.

    Can be a template string or a list of key-value pairs.
    """
