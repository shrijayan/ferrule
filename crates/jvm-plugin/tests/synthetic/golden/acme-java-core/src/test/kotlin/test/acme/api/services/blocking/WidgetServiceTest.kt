// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services.blocking

import java.time.OffsetDateTime
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.extension.ExtendWith
import test.acme.api.TestServerExtension
import test.acme.api.client.okhttp.AcmeOkHttpClient
import test.acme.api.models.widgets.WidgetCreateParams
import test.acme.api.models.widgets.WidgetListParams

@ExtendWith(TestServerExtension::class)
internal class WidgetServiceTest {

    @Test
    fun create() {
        val client =
            AcmeOkHttpClient.builder()
                .baseUrl(TestServerExtension.BASE_URL)
                .apiKey("My API Key")
                .build()
        val widgetService = client.widgets()

        val widget =
            widgetService.create(
                WidgetCreateParams.builder()
                    .name("name")
                    .color(WidgetCreateParams.Color.RED)
                    .dimensions(
                        WidgetCreateParams.Dimensions.builder().width(0.0).height(0.0).build()
                    )
                    .addTag("string")
                    .build()
            )

        widget.validate()
    }

    @Test
    fun retrieve() {
        val client =
            AcmeOkHttpClient.builder()
                .baseUrl(TestServerExtension.BASE_URL)
                .apiKey("My API Key")
                .build()
        val widgetService = client.widgets()

        val widget = widgetService.retrieve("widget_id")

        widget.validate()
    }

    @Test
    fun list() {
        val client =
            AcmeOkHttpClient.builder()
                .baseUrl(TestServerExtension.BASE_URL)
                .apiKey("My API Key")
                .build()
        val widgetService = client.widgets()

        val widgets =
            widgetService.list(
                WidgetListParams.builder()
                    .colors("string")
                    .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                    .limit(0L)
                    .build()
            )

        widgets.validate()
    }

    @Test
    fun delete() {
        val client =
            AcmeOkHttpClient.builder()
                .baseUrl(TestServerExtension.BASE_URL)
                .apiKey("My API Key")
                .build()
        val widgetService = client.widgets()

        widgetService.delete("widget_id")
    }
}
