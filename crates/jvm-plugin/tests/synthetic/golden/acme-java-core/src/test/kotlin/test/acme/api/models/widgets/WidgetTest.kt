// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models.widgets

import com.fasterxml.jackson.module.kotlin.jacksonTypeRef
import java.time.OffsetDateTime
import kotlin.jvm.optionals.getOrNull
import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.Test
import test.acme.api.core.jsonMapper

internal class WidgetTest {

    @Test
    fun create() {
        val widget =
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

        assertThat(widget.id()).isEqualTo("id")
        assertThat(widget.createdAt()).isEqualTo(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
        assertThat(widget.name()).isEqualTo("name")
        assertThat(widget.status()).isEqualTo(Widget.Status.DRAFT)
        assertThat(widget.homepageUrl()).contains("homepage_url")
        assertThat(widget.parts().getOrNull())
            .containsExactly(Widget.Part.builder().label("label").partNumber(0L).build())
        assertThat(widget.sku()).contains("sku")
        assertThat(widget.weightKg()).contains(0.0)
    }

    @Test
    fun roundtrip() {
        val jsonMapper = jsonMapper()
        val widget =
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

        val roundtrippedWidget =
            jsonMapper.readValue(jsonMapper.writeValueAsString(widget), jacksonTypeRef<Widget>())

        assertThat(roundtrippedWidget).isEqualTo(widget)
    }
}
