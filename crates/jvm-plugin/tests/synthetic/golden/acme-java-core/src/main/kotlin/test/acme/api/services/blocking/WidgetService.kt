// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services.blocking

import com.google.errorprone.annotations.MustBeClosed
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
import test.acme.api.services.blocking.widgets.PartService

interface WidgetService {

    /**
     * Returns a view of this service that provides access to raw HTTP responses for each method.
     */
    fun withRawResponse(): WithRawResponse

    /**
     * Returns a view of this service with the given option modifications applied.
     *
     * The original service is not modified.
     */
    fun withOptions(modifier: Consumer<ClientOptions.Builder>): WidgetService

    fun parts(): PartService

    /** Creates a widget with the given name and parts. The SKU is assigned by the server. */
    fun create(params: WidgetCreateParams): Widget = create(params, RequestOptions.none())

    /** @see create */
    fun create(
        params: WidgetCreateParams,
        requestOptions: RequestOptions = RequestOptions.none(),
    ): Widget

    /** Retrieve a widget */
    fun retrieve(widgetId: String): Widget = retrieve(widgetId, WidgetRetrieveParams.none())

    /** @see retrieve */
    fun retrieve(
        widgetId: String,
        params: WidgetRetrieveParams = WidgetRetrieveParams.none(),
        requestOptions: RequestOptions = RequestOptions.none(),
    ): Widget = retrieve(params.toBuilder().widgetId(widgetId).build(), requestOptions)

    /** @see retrieve */
    fun retrieve(
        widgetId: String,
        params: WidgetRetrieveParams = WidgetRetrieveParams.none(),
    ): Widget = retrieve(widgetId, params, RequestOptions.none())

    /** @see retrieve */
    fun retrieve(
        params: WidgetRetrieveParams,
        requestOptions: RequestOptions = RequestOptions.none(),
    ): Widget

    /** @see retrieve */
    fun retrieve(params: WidgetRetrieveParams): Widget = retrieve(params, RequestOptions.none())

    /** @see retrieve */
    fun retrieve(widgetId: String, requestOptions: RequestOptions): Widget =
        retrieve(widgetId, WidgetRetrieveParams.none(), requestOptions)

    /** List widgets */
    fun list(): WidgetListResponse = list(WidgetListParams.none())

    /** @see list */
    fun list(
        params: WidgetListParams = WidgetListParams.none(),
        requestOptions: RequestOptions = RequestOptions.none(),
    ): WidgetListResponse

    /** @see list */
    fun list(params: WidgetListParams = WidgetListParams.none()): WidgetListResponse =
        list(params, RequestOptions.none())

    /** @see list */
    fun list(requestOptions: RequestOptions): WidgetListResponse =
        list(WidgetListParams.none(), requestOptions)

    /** Delete a widget */
    fun delete(widgetId: String) = delete(widgetId, WidgetDeleteParams.none())

    /** @see delete */
    fun delete(
        widgetId: String,
        params: WidgetDeleteParams = WidgetDeleteParams.none(),
        requestOptions: RequestOptions = RequestOptions.none(),
    ) = delete(params.toBuilder().widgetId(widgetId).build(), requestOptions)

    /** @see delete */
    fun delete(widgetId: String, params: WidgetDeleteParams = WidgetDeleteParams.none()) =
        delete(widgetId, params, RequestOptions.none())

    /** @see delete */
    fun delete(params: WidgetDeleteParams, requestOptions: RequestOptions = RequestOptions.none())

    /** @see delete */
    fun delete(params: WidgetDeleteParams) = delete(params, RequestOptions.none())

    /** @see delete */
    fun delete(widgetId: String, requestOptions: RequestOptions) =
        delete(widgetId, WidgetDeleteParams.none(), requestOptions)

    /** A view of [WidgetService] that provides access to raw HTTP responses for each method. */
    interface WithRawResponse {

        /**
         * Returns a view of this service with the given option modifications applied.
         *
         * The original service is not modified.
         */
        fun withOptions(modifier: Consumer<ClientOptions.Builder>): WidgetService.WithRawResponse

        fun parts(): PartService.WithRawResponse

        /**
         * Returns a raw HTTP response for `post /v1/widgets`, but is otherwise the same as
         * [WidgetService.create].
         */
        @MustBeClosed
        fun create(params: WidgetCreateParams): HttpResponseFor<Widget> =
            create(params, RequestOptions.none())

