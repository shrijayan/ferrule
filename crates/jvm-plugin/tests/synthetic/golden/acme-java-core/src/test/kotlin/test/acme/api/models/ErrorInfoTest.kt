// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models

import com.fasterxml.jackson.module.kotlin.jacksonTypeRef
import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.Test
import test.acme.api.core.jsonMapper

internal class ErrorInfoTest {

    @Test
    fun create() {
        val errorInfo = ErrorInfo.builder().code("code").message("message").build()

        assertThat(errorInfo.code()).contains("code")
        assertThat(errorInfo.message()).contains("message")
    }

    @Test
    fun roundtrip() {
        val jsonMapper = jsonMapper()
        val errorInfo = ErrorInfo.builder().code("code").message("message").build()

        val roundtrippedErrorInfo =
            jsonMapper.readValue(
                jsonMapper.writeValueAsString(errorInfo),
                jacksonTypeRef<ErrorInfo>(),
            )

        assertThat(roundtrippedErrorInfo).isEqualTo(errorInfo)
    }
}
