// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models.widgets

import kotlin.jvm.optionals.getOrNull
import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.Test

internal class WidgetCreateParamsTest {

    @Test
    fun create() {
        WidgetCreateParams.builder()
            .name("name")
            .color(WidgetCreateParams.Color.RED)
            .dimensions(WidgetCreateParams.Dimensions.builder().width(0.0).height(0.0).build())
            .addTag("string")
            .build()
    }

    @Test
    fun body() {
        val params =
            WidgetCreateParams.builder()
                .name("name")
                .color(WidgetCreateParams.Color.RED)
                .dimensions(WidgetCreateParams.Dimensions.builder().width(0.0).height(0.0).build())
                .addTag("string")
                .build()

        val body = params._body()

        assertThat(body.name()).isEqualTo("name")
        assertThat(body.color()).contains(WidgetCreateParams.Color.RED)
        assertThat(body.dimensions())
            .contains(WidgetCreateParams.Dimensions.builder().width(0.0).height(0.0).build())
        assertThat(body.tags().getOrNull()).containsExactly("string")
    }

    @Test
    fun bodyWithoutOptionalFields() {
        val params = WidgetCreateParams.builder().name("name").build()

        val body = params._body()

        assertThat(body.name()).isEqualTo("name")
    }
}
