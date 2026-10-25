// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models.widgets

import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.Test

internal class WidgetRetrieveParamsTest {

    @Test
    fun create() {
        WidgetRetrieveParams.builder().widgetId("widget_id").build()
    }

    @Test
    fun pathParams() {
        val params = WidgetRetrieveParams.builder().widgetId("widget_id").build()

        assertThat(params._pathParam(0)).isEqualTo("widget_id")
        // out-of-bound path param
        assertThat(params._pathParam(1)).isEqualTo("")
    }
}
