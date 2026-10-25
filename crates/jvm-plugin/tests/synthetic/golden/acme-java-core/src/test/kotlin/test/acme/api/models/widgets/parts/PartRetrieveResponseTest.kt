// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models.widgets.parts

import com.fasterxml.jackson.module.kotlin.jacksonTypeRef
import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.Test
import test.acme.api.core.jsonMapper
import test.acme.api.models.ErrorInfo

internal class PartRetrieveResponseTest {

    @Test
    fun create() {
        val partRetrieveResponse =
            PartRetrieveResponse.builder()
                .id("id")
                .sku("sku")
                .error(ErrorInfo.builder().code("code").message("message").build())
                .build()

        assertThat(partRetrieveResponse.id()).isEqualTo("id")
        assertThat(partRetrieveResponse.sku()).isEqualTo("sku")
        assertThat(partRetrieveResponse.error())
            .contains(ErrorInfo.builder().code("code").message("message").build())
    }

    @Test
    fun roundtrip() {
        val jsonMapper = jsonMapper()
        val partRetrieveResponse =
            PartRetrieveResponse.builder()
                .id("id")
                .sku("sku")
                .error(ErrorInfo.builder().code("code").message("message").build())
                .build()

        val roundtrippedPartRetrieveResponse =
            jsonMapper.readValue(
                jsonMapper.writeValueAsString(partRetrieveResponse),
                jacksonTypeRef<PartRetrieveResponse>(),
            )

        assertThat(roundtrippedPartRetrieveResponse).isEqualTo(partRetrieveResponse)
    }
}
