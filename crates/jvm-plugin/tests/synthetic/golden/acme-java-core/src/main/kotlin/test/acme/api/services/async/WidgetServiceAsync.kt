// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services.async

import java.util.concurrent.CompletableFuture
import java.util.function.Consumer
import test.acme.api.core.ClientOptions
import test.acme.api.core.RequestOptions
import test.acme.api.core.http.HttpResponse
import test.acme.api.core.http.HttpResponseFor
import test.acme.api.models.widgets.Widget
import test.acme.api.models.widgets.WidgetCreateParams
import test.acme.api.models.widgets.WidgetDeleteParams
import test.acme.api.models.widgets.WidgetListParams
import test.acme.api.models.widgets.WidgetListResponse
import test.acme.api.models.widgets.WidgetRetrieveParams
import test.acme.api.services.async.widgets.PartServiceAsync

interface WidgetServiceAsync {

    /**
     * Returns a view of this service that provides access to raw HTTP responses for each method.
     */
    fun withRawResponse(): WithRawResponse

    /**
     * Returns a view of this service with the given option modifications applied.
     *
     * The original service is not modified.
     */
    fun withOptions(modifier: Consumer<ClientOptions.Builder>): WidgetServiceAsync

    fun parts(): PartServiceAsync

    /** Creates a widget with the given name and parts. The SKU is assigned by the server. */
    fun create(params: WidgetCreateParams): CompletableFuture<Widget> =
        create(params, RequestOptions.none())

    /** @see create */
    fun create(
        params: WidgetCreateParams,
        requestOptions: RequestOptions = RequestOptions.none(),
    ): CompletableFuture<Widget>

    /** Retrieve a widget */
    fun retrieve(widgetId: String): CompletableFuture<Widget> =
        retrieve(widgetId, WidgetRetrieveParams.none())

    /** @see retrieve */
    fun retrieve(
        widgetId: String,
        params: WidgetRetrieveParams = WidgetRetrieveParams.none(),
        requestOptions: RequestOptions = RequestOptions.none(),
    ): CompletableFuture<Widget> =
        retrieve(params.toBuilder().widgetId(widgetId).build(), requestOptions)

    /** @see retrieve */
    fun retrieve(
        widgetId: String,
        params: WidgetRetrieveParams = WidgetRetrieveParams.none(),
    ): CompletableFuture<Widget> = retrieve(widgetId, params, RequestOptions.none())

    /** @see retrieve */
    fun retrieve(
        params: WidgetRetrieveParams,
        requestOptions: RequestOptions = RequestOptions.none(),
    ): CompletableFuture<Widget>

    /** @see retrieve */
    fun retrieve(params: WidgetRetrieveParams): CompletableFuture<Widget> =
        retrieve(params, RequestOptions.none())

    /** @see retrieve */
    fun retrieve(widgetId: String, requestOptions: RequestOptions): CompletableFuture<Widget> =
        retrieve(widgetId, WidgetRetrieveParams.none(), requestOptions)

    /** List widgets */
    fun list(): CompletableFuture<WidgetListResponse> = list(WidgetListParams.none())

    /** @see list */
    fun list(
        params: WidgetListParams = WidgetListParams.none(),
        requestOptions: RequestOptions = RequestOptions.none(),
    ): CompletableFuture<WidgetListResponse>

    /** @see list */
    fun list(
        params: WidgetListParams = WidgetListParams.none()
    ): CompletableFuture<WidgetListResponse> = list(params, RequestOptions.none())

    /** @see list */
    fun list(requestOptions: RequestOptions): CompletableFuture<WidgetListResponse> =
        list(WidgetListParams.none(), requestOptions)

    /** Delete a widget */
    fun delete(widgetId: String): CompletableFuture<Void?> =
        delete(widgetId, WidgetDeleteParams.none())

    /** @see delete */
    fun delete(
        widgetId: String,
        params: WidgetDeleteParams = WidgetDeleteParams.none(),
        requestOptions: RequestOptions = RequestOptions.none(),
    ): CompletableFuture<Void?> =
        delete(params.toBuilder().widgetId(widgetId).build(), requestOptions)

    /** @see delete */
    fun delete(
        widgetId: String,
        params: WidgetDeleteParams = WidgetDeleteParams.none(),
    ): CompletableFuture<Void?> = delete(widgetId, params, RequestOptions.none())

    /** @see delete */
    fun delete(
        params: WidgetDeleteParams,
        requestOptions: RequestOptions = RequestOptions.none(),
    ): CompletableFuture<Void?>

    /** @see delete */
    fun delete(params: WidgetDeleteParams): CompletableFuture<Void?> =
        delete(params, RequestOptions.none())

    /** @see delete */
    fun delete(widgetId: String, requestOptions: RequestOptions): CompletableFuture<Void?> =
        delete(widgetId, WidgetDeleteParams.none(), requestOptions)

