// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.services

import com.github.tomakehurst.wiremock.client.WireMock.anyUrl
import com.github.tomakehurst.wiremock.client.WireMock.get
import com.github.tomakehurst.wiremock.client.WireMock.status
import com.github.tomakehurst.wiremock.client.WireMock.stubFor
import com.github.tomakehurst.wiremock.junit5.WireMockRuntimeInfo
import com.github.tomakehurst.wiremock.junit5.WireMockTest
import java.time.OffsetDateTime
import org.assertj.core.api.Assertions.assertThat
import org.assertj.core.api.Assertions.entry
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import org.junit.jupiter.api.parallel.ResourceLock
import test.acme.api.client.AcmeClient
import test.acme.api.client.okhttp.AcmeOkHttpClient
import test.acme.api.core.JsonValue
import test.acme.api.core.http.Headers
import test.acme.api.core.jsonMapper
import test.acme.api.errors.AcmeException
import test.acme.api.errors.BadRequestException
import test.acme.api.errors.InternalServerException
import test.acme.api.errors.NotFoundException
import test.acme.api.errors.PermissionDeniedException
import test.acme.api.errors.RateLimitException
import test.acme.api.errors.UnauthorizedException
import test.acme.api.errors.UnexpectedStatusCodeException
import test.acme.api.errors.UnprocessableEntityException
import test.acme.api.models.widgets.WidgetListParams

@WireMockTest
@ResourceLock("https://github.com/wiremock/wiremock/issues/169")
internal class ErrorHandlingTest {

    companion object {

        private val ERROR_JSON: JsonValue = JsonValue.from(mapOf("errorProperty" to "42"))

        private val ERROR_JSON_BYTES: ByteArray = jsonMapper().writeValueAsBytes(ERROR_JSON)

        private const val HEADER_NAME: String = "Error-Header"

        private const val HEADER_VALUE: String = "42"

        private const val NOT_JSON: String = "Not JSON"
    }

    private lateinit var client: AcmeClient

    @BeforeEach
    fun beforeEach(wmRuntimeInfo: WireMockRuntimeInfo) {
        client =
            AcmeOkHttpClient.builder()
                .baseUrl(wmRuntimeInfo.httpBaseUrl)
                .apiKey("My API Key")
                .build()
    }

    @Test
    fun widgetList400() {
        val widgetService = client.widgets()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(400).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<BadRequestException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(400)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList400WithRawResponse() {
        val widgetService = client.widgets().withRawResponse()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(400).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<BadRequestException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(400)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList401() {
        val widgetService = client.widgets()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(401).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<UnauthorizedException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(401)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList401WithRawResponse() {
        val widgetService = client.widgets().withRawResponse()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(401).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<UnauthorizedException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(401)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList403() {
        val widgetService = client.widgets()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(403).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<PermissionDeniedException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(403)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList403WithRawResponse() {
        val widgetService = client.widgets().withRawResponse()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(403).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<PermissionDeniedException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(403)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList404() {
        val widgetService = client.widgets()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(404).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<NotFoundException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(404)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList404WithRawResponse() {
        val widgetService = client.widgets().withRawResponse()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(404).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<NotFoundException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(404)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList422() {
        val widgetService = client.widgets()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(422).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<UnprocessableEntityException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(422)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList422WithRawResponse() {
        val widgetService = client.widgets().withRawResponse()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(422).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<UnprocessableEntityException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(422)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList429() {
        val widgetService = client.widgets()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(429).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<RateLimitException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(429)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList429WithRawResponse() {
        val widgetService = client.widgets().withRawResponse()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(429).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<RateLimitException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(429)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList500() {
        val widgetService = client.widgets()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(500).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<InternalServerException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(500)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList500WithRawResponse() {
        val widgetService = client.widgets().withRawResponse()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(500).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<InternalServerException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(500)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList999() {
        val widgetService = client.widgets()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(999).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<UnexpectedStatusCodeException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(999)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetList999WithRawResponse() {
        val widgetService = client.widgets().withRawResponse()
        stubFor(
            get(anyUrl())
                .willReturn(
                    status(999).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)
                )
        )

        val e =
            assertThrows<UnexpectedStatusCodeException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e.statusCode()).isEqualTo(999)
        assertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))
        assertThat(e.body()).isEqualTo(ERROR_JSON)
    }

    @Test
    fun widgetListInvalidJsonBody() {
        val widgetService = client.widgets()
        stubFor(
            get(anyUrl())
                .willReturn(status(200).withHeader(HEADER_NAME, HEADER_VALUE).withBody(NOT_JSON))
        )

        val e =
            assertThrows<AcmeException> {
                widgetService.list(
                    WidgetListParams.builder()
                        .colors("string")
                        .createdAfter(OffsetDateTime.parse("2019-12-27T18:11:19.117Z"))
                        .limit(0L)
                        .build()
                )
            }

        assertThat(e).hasMessage("Error reading response")
    }

    private fun Headers.toMap(): Map<String, List<String>> =
        mutableMapOf<String, List<String>>().also { map ->
            names().forEach { map[it] = values(it) }
        }
}
