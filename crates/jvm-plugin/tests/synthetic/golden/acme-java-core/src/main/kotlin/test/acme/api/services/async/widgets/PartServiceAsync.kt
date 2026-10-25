// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services.async.widgets

import java.util.concurrent.CompletableFuture
import java.util.function.Consumer
import test.acme.api.core.ClientOptions
import test.acme.api.core.RequestOptions
import test.acme.api.core.http.HttpResponseFor
import test.acme.api.models.widgets.parts.PartRetrieveParams
import test.acme.api.models.widgets.parts.PartRetrieveResponse

interface PartServiceAsync {

    /**
     * Returns a view of this service that provides access to raw HTTP responses for each method.
     */
    fun withRawResponse(): WithRawResponse

    /**
     * Returns a view of this service with the given option modifications applied.
     *
     * The original service is not modified.
     */
    fun withOptions(modifier: Consumer<ClientOptions.Builder>): PartServiceAsync

    /** Retrieve one part of a widget */
    fun retrieve(
        partNumber: Long,
        params: PartRetrieveParams,
    ): CompletableFuture<PartRetrieveResponse> = retrieve(partNumber, params, RequestOptions.none())

    /** @see retrieve */
    fun retrieve(
        partNumber: Long,
        params: PartRetrieveParams,
        requestOptions: RequestOptions = RequestOptions.none(),
    ): CompletableFuture<PartRetrieveResponse> =
        retrieve(params.toBuilder().partNumber(partNumber).build(), requestOptions)

    /** @see retrieve */
    fun retrieve(params: PartRetrieveParams): CompletableFuture<PartRetrieveResponse> =
        retrieve(params, RequestOptions.none())

    /** @see retrieve */
    fun retrieve(
        params: PartRetrieveParams,
        requestOptions: RequestOptions = RequestOptions.none(),
    ): CompletableFuture<PartRetrieveResponse>

    /** A view of [PartServiceAsync] that provides access to raw HTTP responses for each method. */
    interface WithRawResponse {

        /**
         * Returns a view of this service with the given option modifications applied.
         *
         * The original service is not modified.
         */
        fun withOptions(modifier: Consumer<ClientOptions.Builder>): PartServiceAsync.WithRawResponse

        /**
         * Returns a raw HTTP response for `get /v1/widgets/{widget_id}/parts/{part_number}`, but is
         * otherwise the same as [PartServiceAsync.retrieve].
         */
        fun retrieve(
            partNumber: Long,
            params: PartRetrieveParams,
        ): CompletableFuture<HttpResponseFor<PartRetrieveResponse>> =
            retrieve(partNumber, params, RequestOptions.none())

        /** @see retrieve */
        fun retrieve(
            partNumber: Long,
            params: PartRetrieveParams,
            requestOptions: RequestOptions = RequestOptions.none(),
        ): CompletableFuture<HttpResponseFor<PartRetrieveResponse>> =
            retrieve(params.toBuilder().partNumber(partNumber).build(), requestOptions)

        /** @see retrieve */
        fun retrieve(
            params: PartRetrieveParams
        ): CompletableFuture<HttpResponseFor<PartRetrieveResponse>> =
            retrieve(params, RequestOptions.none())

        /** @see retrieve */
        fun retrieve(
            params: PartRetrieveParams,
            requestOptions: RequestOptions = RequestOptions.none(),
        ): CompletableFuture<HttpResponseFor<PartRetrieveResponse>>
    }
}
