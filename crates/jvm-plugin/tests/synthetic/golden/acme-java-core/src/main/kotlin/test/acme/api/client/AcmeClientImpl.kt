// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.client

import java.util.function.Consumer
import test.acme.api.core.ClientOptions
import test.acme.api.core.getPackageVersion
import test.acme.api.services.blocking.WidgetService
import test.acme.api.services.blocking.WidgetServiceImpl

class AcmeClientImpl(private val clientOptions: ClientOptions) : AcmeClient {

    private val clientOptionsWithUserAgent =
        if (clientOptions.headers.names().contains("User-Agent")) clientOptions
        else
            clientOptions
                .toBuilder()
                .putHeader("User-Agent", "${javaClass.simpleName}/Java ${getPackageVersion()}")
                .build()

    // Pass the original clientOptions so that this client sets its own User-Agent.
    private val async: AcmeClientAsync by lazy { AcmeClientAsyncImpl(clientOptions) }

    private val withRawResponse: AcmeClient.WithRawResponse by lazy {
        WithRawResponseImpl(clientOptions)
    }

    private val widgets: WidgetService by lazy { WidgetServiceImpl(clientOptionsWithUserAgent) }

    override fun async(): AcmeClientAsync = async

    override fun withRawResponse(): AcmeClient.WithRawResponse = withRawResponse

    override fun withOptions(modifier: Consumer<ClientOptions.Builder>): AcmeClient =
        AcmeClientImpl(clientOptions.toBuilder().apply(modifier::accept).build())

    override fun widgets(): WidgetService = widgets

    override fun close() = clientOptions.close()

    class WithRawResponseImpl internal constructor(private val clientOptions: ClientOptions) :
        AcmeClient.WithRawResponse {

        private val widgets: WidgetService.WithRawResponse by lazy {
            WidgetServiceImpl.WithRawResponseImpl(clientOptions)
        }

        override fun withOptions(
            modifier: Consumer<ClientOptions.Builder>
        ): AcmeClient.WithRawResponse =
            AcmeClientImpl.WithRawResponseImpl(
                clientOptions.toBuilder().apply(modifier::accept).build()
            )

        override fun widgets(): WidgetService.WithRawResponse = widgets
    }
}
