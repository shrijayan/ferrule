// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services.async.widgets

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.extension.ExtendWith
import test.acme.api.TestServerExtension
import test.acme.api.client.okhttp.AcmeOkHttpClientAsync
import test.acme.api.models.widgets.parts.PartRetrieveParams

@ExtendWith(TestServerExtension::class)
internal class PartServiceAsyncTest {

    @Test
    fun retrieve() {
        val client =
            AcmeOkHttpClientAsync.builder()
                .baseUrl(TestServerExtension.BASE_URL)
                .apiKey("My API Key")
                .build()
        val partServiceAsync = client.widgets().parts()

        val partFuture =
            partServiceAsync.retrieve(
                PartRetrieveParams.builder().widgetId("widget_id").partNumber(0L).build()
            )

        val part = partFuture.get()
        part.validate()
    }
}
