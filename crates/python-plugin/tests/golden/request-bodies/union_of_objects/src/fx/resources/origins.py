# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing_extensions import Literal, overload

import httpx

from ..types import origin_create_params
from .._types import Body, Omit, Query, Headers, NotGiven, omit, not_given
from .._utils import required_args, maybe_transform, async_maybe_transform
from .._compat import cached_property
from .._resource import SyncAPIResource, AsyncAPIResource
from .._response import (
    to_raw_response_wrapper,
    to_streamed_response_wrapper,
    async_to_raw_response_wrapper,
    async_to_streamed_response_wrapper,
)
from .._base_client import make_request_options
from ..types.origin_create_response import OriginCreateResponse

__all__ = ["OriginsResource", "AsyncOriginsResource"]


class OriginsResource(SyncAPIResource):
    @cached_property
    def with_raw_response(self) -> OriginsResourceWithRawResponse:
        """
        This property can be used as a prefix for any HTTP method call to return
        the raw response object instead of the parsed content.

        For more information, see https://www.github.com/fx/fx-python#accessing-raw-response-data-eg-headers
        """
        return OriginsResourceWithRawResponse(self)

    @cached_property
    def with_streaming_response(self) -> OriginsResourceWithStreamingResponse:
        """
        An alternative to `.with_raw_response` that doesn't eagerly read the response body.

        For more information, see https://www.github.com/fx/fx-python#with_streaming_response
        """
        return OriginsResourceWithStreamingResponse(self)

    @overload
    def create(
        self,
        *,
        bucket: str,
        name: str,
        type: Literal["S3"],
        credentials: origin_create_params.S3Credentials | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> OriginCreateResponse:
        """
        Creates an origin.

        Args:
          bucket: S3 bucket name.

          name: Display name of the origin.

          credentials: Bucket credentials.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @overload
    def create(
        self,
        *,
        bucket: str,
        name: str,
        type: Literal["CLOUDINARY_BACKUP"],
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> OriginCreateResponse:
        """
        Creates an origin.

        Args:
          bucket: Cloudinary bucket name.

          name: Display name of the origin.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @overload
    def create(
        self,
        *,
        name: str,
        type: Literal["WEB_PROXY"],
        forward_host_header: bool | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> OriginCreateResponse:
        """
        Creates an origin.

        Args:
          name: Display name of the origin.

          forward_host_header: Whether to forward the Host header.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @required_args(["bucket", "name", "type"], ["name", "type"])
    def create(
        self,
        *,
        bucket: str | Omit = omit,
        name: str,
        type: Literal["S3"] | Literal["CLOUDINARY_BACKUP"] | Literal["WEB_PROXY"],
        credentials: origin_create_params.S3Credentials | Omit = omit,
        forward_host_header: bool | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> OriginCreateResponse:
        return self._post(
            "/origins",
            body=maybe_transform(
                {
                    "bucket": bucket,
                    "name": name,
                    "type": type,
                    "credentials": credentials,
                    "forward_host_header": forward_host_header,
                },
                origin_create_params.OriginCreateParams,
            ),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=OriginCreateResponse,
        )


class AsyncOriginsResource(AsyncAPIResource):
    @cached_property
    def with_raw_response(self) -> AsyncOriginsResourceWithRawResponse:
        """
        This property can be used as a prefix for any HTTP method call to return
        the raw response object instead of the parsed content.

        For more information, see https://www.github.com/fx/fx-python#accessing-raw-response-data-eg-headers
        """
        return AsyncOriginsResourceWithRawResponse(self)

    @cached_property
    def with_streaming_response(self) -> AsyncOriginsResourceWithStreamingResponse:
        """
        An alternative to `.with_raw_response` that doesn't eagerly read the response body.

        For more information, see https://www.github.com/fx/fx-python#with_streaming_response
        """
        return AsyncOriginsResourceWithStreamingResponse(self)

    @overload
    async def create(
        self,
        *,
        bucket: str,
        name: str,
        type: Literal["S3"],
        credentials: origin_create_params.S3Credentials | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> OriginCreateResponse:
        """
        Creates an origin.

        Args:
          bucket: S3 bucket name.

          name: Display name of the origin.

          credentials: Bucket credentials.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @overload
    async def create(
        self,
        *,
        bucket: str,
        name: str,
        type: Literal["CLOUDINARY_BACKUP"],
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> OriginCreateResponse:
        """
        Creates an origin.

        Args:
          bucket: Cloudinary bucket name.

          name: Display name of the origin.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @overload
    async def create(
        self,
        *,
        name: str,
        type: Literal["WEB_PROXY"],
        forward_host_header: bool | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> OriginCreateResponse:
        """
        Creates an origin.

        Args:
          name: Display name of the origin.

          forward_host_header: Whether to forward the Host header.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @required_args(["bucket", "name", "type"], ["name", "type"])
    async def create(
        self,
        *,
        bucket: str | Omit = omit,
        name: str,
        type: Literal["S3"] | Literal["CLOUDINARY_BACKUP"] | Literal["WEB_PROXY"],
        credentials: origin_create_params.S3Credentials | Omit = omit,
        forward_host_header: bool | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> OriginCreateResponse:
        return await self._post(
            "/origins",
            body=await async_maybe_transform(
                {
                    "bucket": bucket,
                    "name": name,
                    "type": type,
                    "credentials": credentials,
                    "forward_host_header": forward_host_header,
                },
                origin_create_params.OriginCreateParams,
            ),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=OriginCreateResponse,
        )


class OriginsResourceWithRawResponse:
    def __init__(self, origins: OriginsResource) -> None:
        self._origins = origins

        self.create = to_raw_response_wrapper(
            origins.create,
        )


class AsyncOriginsResourceWithRawResponse:
    def __init__(self, origins: AsyncOriginsResource) -> None:
        self._origins = origins

        self.create = async_to_raw_response_wrapper(
            origins.create,
        )


class OriginsResourceWithStreamingResponse:
    def __init__(self, origins: OriginsResource) -> None:
        self._origins = origins

        self.create = to_streamed_response_wrapper(
            origins.create,
        )


class AsyncOriginsResourceWithStreamingResponse:
    def __init__(self, origins: AsyncOriginsResource) -> None:
        self._origins = origins

        self.create = async_to_streamed_response_wrapper(
            origins.create,
        )
