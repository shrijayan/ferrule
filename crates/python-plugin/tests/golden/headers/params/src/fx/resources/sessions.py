# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import Optional
from typing_extensions import Literal, overload

import httpx

from ..types import session_act_params
from .._types import Body, Omit, Query, Headers, NotGiven, omit, not_given
from .._utils import is_given, path_template, required_args, maybe_transform, strip_not_given, async_maybe_transform
from .._compat import cached_property
from .._resource import SyncAPIResource, AsyncAPIResource
from .._response import (
    to_raw_response_wrapper,
    to_streamed_response_wrapper,
    async_to_raw_response_wrapper,
    async_to_streamed_response_wrapper,
)
from .._streaming import Stream, AsyncStream
from .._base_client import make_request_options
from ..types.stream_event import StreamEvent
from ..types.session_act_response import SessionActResponse
from ..types.session_end_response import SessionEndResponse

__all__ = ["SessionsResource", "AsyncSessionsResource"]


class SessionsResource(SyncAPIResource):
    @cached_property
    def with_raw_response(self) -> SessionsResourceWithRawResponse:
        """
        This property can be used as a prefix for any HTTP method call to return
        the raw response object instead of the parsed content.

        For more information, see https://www.github.com/fx/fx-python#accessing-raw-response-data-eg-headers
        """
        return SessionsResourceWithRawResponse(self)

    @cached_property
    def with_streaming_response(self) -> SessionsResourceWithStreamingResponse:
        """
        An alternative to `.with_raw_response` that doesn't eagerly read the response body.

        For more information, see https://www.github.com/fx/fx-python#with_streaming_response
        """
        return SessionsResourceWithStreamingResponse(self)

    @overload
    def act(
        self,
        id: str,
        *,
        input: str,
        frame_id: Optional[str] | Omit = omit,
        stream_response: Literal[False] | Omit = omit,
        x_stream_response: Literal["true", "false"] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> SessionActResponse:
        """
        Executes a browser action using natural language instructions.

        Args:
          input: Natural language instruction

          frame_id: Target frame ID for the action

          stream_response: Whether to stream the response via SSE

          x_stream_response: Whether to stream the response via SSE

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @overload
    def act(
        self,
        id: str,
        *,
        input: str,
        stream_response: Literal[True],
        frame_id: Optional[str] | Omit = omit,
        x_stream_response: Literal["true", "false"] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> Stream[StreamEvent]:
        """
        Executes a browser action using natural language instructions.

        Args:
          input: Natural language instruction

          stream_response: Whether to stream the response via SSE

          frame_id: Target frame ID for the action

          x_stream_response: Whether to stream the response via SSE

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @overload
    def act(
        self,
        id: str,
        *,
        input: str,
        stream_response: bool,
        frame_id: Optional[str] | Omit = omit,
        x_stream_response: Literal["true", "false"] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> SessionActResponse | Stream[StreamEvent]:
        """
        Executes a browser action using natural language instructions.

        Args:
          input: Natural language instruction

          stream_response: Whether to stream the response via SSE

          frame_id: Target frame ID for the action

          x_stream_response: Whether to stream the response via SSE

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @required_args(["input"], ["input", "stream_response"])
    def act(
        self,
        id: str,
        *,
        input: str,
        frame_id: Optional[str] | Omit = omit,
        stream_response: Literal[False] | Literal[True] | Omit = omit,
        x_stream_response: Literal["true", "false"] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> SessionActResponse | Stream[StreamEvent]:
        if not id:
            raise ValueError(f"Expected a non-empty value for `id` but received {id!r}")
        extra_headers = {
            **strip_not_given(
                {"x-stream-response": str(x_stream_response) if is_given(x_stream_response) else not_given}
            ),
            **(extra_headers or {}),
        }
        return self._post(
            path_template("/sessions/{id}/act", id=id),
            body=maybe_transform(
                {
                    "input": input,
                    "frame_id": frame_id,
                    "stream_response": stream_response,
                },
                session_act_params.SessionActParamsStreaming
                if stream_response
                else session_act_params.SessionActParamsNonStreaming,
            ),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=SessionActResponse,
            stream=stream_response or False,
            stream_cls=Stream[StreamEvent],
        )

    def end(
        self,
        id: str,
        *,
        x_stream_response: Literal["true", "false"] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> SessionEndResponse:
        """
        Terminates the browser session and releases all associated resources.

        Args:
          x_stream_response: Whether to stream the response via SSE

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not id:
            raise ValueError(f"Expected a non-empty value for `id` but received {id!r}")
        extra_headers = {
            **strip_not_given(
                {"x-stream-response": str(x_stream_response) if is_given(x_stream_response) else not_given}
            ),
            **(extra_headers or {}),
        }
        return self._post(
            path_template("/sessions/{id}/end", id=id),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=SessionEndResponse,
        )


class AsyncSessionsResource(AsyncAPIResource):
    @cached_property
    def with_raw_response(self) -> AsyncSessionsResourceWithRawResponse:
        """
        This property can be used as a prefix for any HTTP method call to return
        the raw response object instead of the parsed content.

        For more information, see https://www.github.com/fx/fx-python#accessing-raw-response-data-eg-headers
        """
        return AsyncSessionsResourceWithRawResponse(self)

    @cached_property
    def with_streaming_response(self) -> AsyncSessionsResourceWithStreamingResponse:
        """
        An alternative to `.with_raw_response` that doesn't eagerly read the response body.

        For more information, see https://www.github.com/fx/fx-python#with_streaming_response
        """
        return AsyncSessionsResourceWithStreamingResponse(self)

    @overload
    async def act(
        self,
        id: str,
        *,
        input: str,
        frame_id: Optional[str] | Omit = omit,
        stream_response: Literal[False] | Omit = omit,
        x_stream_response: Literal["true", "false"] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> SessionActResponse:
        """
        Executes a browser action using natural language instructions.

        Args:
          input: Natural language instruction

          frame_id: Target frame ID for the action

          stream_response: Whether to stream the response via SSE

          x_stream_response: Whether to stream the response via SSE

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @overload
    async def act(
        self,
        id: str,
        *,
        input: str,
        stream_response: Literal[True],
        frame_id: Optional[str] | Omit = omit,
        x_stream_response: Literal["true", "false"] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> AsyncStream[StreamEvent]:
        """
        Executes a browser action using natural language instructions.

        Args:
          input: Natural language instruction

          stream_response: Whether to stream the response via SSE

          frame_id: Target frame ID for the action

          x_stream_response: Whether to stream the response via SSE

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @overload
    async def act(
        self,
        id: str,
        *,
        input: str,
        stream_response: bool,
        frame_id: Optional[str] | Omit = omit,
        x_stream_response: Literal["true", "false"] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> SessionActResponse | AsyncStream[StreamEvent]:
        """
        Executes a browser action using natural language instructions.

        Args:
          input: Natural language instruction

          stream_response: Whether to stream the response via SSE

          frame_id: Target frame ID for the action

          x_stream_response: Whether to stream the response via SSE

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        ...

    @required_args(["input"], ["input", "stream_response"])
    async def act(
        self,
        id: str,
        *,
        input: str,
        frame_id: Optional[str] | Omit = omit,
        stream_response: Literal[False] | Literal[True] | Omit = omit,
        x_stream_response: Literal["true", "false"] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> SessionActResponse | AsyncStream[StreamEvent]:
        if not id:
            raise ValueError(f"Expected a non-empty value for `id` but received {id!r}")
        extra_headers = {
            **strip_not_given(
                {"x-stream-response": str(x_stream_response) if is_given(x_stream_response) else not_given}
            ),
            **(extra_headers or {}),
        }
        return await self._post(
            path_template("/sessions/{id}/act", id=id),
            body=await async_maybe_transform(
                {
                    "input": input,
                    "frame_id": frame_id,
                    "stream_response": stream_response,
                },
                session_act_params.SessionActParamsStreaming
                if stream_response
                else session_act_params.SessionActParamsNonStreaming,
            ),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=SessionActResponse,
            stream=stream_response or False,
            stream_cls=AsyncStream[StreamEvent],
        )

    async def end(
        self,
        id: str,
        *,
        x_stream_response: Literal["true", "false"] | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> SessionEndResponse:
        """
        Terminates the browser session and releases all associated resources.

        Args:
          x_stream_response: Whether to stream the response via SSE

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not id:
            raise ValueError(f"Expected a non-empty value for `id` but received {id!r}")
        extra_headers = {
            **strip_not_given(
                {"x-stream-response": str(x_stream_response) if is_given(x_stream_response) else not_given}
            ),
            **(extra_headers or {}),
        }
        return await self._post(
            path_template("/sessions/{id}/end", id=id),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=SessionEndResponse,
        )


class SessionsResourceWithRawResponse:
    def __init__(self, sessions: SessionsResource) -> None:
        self._sessions = sessions

        self.act = to_raw_response_wrapper(
            sessions.act,
        )
        self.end = to_raw_response_wrapper(
            sessions.end,
        )


class AsyncSessionsResourceWithRawResponse:
    def __init__(self, sessions: AsyncSessionsResource) -> None:
        self._sessions = sessions

        self.act = async_to_raw_response_wrapper(
            sessions.act,
        )
        self.end = async_to_raw_response_wrapper(
            sessions.end,
        )


class SessionsResourceWithStreamingResponse:
    def __init__(self, sessions: SessionsResource) -> None:
        self._sessions = sessions

        self.act = to_streamed_response_wrapper(
            sessions.act,
        )
        self.end = to_streamed_response_wrapper(
            sessions.end,
        )


class AsyncSessionsResourceWithStreamingResponse:
    def __init__(self, sessions: AsyncSessionsResource) -> None:
        self._sessions = sessions

        self.act = async_to_streamed_response_wrapper(
            sessions.act,
        )
        self.end = async_to_streamed_response_wrapper(
            sessions.end,
        )
