# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import Union
from datetime import datetime
from typing_extensions import overload

import httpx

from ..types import guide_activate_params
from .._types import Body, Omit, Query, Headers, NotGiven, omit, not_given
from .._utils import path_template, required_args, maybe_transform, async_maybe_transform
from .._compat import cached_property
from .._resource import SyncAPIResource, AsyncAPIResource
from .._response import (
    to_raw_response_wrapper,
    to_streamed_response_wrapper,
    async_to_raw_response_wrapper,
    async_to_streamed_response_wrapper,
)
from .._base_client import make_request_options
from ..types.guide_activate_response import GuideActivateResponse

__all__ = ["GuidesResource", "AsyncGuidesResource"]


class GuidesResource(SyncAPIResource):
    @cached_property
    def with_raw_response(self) -> GuidesResourceWithRawResponse:
        """
        This property can be used as a prefix for any HTTP method call to return
        the raw response object instead of the parsed content.

        For more information, see https://www.github.com/fx/fx-python#accessing-raw-response-data-eg-headers
        """
        return GuidesResourceWithRawResponse(self)

    @cached_property
    def with_streaming_response(self) -> GuidesResourceWithStreamingResponse:
        """
        An alternative to `.with_raw_response` that doesn't eagerly read the response body.

        For more information, see https://www.github.com/fx/fx-python#with_streaming_response
        """
        return GuidesResourceWithStreamingResponse(self)

    @overload
    def activate(
        self,
        guide_key: str,
        *,
        environment: str,
        status: bool,
        branch: str | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> GuideActivateResponse:
        """
        Activates a guide.

        Args:
          environment: The environment slug.

          status: Whether to activate or deactivate the guide.

          branch: The slug of a branch to use.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @overload
    def activate(
        self,
        guide_key: str,
        *,
        environment: str,
        branch: str | Omit = omit,
        from_: Union[str, datetime] | Omit = omit,
        until: Union[str, datetime] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> GuideActivateResponse:
        """
        Activates a guide.

        Args:
          environment: The environment slug.

          branch: The slug of a branch to use.

          from_: When to activate the guide.

          until: When to deactivate the guide.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @required_args(["environment", "status"], ["environment"])
    def activate(
        self,
        guide_key: str,
        *,
        environment: str,
        status: bool | Omit = omit,
        branch: str | Omit = omit,
        from_: Union[str, datetime] | Omit = omit,
        until: Union[str, datetime] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> GuideActivateResponse:
        if not guide_key:
            raise ValueError(f"Expected a non-empty value for `guide_key` but received {guide_key!r}")
        return self._put(
            path_template("/guides/{guide_key}/activate", guide_key=guide_key),
            body=maybe_transform(
                {
                    "status": status,
                    "from_": from_,
                    "until": until,
                },
                guide_activate_params.GuideActivateParams,
            ),
            options=make_request_options(
                extra_headers=extra_headers,
                extra_query=extra_query,
                extra_body=extra_body,
                timeout=timeout,
                query=maybe_transform(
                    {
                        "environment": environment,
                        "branch": branch,
                    },
                    guide_activate_params.GuideActivateParams,
                ),
            ),
            cast_to=GuideActivateResponse,
        )


class AsyncGuidesResource(AsyncAPIResource):
    @cached_property
    def with_raw_response(self) -> AsyncGuidesResourceWithRawResponse:
        """
        This property can be used as a prefix for any HTTP method call to return
        the raw response object instead of the parsed content.

        For more information, see https://www.github.com/fx/fx-python#accessing-raw-response-data-eg-headers
        """
        return AsyncGuidesResourceWithRawResponse(self)

    @cached_property
    def with_streaming_response(self) -> AsyncGuidesResourceWithStreamingResponse:
        """
        An alternative to `.with_raw_response` that doesn't eagerly read the response body.

        For more information, see https://www.github.com/fx/fx-python#with_streaming_response
        """
        return AsyncGuidesResourceWithStreamingResponse(self)

    @overload
    async def activate(
        self,
        guide_key: str,
        *,
        environment: str,
        status: bool,
        branch: str | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> GuideActivateResponse:
        """
        Activates a guide.

        Args:
          environment: The environment slug.

          status: Whether to activate or deactivate the guide.

          branch: The slug of a branch to use.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @overload
    async def activate(
        self,
        guide_key: str,
        *,
        environment: str,
        branch: str | Omit = omit,
        from_: Union[str, datetime] | Omit = omit,
        until: Union[str, datetime] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> GuideActivateResponse:
        """
        Activates a guide.

        Args:
          environment: The environment slug.

          branch: The slug of a branch to use.

          from_: When to activate the guide.

          until: When to deactivate the guide.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @required_args(["environment", "status"], ["environment"])
    async def activate(
        self,
        guide_key: str,
        *,
        environment: str,
        status: bool | Omit = omit,
        branch: str | Omit = omit,
        from_: Union[str, datetime] | Omit = omit,
        until: Union[str, datetime] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> GuideActivateResponse:
        if not guide_key:
            raise ValueError(f"Expected a non-empty value for `guide_key` but received {guide_key!r}")
        return await self._put(
            path_template("/guides/{guide_key}/activate", guide_key=guide_key),
            body=await async_maybe_transform(
                {
                    "status": status,
                    "from_": from_,
                    "until": until,
                },
                guide_activate_params.GuideActivateParams,
            ),
            options=make_request_options(
                extra_headers=extra_headers,
                extra_query=extra_query,
                extra_body=extra_body,
                timeout=timeout,
                query=await async_maybe_transform(
                    {
                        "environment": environment,
                        "branch": branch,
                    },
                    guide_activate_params.GuideActivateParams,
                ),
            ),
            cast_to=GuideActivateResponse,
        )


class GuidesResourceWithRawResponse:
    def __init__(self, guides: GuidesResource) -> None:
        self._guides = guides

        self.activate = to_raw_response_wrapper(
            guides.activate,
        )


class AsyncGuidesResourceWithRawResponse:
    def __init__(self, guides: AsyncGuidesResource) -> None:
        self._guides = guides

        self.activate = async_to_raw_response_wrapper(
            guides.activate,
        )


class GuidesResourceWithStreamingResponse:
    def __init__(self, guides: GuidesResource) -> None:
        self._guides = guides

        self.activate = to_streamed_response_wrapper(
            guides.activate,
        )


class AsyncGuidesResourceWithStreamingResponse:
    def __init__(self, guides: AsyncGuidesResource) -> None:
        self._guides = guides

        self.activate = async_to_streamed_response_wrapper(
            guides.activate,
        )
