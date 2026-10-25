# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

import httpx

from ..types import peer_card_params
from .._types import Body, Omit, Query, Headers, NotGiven, omit, not_given
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
from ..types.peer_card_response import PeerCardResponse
from ..types.peer_delete_response import PeerDeleteResponse

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

    def delete(
        self,
        peer_id: str,
        *,
        workspace_id: str,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> PeerDeleteResponse:
        """
        Args:
          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not workspace_id:
            raise ValueError(f"Expected a non-empty value for `workspace_id` but received {workspace_id!r}")
        if not peer_id:
            raise ValueError(f"Expected a non-empty value for `peer_id` but received {peer_id!r}")
        return self._delete(
            path_template("/workspaces/{workspace_id}/peers/{peer_id}", workspace_id=workspace_id, peer_id=peer_id),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=PeerDeleteResponse,
        )

    def card(
        self,
        peer_id: str,
        *,
        workspace_id: str,
        target: str | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> PeerCardResponse:
        """
        Args:
          target: The peer whose card to get.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not workspace_id:
            raise ValueError(f"Expected a non-empty value for `workspace_id` but received {workspace_id!r}")
        if not peer_id:
            raise ValueError(f"Expected a non-empty value for `peer_id` but received {peer_id!r}")
        return self._get(
            path_template(
                "/workspaces/{workspace_id}/peers/{peer_id}/card", workspace_id=workspace_id, peer_id=peer_id
            ),
            options=make_request_options(
                extra_headers=extra_headers,
                extra_query=extra_query,
                extra_body=extra_body,
                timeout=timeout,
                query=maybe_transform({"target": target}, peer_card_params.PeerCardParams),
            ),
            cast_to=PeerCardResponse,
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

    async def delete(
        self,
        peer_id: str,
        *,
        workspace_id: str,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> PeerDeleteResponse:
        """
        Args:
          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not workspace_id:
            raise ValueError(f"Expected a non-empty value for `workspace_id` but received {workspace_id!r}")
        if not peer_id:
            raise ValueError(f"Expected a non-empty value for `peer_id` but received {peer_id!r}")
        return await self._delete(
            path_template("/workspaces/{workspace_id}/peers/{peer_id}", workspace_id=workspace_id, peer_id=peer_id),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=PeerDeleteResponse,
        )

    async def card(
        self,
        peer_id: str,
        *,
        workspace_id: str,
        target: str | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> PeerCardResponse:
        """
        Args:
          target: The peer whose card to get.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not workspace_id:
            raise ValueError(f"Expected a non-empty value for `workspace_id` but received {workspace_id!r}")
        if not peer_id:
            raise ValueError(f"Expected a non-empty value for `peer_id` but received {peer_id!r}")
        return await self._get(
            path_template(
                "/workspaces/{workspace_id}/peers/{peer_id}/card", workspace_id=workspace_id, peer_id=peer_id
            ),
            options=make_request_options(
                extra_headers=extra_headers,
                extra_query=extra_query,
                extra_body=extra_body,
                timeout=timeout,
                query=await async_maybe_transform({"target": target}, peer_card_params.PeerCardParams),
            ),
            cast_to=PeerCardResponse,
        )


class PeersResourceWithRawResponse:
    def __init__(self, peers: PeersResource) -> None:
        self._peers = peers

        self.delete = to_raw_response_wrapper(
            peers.delete,
        )
        self.card = to_raw_response_wrapper(
            peers.card,
        )


class AsyncPeersResourceWithRawResponse:
    def __init__(self, peers: AsyncPeersResource) -> None:
        self._peers = peers

        self.delete = async_to_raw_response_wrapper(
            peers.delete,
        )
        self.card = async_to_raw_response_wrapper(
            peers.card,
        )


class PeersResourceWithStreamingResponse:
    def __init__(self, peers: PeersResource) -> None:
        self._peers = peers

        self.delete = to_streamed_response_wrapper(
            peers.delete,
        )
        self.card = to_streamed_response_wrapper(
            peers.card,
        )


class AsyncPeersResourceWithStreamingResponse:
    def __init__(self, peers: AsyncPeersResource) -> None:
        self._peers = peers

        self.delete = async_to_streamed_response_wrapper(
            peers.delete,
        )
        self.card = async_to_streamed_response_wrapper(
            peers.card,
        )
