// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models.widgets

import com.fasterxml.jackson.module.kotlin.jacksonTypeRef
import java.time.OffsetDateTime
import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.Test
import test.acme.api.core.jsonMapper

internal class WidgetListResponseTest {

    @Test
    fun create() {
        val widgetListResponse =
            WidgetListResponse.builder()
                .addData(
                    Widget.builder()
                        .id("id")
                        .createdAt(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .name("name")
                        .status(Widget.Status.DRAFT)
                        .homepageUrl("homepage_url")
                        .addPart(Widget.Part.builder().label("label").partNumber(0L).build())
                        .sku("sku")
                        .weightKg(0.0)
                        .build()
                )
                .nextCursor("next_cursor")
                .build()

        assertThat(widgetListResponse.data())
            .containsExactly(
                Widget.builder()
                    .id("id")
                    .createdAt(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                    .name("name")
                    .status(Widget.Status.DRAFT)
                    .homepageUrl("homepage_url")
                    .addPart(Widget.Part.builder().label("label").partNumber(0L).build())
                    .sku("sku")
                    .weightKg(0.0)
                    .build()
            )
        assertThat(widgetListResponse.nextCursor()).contains("next_cursor")
    }

    @Test
    fun roundtrip() {
        val jsonMapper = jsonMapper()
        val widgetListResponse =
            WidgetListResponse.builder()
                .addData(
                    Widget.builder()
                        .id("id")
                        .createdAt(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .name("name")
                        .status(Widget.Status.DRAFT)
                        .homepageUrl("homepage_url")
                        .addPart(Widget.Part.builder().label("label").partNumber(0L).build())
                        .sku("sku")
                        .weightKg(0.0)
                        .build()
                )
                .nextCursor("next_cursor")
                .build()

        val roundtrippedWidgetListResponse =
            jsonMapper.readValue(
                jsonMapper.writeValueAsString(widgetListResponse),
                jacksonTypeRef<WidgetListResponse>(),
            )

        assertThat(roundtrippedWidgetListResponse).isEqualTo(widgetListResponse)
    }
}
