# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import Dict

import httpx

from ..types import peer_add_params
from .._types import Body, Query, Headers, NotGiven, SequenceNotStr, not_given
from .._utils import path_template, maybe_transform, async_maybe_transform
from .._compat import cached_property
from .._resource import SyncAPIResource, AsyncAPIResource
from .._response import (
    to_raw_response_wrapper,
    to_streamed_response_wrapper,
    async_to_raw_response_wrapper,
    async_to_streamed_response_wrapper,
)
from .._base_client import make_request_options
from ..types.peer_add_response import PeerAddResponse
from ..types.peer_remove_response import PeerRemoveResponse

__all__ = ["PeersResource", "AsyncPeersResource"]


class PeersResource(SyncAPIResource):
    @cached_property
    def with_raw_response(self) -> PeersResourceWithRawResponse:
        """
        This property can be used as a prefix for any HTTP method call to return
        the raw response object instead of the parsed content.

        For more information, see https://www.github.com/fx/fx-python#accessing-raw-response-data-eg-headers
        """
        return PeersResourceWithRawResponse(self)

    @cached_property
    def with_streaming_response(self) -> PeersResourceWithStreamingResponse:
        """
        An alternative to `.with_raw_response` that doesn't eagerly read the response body.

        For more information, see https://www.github.com/fx/fx-python#with_streaming_response
        """
        return PeersResourceWithStreamingResponse(self)

    def add(
        self,
        session_id: str,
        *,
        body: Dict[str, str],
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> PeerAddResponse:
        """
        Args:
          body: Peers to add

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not session_id:
            raise ValueError(f"Expected a non-empty value for `session_id` but received {session_id!r}")
        return self._post(
            path_template("/sessions/{session_id}/peers", session_id=session_id),
            body=maybe_transform(body, peer_add_params.PeerAddParams),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=PeerAddResponse,
        )

    def remove(
        self,
        session_id: str,
        *,
        body: SequenceNotStr[str],
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> PeerRemoveResponse:
        """
        Args:
          body: Peer IDs to remove

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not session_id:
            raise ValueError(f"Expected a non-empty value for `session_id` but received {session_id!r}")
        return self._delete(
            path_template("/sessions/{session_id}/peers", session_id=session_id),
            body=maybe_transform(body, SequenceNotStr[str]),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=PeerRemoveResponse,
        )


class AsyncPeersResource(AsyncAPIResource):
    @cached_property
    def with_raw_response(self) -> AsyncPeersResourceWithRawResponse:
        """
        This property can be used as a prefix for any HTTP method call to return
        the raw response object instead of the parsed content.

        For more information, see https://www.github.com/fx/fx-python#accessing-raw-response-data-eg-headers
        """
        return AsyncPeersResourceWithRawResponse(self)

    @cached_property
    def with_streaming_response(self) -> AsyncPeersResourceWithStreamingResponse:
        """
        An alternative to `.with_raw_response` that doesn't eagerly read the response body.

        For more information, see https://www.github.com/fx/fx-python#with_streaming_response
        """
        return AsyncPeersResourceWithStreamingResponse(self)

    async def add(
        self,
        session_id: str,
        *,
        body: Dict[str, str],
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> PeerAddResponse:
        """
        Args:
          body: Peers to add

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not session_id:
            raise ValueError(f"Expected a non-empty value for `session_id` but received {session_id!r}")
        return await self._post(
            path_template("/sessions/{session_id}/peers", session_id=session_id),
            body=await async_maybe_transform(body, peer_add_params.PeerAddParams),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=PeerAddResponse,
        )

    async def remove(
        self,
        session_id: str,
        *,
        body: SequenceNotStr[str],
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> PeerRemoveResponse:
        """
        Args:
          body: Peer IDs to remove

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not session_id:
            raise ValueError(f"Expected a non-empty value for `session_id` but received {session_id!r}")
        return await self._delete(
            path_template("/sessions/{session_id}/peers", session_id=session_id),
            body=await async_maybe_transform(body, SequenceNotStr[str]),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=PeerRemoveResponse,
        )


class PeersResourceWithRawResponse:
    def __init__(self, peers: PeersResource) -> None:
        self._peers = peers

        self.add = to_raw_response_wrapper(
            peers.add,
        )
        self.remove = to_raw_response_wrapper(
            peers.remove,
        )


class AsyncPeersResourceWithRawResponse:
    def __init__(self, peers: AsyncPeersResource) -> None:
        self._peers = peers

        self.add = async_to_raw_response_wrapper(
            peers.add,
        )
        self.remove = async_to_raw_response_wrapper(
            peers.remove,
        )


class PeersResourceWithStreamingResponse:
    def __init__(self, peers: PeersResource) -> None:
        self._peers = peers

        self.add = to_streamed_response_wrapper(
            peers.add,
        )
        self.remove = to_streamed_response_wrapper(
            peers.remove,
        )


class AsyncPeersResourceWithStreamingResponse:
    def __init__(self, peers: AsyncPeersResource) -> None:
        self._peers = peers

        self.add = async_to_streamed_response_wrapper(
            peers.add,
        )
        self.remove = async_to_streamed_response_wrapper(
            peers.remove,
        )
