// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models.widgets.parts

import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.Test

internal class PartRetrieveParamsTest {

    @Test
    fun create() {
        PartRetrieveParams.builder().widgetId("widget_id").partNumber(0L).build()
    }

    @Test
    fun pathParams() {
        val params = PartRetrieveParams.builder().widgetId("widget_id").partNumber(0L).build()

        assertThat(params._pathParam(0)).isEqualTo("widget_id")
        assertThat(params._pathParam(1)).isEqualTo("0")
        // out-of-bound path param
        assertThat(params._pathParam(2)).isEqualTo("")
    }
}
