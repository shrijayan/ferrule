// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services.async

import java.util.concurrent.CompletableFuture
import java.util.function.Consumer
import kotlin.jvm.optionals.getOrNull
import test.acme.api.core.ClientOptions
import test.acme.api.core.RequestOptions
import test.acme.api.core.checkRequired
import test.acme.api.core.handlers.emptyHandler
import test.acme.api.core.handlers.errorBodyHandler
import test.acme.api.core.handlers.errorHandler
import test.acme.api.core.handlers.jsonHandler
import test.acme.api.core.http.HttpMethod
import test.acme.api.core.http.HttpRequest
import test.acme.api.core.http.HttpResponse
import test.acme.api.core.http.HttpResponse.Handler
import test.acme.api.core.http.HttpResponseFor
import test.acme.api.core.http.json
import test.acme.api.core.http.parseable
import test.acme.api.core.prepareAsync
import test.acme.api.models.widgets.Widget
import test.acme.api.models.widgets.WidgetCreateParams
import test.acme.api.models.widgets.WidgetDeleteParams
import test.acme.api.models.widgets.WidgetListParams
import test.acme.api.models.widgets.WidgetListResponse
import test.acme.api.models.widgets.WidgetRetrieveParams
import test.acme.api.services.async.widgets.PartServiceAsync
import test.acme.api.services.async.widgets.PartServiceAsyncImpl

