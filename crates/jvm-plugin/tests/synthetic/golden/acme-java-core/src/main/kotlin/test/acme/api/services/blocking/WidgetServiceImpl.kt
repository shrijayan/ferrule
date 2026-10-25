// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services.blocking

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
import test.acme.api.core.prepare
import test.acme.api.models.widgets.Widget
import test.acme.api.models.widgets.WidgetCreateParams
import test.acme.api.models.widgets.WidgetDeleteParams
import test.acme.api.models.widgets.WidgetListParams
import test.acme.api.models.widgets.WidgetListResponse
import test.acme.api.models.widgets.WidgetRetrieveParams
import test.acme.api.services.blocking.widgets.PartService
import test.acme.api.services.blocking.widgets.PartServiceImpl

class WidgetServiceImpl internal constructor(private val clientOptions: ClientOptions) :
    WidgetService {

    private val withRawResponse: WidgetService.WithRawResponse by lazy {
        WithRawResponseImpl(clientOptions)
    }

    private val parts: PartService by lazy { PartServiceImpl(clientOptions) }

    override fun withRawResponse(): WidgetService.WithRawResponse = withRawResponse

    override fun withOptions(modifier: Consumer<ClientOptions.Builder>): WidgetService =
        WidgetServiceImpl(clientOptions.toBuilder().apply(modifier::accept).build())

    override fun parts(): PartService = parts

    override fun create(params: WidgetCreateParams, requestOptions: RequestOptions): Widget =
        // post /v1/widgets
        withRawResponse().create(params, requestOptions).parse()

    override fun retrieve(params: WidgetRetrieveParams, requestOptions: RequestOptions): Widget =
        // get /v1/widgets/{widget_id}
        withRawResponse().retrieve(params, requestOptions).parse()

    override fun list(
        params: WidgetListParams,
        requestOptions: RequestOptions,
    ): WidgetListResponse =
        // get /v1/widgets
        withRawResponse().list(params, requestOptions).parse()

    override fun delete(params: WidgetDeleteParams, requestOptions: RequestOptions) {
        // delete /v1/widgets/{widget_id}
        withRawResponse().delete(params, requestOptions)
    }

    class WithRawResponseImpl internal constructor(private val clientOptions: ClientOptions) :
        WidgetService.WithRawResponse {

        private val errorHandler: Handler<HttpResponse> =
            errorHandler(errorBodyHandler(clientOptions.jsonMapper))

        private val parts: PartService.WithRawResponse by lazy {
            PartServiceImpl.WithRawResponseImpl(clientOptions)
        }

        override fun withOptions(
            modifier: Consumer<ClientOptions.Builder>
        ): WidgetService.WithRawResponse =
            WidgetServiceImpl.WithRawResponseImpl(
                clientOptions.toBuilder().apply(modifier::accept).build()
            )

        override fun parts(): PartService.WithRawResponse = parts

        private val createHandler: Handler<Widget> = jsonHandler<Widget>(clientOptions.jsonMapper)

        override fun create(
            params: WidgetCreateParams,
            requestOptions: RequestOptions,
        ): HttpResponseFor<Widget> {
            val request =
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(clientOptions.baseUrl())
                    .addPathSegments("v1", "widgets")
                    .body(json(clientOptions.jsonMapper, params._body()))
                    .build()
                    .prepare(clientOptions, params)
            val requestOptions = requestOptions.applyDefaults(RequestOptions.from(clientOptions))
            val response = clientOptions.httpClient.execute(request, requestOptions)
            return errorHandler.handle(response).parseable {
                response
                    .use { createHandler.handle(it) }
                    .also {
                        if (requestOptions.responseValidation!!) {
                            it.validate()
                        }
                    }
            }
        }

        private val retrieveHandler: Handler<Widget> = jsonHandler<Widget>(clientOptions.jsonMapper)

        override fun retrieve(
            params: WidgetRetrieveParams,
            requestOptions: RequestOptions,
        ): HttpResponseFor<Widget> {
            // We check here instead of in the params builder because this can be specified
            // positionally or in the params class.
            checkRequired("widgetId", params.widgetId().getOrNull())
            val request =
                HttpRequest.builder()
                    .method(HttpMethod.GET)
                    .baseUrl(clientOptions.baseUrl())
                    .addPathSegments("v1", "widgets", params._pathParam(0))
                    .build()
                    .prepare(clientOptions, params)
            val requestOptions = requestOptions.applyDefaults(RequestOptions.from(clientOptions))
            val response = clientOptions.httpClient.execute(request, requestOptions)
            return errorHandler.handle(response).parseable {
                response
                    .use { retrieveHandler.handle(it) }
                    .also {
                        if (requestOptions.responseValidation!!) {
                            it.validate()
                        }
                    }
            }
        }

        private val listHandler: Handler<WidgetListResponse> =
            jsonHandler<WidgetListResponse>(clientOptions.jsonMapper)

        override fun list(
            params: WidgetListParams,
            requestOptions: RequestOptions,
        ): HttpResponseFor<WidgetListResponse> {
            val request =
                HttpRequest.builder()
                    .method(HttpMethod.GET)
                    .baseUrl(clientOptions.baseUrl())
                    .addPathSegments("v1", "widgets")
                    .build()
                    .prepare(clientOptions, params)
            val requestOptions = requestOptions.applyDefaults(RequestOptions.from(clientOptions))
            val response = clientOptions.httpClient.execute(request, requestOptions)
            return errorHandler.handle(response).parseable {
                response
                    .use { listHandler.handle(it) }
                    .also {
                        if (requestOptions.responseValidation!!) {
                            it.validate()
                        }
                    }
            }
        }

        private val deleteHandler: Handler<Void?> = emptyHandler()

        override fun delete(
            params: WidgetDeleteParams,
            requestOptions: RequestOptions,
        ): HttpResponse {
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
                    .prepare(clientOptions, params)
            val requestOptions = requestOptions.applyDefaults(RequestOptions.from(clientOptions))
            val response = clientOptions.httpClient.execute(request, requestOptions)
            return errorHandler.handle(response).parseable {
                response.use { deleteHandler.handle(it) }
            }
        }
    }
}
