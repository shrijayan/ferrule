// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.client

import java.util.function.Consumer
import test.acme.api.core.ClientOptions
import test.acme.api.core.getPackageVersion
import test.acme.api.services.async.WidgetServiceAsync
import test.acme.api.services.async.WidgetServiceAsyncImpl

class AcmeClientAsyncImpl(private val clientOptions: ClientOptions) : AcmeClientAsync {

    private val clientOptionsWithUserAgent =
        if (clientOptions.headers.names().contains("User-Agent")) clientOptions
        else
            clientOptions
                .toBuilder()
                .putHeader("User-Agent", "${javaClass.simpleName}/Java ${getPackageVersion()}")
                .build()

    // Pass the original clientOptions so that this client sets its own User-Agent.
    private val sync: AcmeClient by lazy { AcmeClientImpl(clientOptions) }

    private val withRawResponse: AcmeClientAsync.WithRawResponse by lazy {
        WithRawResponseImpl(clientOptions)
    }

    private val widgets: WidgetServiceAsync by lazy {
        WidgetServiceAsyncImpl(clientOptionsWithUserAgent)
    }

    override fun sync(): AcmeClient = sync

    override fun withRawResponse(): AcmeClientAsync.WithRawResponse = withRawResponse

    override fun withOptions(modifier: Consumer<ClientOptions.Builder>): AcmeClientAsync =
        AcmeClientAsyncImpl(clientOptions.toBuilder().apply(modifier::accept).build())

    override fun widgets(): WidgetServiceAsync = widgets

    override fun close() = clientOptions.close()

    class WithRawResponseImpl internal constructor(private val clientOptions: ClientOptions) :
        AcmeClientAsync.WithRawResponse {

        private val widgets: WidgetServiceAsync.WithRawResponse by lazy {
            WidgetServiceAsyncImpl.WithRawResponseImpl(clientOptions)
        }

        override fun withOptions(
            modifier: Consumer<ClientOptions.Builder>
        ): AcmeClientAsync.WithRawResponse =
            AcmeClientAsyncImpl.WithRawResponseImpl(
                clientOptions.toBuilder().apply(modifier::accept).build()
            )

        override fun widgets(): WidgetServiceAsync.WithRawResponse = widgets
    }
}