class WidgetServiceAsyncImpl internal constructor(private val clientOptions: ClientOptions) :
    WidgetServiceAsync {

    private val withRawResponse: WidgetServiceAsync.WithRawResponse by lazy {
        WithRawResponseImpl(clientOptions)
    }

    private val parts: PartServiceAsync by lazy { PartServiceAsyncImpl(clientOptions) }

    override fun withRawResponse(): WidgetServiceAsync.WithRawResponse = withRawResponse

    override fun withOptions(modifier: Consumer<ClientOptions.Builder>): WidgetServiceAsync =
        WidgetServiceAsyncImpl(clientOptions.toBuilder().apply(modifier::accept).build())

    override fun parts(): PartServiceAsync = parts

    override fun create(
        params: WidgetCreateParams,
        requestOptions: RequestOptions,
    ): CompletableFuture<Widget> =
        // post /v1/widgets
        withRawResponse().create(params, requestOptions).thenApply { it.parse() }

    override fun retrieve(
        params: WidgetRetrieveParams,
        requestOptions: RequestOptions,
    ): CompletableFuture<Widget> =
        // get /v1/widgets/{widget_id}
        withRawResponse().retrieve(params, requestOptions).thenApply { it.parse() }

    override fun list(
        params: WidgetListParams,
        requestOptions: RequestOptions,
    ): CompletableFuture<WidgetListResponse> =
        // get /v1/widgets
        withRawResponse().list(params, requestOptions).thenApply { it.parse() }

    override fun delete(
        params: WidgetDeleteParams,
        requestOptions: RequestOptions,
    ): CompletableFuture<Void?> =
        // delete /v1/widgets/{widget_id}
        withRawResponse().delete(params, requestOptions).thenAccept {}

    class WithRawResponseImpl internal constructor(private val clientOptions: ClientOptions) :
        WidgetServiceAsync.WithRawResponse {

        private val errorHandler: Handler<HttpResponse> =
            errorHandler(errorBodyHandler(clientOptions.jsonMapper))

        private val parts: PartServiceAsync.WithRawResponse by lazy {
            PartServiceAsyncImpl.WithRawResponseImpl(clientOptions)
        }

        override fun withOptions(
            modifier: Consumer<ClientOptions.Builder>
        ): WidgetServiceAsync.WithRawResponse =
            WidgetServiceAsyncImpl.WithRawResponseImpl(
                clientOptions.toBuilder().apply(modifier::accept).build()
            )

        override fun parts(): PartServiceAsync.WithRawResponse = parts

        private val createHandler: Handler<Widget> = jsonHandler<Widget>(clientOptions.jsonMapper)

        override fun create(
            params: WidgetCreateParams,
            requestOptions: RequestOptions,
        ): CompletableFuture<HttpResponseFor<Widget>> {
            val request =
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(clientOptions.baseUrl())
                    .addPathSegments("v1", "widgets")
                    .body(json(clientOptions.jsonMapper, params._body()))
                    .build()
                    .prepareAsync(clientOptions, params)
            val requestOptions = requestOptions.applyDefaults(RequestOptions.from(clientOptions))
            return request
                .thenComposeAsync { clientOptions.httpClient.executeAsync(it, requestOptions) }
                .thenApply { response ->
                    errorHandler.handle(response).parseable {
                        response
                            .use { createHandler.handle(it) }
                            .also {
                                if (requestOptions.responseValidation!!) {
                                    it.validate()
                                }
                            }
                    }
                }
        }

        private val retrieveHandler: Handler<Widget> = jsonHandler<Widget>(clientOptions.jsonMapper)

        override fun retrieve(
            params: WidgetRetrieveParams,
            requestOptions: RequestOptions,
        ): CompletableFuture<HttpResponseFor<Widget>> {
            // We check here instead of in the params builder because this can be specified
            // positionally or in the params class.
            checkRequired("widgetId", params.widgetId().getOrNull())
            val request =
                HttpRequest.builder()
                    .method(HttpMethod.GET)
                    .baseUrl(clientOptions.baseUrl())
                    .addPathSegments("v1", "widgets", params._pathParam(0))
                    .build()
                    .prepareAsync(clientOptions, params)
            val requestOptions = requestOptions.applyDefaults(RequestOptions.from(clientOptions))
            return request
                .thenComposeAsync { clientOptions.httpClient.executeAsync(it, requestOptions) }
                .thenApply { response ->
                    errorHandler.handle(response).parseable {
                        response
                            .use { retrieveHandler.handle(it) }
                            .also {
                                if (requestOptions.responseValidation!!) {
                                    it.validate()
                                }
                            }
                    }
                }
        }

        private val listHandler: Handler<WidgetListResponse> =
            jsonHandler<WidgetListResponse>(clientOptions.jsonMapper)

        override fun list(
            params: WidgetListParams,
            requestOptions: RequestOptions,
        ): CompletableFuture<HttpResponseFor<WidgetListResponse>> {
            val request =
                HttpRequest.builder()
                    .method(HttpMethod.GET)
                    .baseUrl(clientOptions.baseUrl())
                    .addPathSegments("v1", "widgets")
                    .build()
                    .prepareAsync(clientOptions, params)
            val requestOptions = requestOptions.applyDefaults(RequestOptions.from(clientOptions))
            return request
                .thenComposeAsync { clientOptions.httpClient.executeAsync(it, requestOptions) }
                .thenApply { response ->
                    errorHandler.handle(response).parseable {
                        response
                            .use { listHandler.handle(it) }
                            .also {
                                if (requestOptions.responseValidation!!) {
                                    it.validate()
                                }
                            }
                    }
                }
        }

        private val deleteHandler: Handler<Void?> = emptyHandler()

        override fun delete(
            params: WidgetDeleteParams,
            requestOptions: RequestOptions,
        ): CompletableFuture<HttpResponse> {
            // We check here instead of in the params builder because this can be specified
            // positionally or in the params class.
            checkRequired("widgetId", params.widgetId().getOrNull())
            val request =
                HttpRequest.builder()
                    .method(HttpMethod.DELETE)
                    .baseUrl(clientOptions.baseUrl())
                    .addPathSegments("v1", "widgets", params._pathParam(0))
                    .apply { params._body().ifPresent { body(json(clientOptions.jsonMapper, it)) } }
                    .build()
                    .prepareAsync(clientOptions, params)
            val requestOptions = requestOptions.applyDefaults(RequestOptions.from(clientOptions))
            return request
                .thenComposeAsync { clientOptions.httpClient.executeAsync(it, requestOptions) }
                .thenApply { response ->
                    errorHandler.handle(response).parseable {
                        response.use { deleteHandler.handle(it) }
                    }
                }
        }
    }
}
