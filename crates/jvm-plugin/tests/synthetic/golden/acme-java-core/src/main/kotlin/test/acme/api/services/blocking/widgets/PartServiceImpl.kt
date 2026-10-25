// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services.blocking.widgets

import java.util.function.Consumer
import kotlin.jvm.optionals.getOrNull
import test.acme.api.core.ClientOptions
import test.acme.api.core.RequestOptions
import test.acme.api.core.checkRequired
import test.acme.api.core.handlers.errorBodyHandler
import test.acme.api.core.handlers.errorHandler
import test.acme.api.core.handlers.jsonHandler
import test.acme.api.core.http.HttpMethod
import test.acme.api.core.http.HttpRequest
import test.acme.api.core.http.HttpResponse
import test.acme.api.core.http.HttpResponse.Handler
import test.acme.api.core.http.HttpResponseFor
import test.acme.api.core.http.parseable
import test.acme.api.core.prepare
import test.acme.api.models.widgets.parts.PartRetrieveParams
import test.acme.api.models.widgets.parts.PartRetrieveResponse

class PartServiceImpl internal constructor(private val clientOptions: ClientOptions) : PartService {

    private val withRawResponse: PartService.WithRawResponse by lazy {
        WithRawResponseImpl(clientOptions)
    }

    override fun withRawResponse(): PartService.WithRawResponse = withRawResponse

    override fun withOptions(modifier: Consumer<ClientOptions.Builder>): PartService =
        PartServiceImpl(clientOptions.toBuilder().apply(modifier::accept).build())

    override fun retrieve(
        params: PartRetrieveParams,
        requestOptions: RequestOptions,
    ): PartRetrieveResponse =
        // get /v1/widgets/{widget_id}/parts/{part_number}
        withRawResponse().retrieve(params, requestOptions).parse()

    class WithRawResponseImpl internal constructor(private val clientOptions: ClientOptions) :
        PartService.WithRawResponse {

        private val errorHandler: Handler<HttpResponse> =
            errorHandler(errorBodyHandler(clientOptions.jsonMapper))

        override fun withOptions(
            modifier: Consumer<ClientOptions.Builder>
        ): PartService.WithRawResponse =
            PartServiceImpl.WithRawResponseImpl(
                clientOptions.toBuilder().apply(modifier::accept).build()
            )

        private val retrieveHandler: Handler<PartRetrieveResponse> =
            jsonHandler<PartRetrieveResponse>(clientOptions.jsonMapper)

        override fun retrieve(
            params: PartRetrieveParams,
            requestOptions: RequestOptions,
        ): HttpResponseFor<PartRetrieveResponse> {
            // We check here instead of in the params builder because this can be specified
            // positionally or in the params class.
            checkRequired("partNumber", params.partNumber().getOrNull())
            val request =
                HttpRequest.builder()
                    .method(HttpMethod.GET)
                    .baseUrl(clientOptions.baseUrl())
                    .addPathSegments(
                        "v1",
                        "widgets",
                        params._pathParam(0),
                        "parts",
                        params._pathParam(1),
                    )
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
    }
}
