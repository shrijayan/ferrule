// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services.blocking.widgets

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.extension.ExtendWith
import test.acme.api.TestServerExtension
import test.acme.api.client.okhttp.AcmeOkHttpClient
import test.acme.api.models.widgets.parts.PartRetrieveParams

@ExtendWith(TestServerExtension::class)
internal class PartServiceTest {

    @Test
    fun retrieve() {
        val client =
            AcmeOkHttpClient.builder()
                .baseUrl(TestServerExtension.BASE_URL)
                .apiKey("My API Key")
                .build()
        val partService = client.widgets().parts()

        val part =
            partService.retrieve(
                PartRetrieveParams.builder().widgetId("widget_id").partNumber(0L).build()
            )

        part.validate()
    }
}
