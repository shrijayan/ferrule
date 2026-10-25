# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import Union
from typing_extensions import Literal, Annotated, TypeAlias

from .._utils import PropertyInfo
from .._models import BaseModel

__all__ = ["Source", "S3", "WebFolder"]


class S3(BaseModel):
    bucket: str

    type: Literal["S3"]


class WebFolder(BaseModel):
    base_url: str

    type: Literal["WEB_FOLDER"]


Source: TypeAlias = Annotated[Union[S3, WebFolder], PropertyInfo(discriminator="type")]
