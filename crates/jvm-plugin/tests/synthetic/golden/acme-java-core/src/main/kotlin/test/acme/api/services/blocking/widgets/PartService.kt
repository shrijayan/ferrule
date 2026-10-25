// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services.blocking.widgets

import com.google.errorprone.annotations.MustBeClosed
import java.util.function.Consumer
import test.acme.api.core.ClientOptions
import test.acme.api.core.RequestOptions
import test.acme.api.core.http.HttpResponseFor
import test.acme.api.models.widgets.parts.PartRetrieveParams
import test.acme.api.models.widgets.parts.PartRetrieveResponse

interface PartService {

    /**
     * Returns a view of this service that provides access to raw HTTP responses for each method.
     */
    fun withRawResponse(): WithRawResponse

    /**
     * Returns a view of this service with the given option modifications applied.
     *
     * The original service is not modified.
     */
    fun withOptions(modifier: Consumer<ClientOptions.Builder>): PartService

    /** Retrieve one part of a widget */
    fun retrieve(partNumber: Long, params: PartRetrieveParams): PartRetrieveResponse =
        retrieve(partNumber, params, RequestOptions.none())

    /** @see retrieve */
    fun retrieve(
        partNumber: Long,
        params: PartRetrieveParams,
        requestOptions: RequestOptions = RequestOptions.none(),
    ): PartRetrieveResponse =
        retrieve(params.toBuilder().partNumber(partNumber).build(), requestOptions)

    /** @see retrieve */
    fun retrieve(params: PartRetrieveParams): PartRetrieveResponse =
        retrieve(params, RequestOptions.none())

    /** @see retrieve */
    fun retrieve(
        params: PartRetrieveParams,
        requestOptions: RequestOptions = RequestOptions.none(),
    ): PartRetrieveResponse

    /** A view of [PartService] that provides access to raw HTTP responses for each method. */
    interface WithRawResponse {

        /**
         * Returns a view of this service with the given option modifications applied.
         *
         * The original service is not modified.
         */
        fun withOptions(modifier: Consumer<ClientOptions.Builder>): PartService.WithRawResponse

        /**
         * Returns a raw HTTP response for `get /v1/widgets/{widget_id}/parts/{part_number}`, but is
         * otherwise the same as [PartService.retrieve].
         */
        @MustBeClosed
        fun retrieve(
            partNumber: Long,
            params: PartRetrieveParams,
        ): HttpResponseFor<PartRetrieveResponse> =
            retrieve(partNumber, params, RequestOptions.none())

        /** @see retrieve */
        @MustBeClosed
        fun retrieve(
            partNumber: Long,
            params: PartRetrieveParams,
            requestOptions: RequestOptions = RequestOptions.none(),
        ): HttpResponseFor<PartRetrieveResponse> =
            retrieve(params.toBuilder().partNumber(partNumber).build(), requestOptions)

        /** @see retrieve */
        @MustBeClosed
        fun retrieve(params: PartRetrieveParams): HttpResponseFor<PartRetrieveResponse> =
            retrieve(params, RequestOptions.none())

        /** @see retrieve */
        @MustBeClosed
        fun retrieve(
            params: PartRetrieveParams,
            requestOptions: RequestOptions = RequestOptions.none(),
        ): HttpResponseFor<PartRetrieveResponse>
    }
}