    /**
     * A view of [WidgetServiceAsync] that provides access to raw HTTP responses for each method.
     */
    interface WithRawResponse {

        /**
         * Returns a view of this service with the given option modifications applied.
         *
         * The original service is not modified.
         */
        fun withOptions(
            modifier: Consumer<ClientOptions.Builder>
        ): WidgetServiceAsync.WithRawResponse

        fun parts(): PartServiceAsync.WithRawResponse

        /**
         * Returns a raw HTTP response for `post /v1/widgets`, but is otherwise the same as
         * [WidgetServiceAsync.create].
         */
        fun create(params: WidgetCreateParams): CompletableFuture<HttpResponseFor<Widget>> =
            create(params, RequestOptions.none())

        /** @see create */
        fun create(
            params: WidgetCreateParams,
            requestOptions: RequestOptions = RequestOptions.none(),
        ): CompletableFuture<HttpResponseFor<Widget>>

        /**
         * Returns a raw HTTP response for `get /v1/widgets/{widget_id}`, but is otherwise the same
         * as [WidgetServiceAsync.retrieve].
         */
        fun retrieve(widgetId: String): CompletableFuture<HttpResponseFor<Widget>> =
            retrieve(widgetId, WidgetRetrieveParams.none())

        /** @see retrieve */
        fun retrieve(
            widgetId: String,
            params: WidgetRetrieveParams = WidgetRetrieveParams.none(),
            requestOptions: RequestOptions = RequestOptions.none(),
        ): CompletableFuture<HttpResponseFor<Widget>> =
            retrieve(params.toBuilder().widgetId(widgetId).build(), requestOptions)

        /** @see retrieve */
        fun retrieve(
            widgetId: String,
            params: WidgetRetrieveParams = WidgetRetrieveParams.none(),
        ): CompletableFuture<HttpResponseFor<Widget>> =
            retrieve(widgetId, params, RequestOptions.none())

        /** @see retrieve */
        fun retrieve(
            params: WidgetRetrieveParams,
            requestOptions: RequestOptions = RequestOptions.none(),
        ): CompletableFuture<HttpResponseFor<Widget>>

        /** @see retrieve */
        fun retrieve(params: WidgetRetrieveParams): CompletableFuture<HttpResponseFor<Widget>> =
            retrieve(params, RequestOptions.none())

        /** @see retrieve */
        fun retrieve(
            widgetId: String,
            requestOptions: RequestOptions,
        ): CompletableFuture<HttpResponseFor<Widget>> =
            retrieve(widgetId, WidgetRetrieveParams.none(), requestOptions)

        /**
         * Returns a raw HTTP response for `get /v1/widgets`, but is otherwise the same as
         * [WidgetServiceAsync.list].
         */
        fun list(): CompletableFuture<HttpResponseFor<WidgetListResponse>> =
            list(WidgetListParams.none())

        /** @see list */
        fun list(
            params: WidgetListParams = WidgetListParams.none(),
            requestOptions: RequestOptions = RequestOptions.none(),
        ): CompletableFuture<HttpResponseFor<WidgetListResponse>>

        /** @see list */
        fun list(
            params: WidgetListParams = WidgetListParams.none()
        ): CompletableFuture<HttpResponseFor<WidgetListResponse>> =
            list(params, RequestOptions.none())

        /** @see list */
        fun list(
            requestOptions: RequestOptions
        ): CompletableFuture<HttpResponseFor<WidgetListResponse>> =
            list(WidgetListParams.none(), requestOptions)

        /**
         * Returns a raw HTTP response for `delete /v1/widgets/{widget_id}`, but is otherwise the
         * same as [WidgetServiceAsync.delete].
         */
        fun delete(widgetId: String): CompletableFuture<HttpResponse> =
            delete(widgetId, WidgetDeleteParams.none())

        /** @see delete */
        fun delete(
            widgetId: String,
            params: WidgetDeleteParams = WidgetDeleteParams.none(),
            requestOptions: RequestOptions = RequestOptions.none(),
        ): CompletableFuture<HttpResponse> =
            delete(params.toBuilder().widgetId(widgetId).build(), requestOptions)

        /** @see delete */
        fun delete(
            widgetId: String,
            params: WidgetDeleteParams = WidgetDeleteParams.none(),
        ): CompletableFuture<HttpResponse> = delete(widgetId, params, RequestOptions.none())

        /** @see delete */
        fun delete(
            params: WidgetDeleteParams,
            requestOptions: RequestOptions = RequestOptions.none(),
        ): CompletableFuture<HttpResponse>

        /** @see delete */
        fun delete(params: WidgetDeleteParams): CompletableFuture<HttpResponse> =
            delete(params, RequestOptions.none())

        /** @see delete */
        fun delete(
            widgetId: String,
            requestOptions: RequestOptions,
        ): CompletableFuture<HttpResponse> =
            delete(widgetId, WidgetDeleteParams.none(), requestOptions)
    }
}
