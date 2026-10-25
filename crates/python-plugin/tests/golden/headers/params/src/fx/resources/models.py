# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

import httpx

from ..types import model_list_params
from .._types import Body, Omit, Query, Headers, NotGiven, omit, not_given
from .._utils import path_template, maybe_transform, strip_not_given, async_maybe_transform
from .._compat import cached_property
from .._resource import SyncAPIResource, AsyncAPIResource
from .._response import (
    to_raw_response_wrapper,
    to_streamed_response_wrapper,
    async_to_raw_response_wrapper,
    async_to_streamed_response_wrapper,
)
from ..types.model import Model
from .._base_client import make_request_options
from ..types.model_list_response import ModelListResponse

__all__ = ["ModelsResource", "AsyncModelsResource"]


class ModelsResource(SyncAPIResource):
    @cached_property
    def with_raw_response(self) -> ModelsResourceWithRawResponse:
        """
        This property can be used as a prefix for any HTTP method call to return
        the raw response object instead of the parsed content.

        For more information, see https://www.github.com/fx/fx-python#accessing-raw-response-data-eg-headers
        """
        return ModelsResourceWithRawResponse(self)

    @cached_property
    def with_streaming_response(self) -> ModelsResourceWithStreamingResponse:
        """
        An alternative to `.with_raw_response` that doesn't eagerly read the response body.

        For more information, see https://www.github.com/fx/fx-python#with_streaming_response
        """
        return ModelsResourceWithStreamingResponse(self)

    def retrieve(
        self,
        model_id: str,
        *,
        anthropic_version: str | Omit = omit,
        x_goog_api_client: str | Omit = omit,
        x_goog_api_key: str | Omit = omit,
        x_goog_user_project: str | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> Model:
        """
        Get a model by its identifier.

        Args:
          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not model_id:
            raise ValueError(f"Expected a non-empty value for `model_id` but received {model_id!r}")
        extra_headers = {
            **strip_not_given(
                {
                    "anthropic-version": anthropic_version,
                    "x-goog-api-client": x_goog_api_client,
                    "x-goog-api-key": x_goog_api_key,
                    "x-goog-user-project": x_goog_user_project,
                }
            ),
            **(extra_headers or {}),
        }
        return self._get(
            path_template("/models/{model_id}", model_id=model_id),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=Model,
        )

    def list(
        self,
        *,
        after_id: str | Omit = omit,
        limit: int | Omit = omit,
        anthropic_version: str | Omit = omit,
        x_goog_api_client: str | Omit = omit,
        x_goog_api_key: str | Omit = omit,
        x_goog_user_project: str | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> ModelListResponse:
        """
        List models.

        Args:
          after_id: Return models after this model ID.

          limit: Maximum number of models to return.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        extra_headers = {
            **strip_not_given(
                {
                    "anthropic-version": anthropic_version,
                    "x-goog-api-client": x_goog_api_client,
                    "x-goog-api-key": x_goog_api_key,
                    "x-goog-user-project": x_goog_user_project,
                }
            ),
            **(extra_headers or {}),
        }
        return self._get(
            "/models",
            options=make_request_options(
                extra_headers=extra_headers,
                extra_query=extra_query,
                extra_body=extra_body,
                timeout=timeout,
                query=maybe_transform(
                    {
                        "after_id": after_id,
                        "limit": limit,
                    },
                    model_list_params.ModelListParams,
                ),
            ),
            cast_to=ModelListResponse,
        )


class AsyncModelsResource(AsyncAPIResource):
    @cached_property
    def with_raw_response(self) -> AsyncModelsResourceWithRawResponse:
        """
        This property can be used as a prefix for any HTTP method call to return
        the raw response object instead of the parsed content.

        For more information, see https://www.github.com/fx/fx-python#accessing-raw-response-data-eg-headers
        """
        return AsyncModelsResourceWithRawResponse(self)

    @cached_property
    def with_streaming_response(self) -> AsyncModelsResourceWithStreamingResponse:
        """
        An alternative to `.with_raw_response` that doesn't eagerly read the response body.

        For more information, see https://www.github.com/fx/fx-python#with_streaming_response
        """
        return AsyncModelsResourceWithStreamingResponse(self)

    async def retrieve(
        self,
        model_id: str,
        *,
        anthropic_version: str | Omit = omit,
        x_goog_api_client: str | Omit = omit,
        x_goog_api_key: str | Omit = omit,
        x_goog_user_project: str | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> Model:
        """
        Get a model by its identifier.

        Args:
          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        if not model_id:
            raise ValueError(f"Expected a non-empty value for `model_id` but received {model_id!r}")
        extra_headers = {
            **strip_not_given(
                {
                    "anthropic-version": anthropic_version,
                    "x-goog-api-client": x_goog_api_client,
                    "x-goog-api-key": x_goog_api_key,
                    "x-goog-user-project": x_goog_user_project,
                }
            ),
            **(extra_headers or {}),
        }
        return await self._get(
            path_template("/models/{model_id}", model_id=model_id),
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=Model,
        )

    async def list(
        self,
        *,
        after_id: str | Omit = omit,
        limit: int | Omit = omit,
        anthropic_version: str | Omit = omit,
        x_goog_api_client: str | Omit = omit,
        x_goog_api_key: str | Omit = omit,
        x_goog_user_project: str | Omit = omit,
        # Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.
        # The extra values given here take precedence over values defined on the client or passed to this method.
        extra_headers: Headers | None = None,
        extra_query: Query | None = None,
        extra_body: Body | None = None,
        timeout: float | httpx.Timeout | None | NotGiven = not_given,
    ) -> ModelListResponse:
        """
        List models.

        Args:
          after_id: Return models after this model ID.

          limit: Maximum number of models to return.

          extra_headers: Send extra headers

          extra_query: Add additional query parameters to the request

          extra_body: Add additional JSON properties to the request

          timeout: Override the client-level default timeout for this request, in seconds
        """
        extra_headers = {
            **strip_not_given(
                {
                    "anthropic-version": anthropic_version,
                    "x-goog-api-client": x_goog_api_client,
                    "x-goog-api-key": x_goog_api_key,
                    "x-goog-user-project": x_goog_user_project,
                }
            ),
            **(extra_headers or {}),
        }
        return await self._get(
            "/models",
            options=make_request_options(
                extra_headers=extra_headers,
                extra_query=extra_query,
                extra_body=extra_body,
                timeout=timeout,
                query=await async_maybe_transform(
                    {
                        "after_id": after_id,
                        "limit": limit,
                    },
                    model_list_params.ModelListParams,
                ),
            ),
            cast_to=ModelListResponse,
        )


class ModelsResourceWithRawResponse:
    def __init__(self, models: ModelsResource) -> None:
        self._models = models

        self.retrieve = to_raw_response_wrapper(
            models.retrieve,
        )
        self.list = to_raw_response_wrapper(
            models.list,
        )


class AsyncModelsResourceWithRawResponse:
    def __init__(self, models: AsyncModelsResource) -> None:
        self._models = models

        self.retrieve = async_to_raw_response_wrapper(
            models.retrieve,
        )
        self.list = async_to_raw_response_wrapper(
            models.list,
        )


class ModelsResourceWithStreamingResponse:
    def __init__(self, models: ModelsResource) -> None:
        self._models = models

        self.retrieve = to_streamed_response_wrapper(
            models.retrieve,
        )
        self.list = to_streamed_response_wrapper(
            models.list,
        )


class AsyncModelsResourceWithStreamingResponse:
    def __init__(self, models: AsyncModelsResource) -> None:
        self._models = models

        self.retrieve = async_to_streamed_response_wrapper(
            models.retrieve,
        )
        self.list = async_to_streamed_response_wrapper(
            models.list,
        )