        /** @see create */
        @MustBeClosed
        fun create(
            params: WidgetCreateParams,
            requestOptions: RequestOptions = RequestOptions.none(),
        ): HttpResponseFor<Widget>

        /**
         * Returns a raw HTTP response for `get /v1/widgets/{widget_id}`, but is otherwise the same
         * as [WidgetService.retrieve].
         */
        @MustBeClosed
        fun retrieve(widgetId: String): HttpResponseFor<Widget> =
            retrieve(widgetId, WidgetRetrieveParams.none())

        /** @see retrieve */
        @MustBeClosed
        fun retrieve(
            widgetId: String,
            params: WidgetRetrieveParams = WidgetRetrieveParams.none(),
            requestOptions: RequestOptions = RequestOptions.none(),
        ): HttpResponseFor<Widget> =
            retrieve(params.toBuilder().widgetId(widgetId).build(), requestOptions)

        /** @see retrieve */
        @MustBeClosed
        fun retrieve(
            widgetId: String,
            params: WidgetRetrieveParams = WidgetRetrieveParams.none(),
        ): HttpResponseFor<Widget> = retrieve(widgetId, params, RequestOptions.none())

        /** @see retrieve */
        @MustBeClosed
        fun retrieve(
            params: WidgetRetrieveParams,
            requestOptions: RequestOptions = RequestOptions.none(),
        ): HttpResponseFor<Widget>

        /** @see retrieve */
        @MustBeClosed
        fun retrieve(params: WidgetRetrieveParams): HttpResponseFor<Widget> =
            retrieve(params, RequestOptions.none())

        /** @see retrieve */
        @MustBeClosed
        fun retrieve(widgetId: String, requestOptions: RequestOptions): HttpResponseFor<Widget> =
            retrieve(widgetId, WidgetRetrieveParams.none(), requestOptions)

        /**
         * Returns a raw HTTP response for `get /v1/widgets`, but is otherwise the same as
         * [WidgetService.list].
         */
        @MustBeClosed
        fun list(): HttpResponseFor<WidgetListResponse> = list(WidgetListParams.none())

        /** @see list */
        @MustBeClosed
        fun list(
            params: WidgetListParams = WidgetListParams.none(),
            requestOptions: RequestOptions = RequestOptions.none(),
        ): HttpResponseFor<WidgetListResponse>

        /** @see list */
        @MustBeClosed
        fun list(
            params: WidgetListParams = WidgetListParams.none()
        ): HttpResponseFor<WidgetListResponse> = list(params, RequestOptions.none())

        /** @see list */
        @MustBeClosed
        fun list(requestOptions: RequestOptions): HttpResponseFor<WidgetListResponse> =
            list(WidgetListParams.none(), requestOptions)

        /**
         * Returns a raw HTTP response for `delete /v1/widgets/{widget_id}`, but is otherwise the
         * same as [WidgetService.delete].
         */
        @MustBeClosed
        fun delete(widgetId: String): HttpResponse = delete(widgetId, WidgetDeleteParams.none())

        /** @see delete */
        @MustBeClosed
        fun delete(
            widgetId: String,
            params: WidgetDeleteParams = WidgetDeleteParams.none(),
            requestOptions: RequestOptions = RequestOptions.none(),
        ): HttpResponse = delete(params.toBuilder().widgetId(widgetId).build(), requestOptions)

        /** @see delete */
        @MustBeClosed
        fun delete(
            widgetId: String,
            params: WidgetDeleteParams = WidgetDeleteParams.none(),
        ): HttpResponse = delete(widgetId, params, RequestOptions.none())

        /** @see delete */
        @MustBeClosed
        fun delete(
            params: WidgetDeleteParams,
            requestOptions: RequestOptions = RequestOptions.none(),
        ): HttpResponse

        /** @see delete */
        @MustBeClosed
        fun delete(params: WidgetDeleteParams): HttpResponse = delete(params, RequestOptions.none())

        /** @see delete */
        @MustBeClosed
        fun delete(widgetId: String, requestOptions: RequestOptions): HttpResponse =
            delete(widgetId, WidgetDeleteParams.none(), requestOptions)
    }
}
