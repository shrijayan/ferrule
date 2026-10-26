// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models.widgets

import java.time.OffsetDateTime
import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.Test
import test.acme.api.core.http.QueryParams

internal class WidgetListParamsTest {

    @Test
    fun create() {
        WidgetListParams.builder()
            .colors("string")
            .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
            .limit(0L)
            .build()
    }

    @Test
    fun queryParams() {
        val params =
            WidgetListParams.builder()
                .colors("string")
                .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                .limit(0L)
                .build()

        val queryParams = params._queryParams()

        assertThat(queryParams)
            .isEqualTo(
                QueryParams.builder()
                    .put("colors", "string")
                    .put("created_after", "2019-12-27T18:11:19.117Z")
                    .put("limit", "0")
                    .build()
            )
    }

    @Test
    fun queryParamsWithoutOptionalFields() {
        val params = WidgetListParams.builder().build()

        val queryParams = params._queryParams()

        assertThat(queryParams).isEqualTo(QueryParams.builder().build())
    }
}
