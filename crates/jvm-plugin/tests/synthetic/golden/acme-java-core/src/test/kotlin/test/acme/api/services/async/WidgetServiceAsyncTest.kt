// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services.async

import java.time.OffsetDateTime
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.extension.ExtendWith
import test.acme.api.TestServerExtension
import test.acme.api.client.okhttp.AcmeOkHttpClientAsync
import test.acme.api.models.widgets.WidgetCreateParams
import test.acme.api.models.widgets.WidgetListParams

@ExtendWith(TestServerExtension::class)
internal class WidgetServiceAsyncTest {

    @Test
    fun create() {
        val client =
            AcmeOkHttpClientAsync.builder()
                .baseUrl(TestServerExtension.BASE_URL)
                .apiKey("My API Key")
                .build()
        val widgetServiceAsync = client.widgets()

        val widgetFuture =
            widgetServiceAsync.create(
                WidgetCreateParams.builder()
                    .name("name")
                    .color(WidgetCreateParams.Color.RED)
                    .dimensions(
                        WidgetCreateParams.Dimensions.builder().width(0.0).height(0.0).build()
                    )
                    .addTag("string")
                    .build()
            )

        val widget = widgetFuture.get()
        widget.validate()
    }

    @Test
    fun retrieve() {
        val client =
            AcmeOkHttpClientAsync.builder()
                .baseUrl(TestServerExtension.BASE_URL)
                .apiKey("My API Key")
                .build()
        val widgetServiceAsync = client.widgets()

        val widgetFuture = widgetServiceAsync.retrieve("widget_id")

        val widget = widgetFuture.get()
        widget.validate()
    }

    @Test
    fun list() {
        val client =
            AcmeOkHttpClientAsync.builder()
                .baseUrl(TestServerExtension.BASE_URL)
                .apiKey("My API Key")
                .build()
        val widgetServiceAsync = client.widgets()

        val widgetsFuture =
            widgetServiceAsync.list(
                WidgetListParams.builder()
                    .colors("string")
                    .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                    .limit(0L)
                    .build()
            )

        val widgets = widgetsFuture.get()
        widgets.validate()
    }

    @Test
    fun delete() {
        val client =
            AcmeOkHttpClientAsync.builder()
                .baseUrl(TestServerExtension.BASE_URL)
                .apiKey("My API Key")
                .build()
        val widgetServiceAsync = client.widgets()

        val future = widgetServiceAsync.delete("widget_id")

        val response = future.get()
    }
}
