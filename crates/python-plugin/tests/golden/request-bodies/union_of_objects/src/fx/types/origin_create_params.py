# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import Union
from typing_extensions import Literal, Required, Annotated, TypeAlias, TypedDict

from .._utils import PropertyInfo

__all__ = ["OriginCreateParams", "S3", "S3Credentials", "CloudinaryBackup", "WebProxy"]


class S3(TypedDict, total=False):
    bucket: Required[str]
    """S3 bucket name."""

    name: Required[str]
    """Display name of the origin."""

    type: Required[Literal["S3"]]

    credentials: S3Credentials
    """Bucket credentials."""


class S3Credentials(TypedDict, total=False):
    """Bucket credentials."""

    access_key: Required[Annotated[str, PropertyInfo(alias="accessKey")]]
    """Access key for the bucket."""


class CloudinaryBackup(TypedDict, total=False):
    bucket: Required[str]
    """Cloudinary bucket name."""

    name: Required[str]
    """Display name of the origin."""

    type: Required[Literal["CLOUDINARY_BACKUP"]]


class WebProxy(TypedDict, total=False):
    name: Required[str]
    """Display name of the origin."""

    type: Required[Literal["WEB_PROXY"]]

    forward_host_header: Annotated[bool, PropertyInfo(alias="forwardHostHeader")]
    """Whether to forward the Host header."""


OriginCreateParams: TypeAlias = Union[S3, CloudinaryBackup, WebProxy]
