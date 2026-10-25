// File generated from our OpenAPI spec by Stainless.

package __ferrule_pkg__.core.http

import com.github.tomakehurst.wiremock.client.WireMock.equalTo
import com.github.tomakehurst.wiremock.client.WireMock.matching
import com.github.tomakehurst.wiremock.client.WireMock.ok
import com.github.tomakehurst.wiremock.client.WireMock.post
import com.github.tomakehurst.wiremock.client.WireMock.postRequestedFor
import com.github.tomakehurst.wiremock.client.WireMock.resetAllScenarios
import com.github.tomakehurst.wiremock.client.WireMock.serviceUnavailable
import com.github.tomakehurst.wiremock.client.WireMock.stubFor
import com.github.tomakehurst.wiremock.client.WireMock.urlPathEqualTo
import com.github.tomakehurst.wiremock.client.WireMock.verify
import com.github.tomakehurst.wiremock.junit5.WireMockRuntimeInfo
import com.github.tomakehurst.wiremock.junit5.WireMockTest
import com.github.tomakehurst.wiremock.stubbing.Scenario
import java.io.InputStream
//ferrule:if java
import java.time.Clock
//ferrule:endif
import java.time.Duration
//ferrule:if kotlin
import kotlinx.coroutines.runBlocking
//ferrule:else
import java.time.OffsetDateTime
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.util.concurrent.CompletableFuture
//ferrule:endif
import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.parallel.ResourceLock
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.ValueSource
import __ferrule_pkg__.client.okhttp.OkHttpClient
import __ferrule_pkg__.core.RequestOptions
import __ferrule_pkg__.core.Sleeper
import __ferrule_pkg__.errors.__FerruleClient__RetryableException

@WireMockTest
@ResourceLock("https://github.com/wiremock/wiremock/issues/169")
internal class RetryingHttpClientTest {

    private var openResponseCount = 0
    private lateinit var baseUrl: String
    private lateinit var httpClient: HttpClient

//ferrule:if java
    private class RecordingSleeper : Sleeper {
        val durations = mutableListOf<Duration>()

        override fun sleep(duration: Duration) {
            durations.add(duration)
        }

        override fun sleepAsync(duration: Duration): CompletableFuture<Void> {
            durations.add(duration)
            return CompletableFuture.completedFuture(null)
        }

        override fun close() {}
    }

//ferrule:endif
    @BeforeEach
    fun beforeEach(wmRuntimeInfo: WireMockRuntimeInfo) {
        baseUrl = wmRuntimeInfo.httpBaseUrl
        val okHttpClient = OkHttpClient.builder().build()
        httpClient =
            object : HttpClient {

                override fun execute(
                    request: HttpRequest,
                    requestOptions: RequestOptions,
                ): HttpResponse = trackClose(okHttpClient.execute(request, requestOptions))

//ferrule:if kotlin
                override suspend fun executeAsync(
//ferrule:else
                override fun executeAsync(
//ferrule:endif
                    request: HttpRequest,
                    requestOptions: RequestOptions,
//ferrule:if kotlin
                ): HttpResponse = trackClose(okHttpClient.executeAsync(request, requestOptions))
//ferrule:else
                ): CompletableFuture<HttpResponse> =
                    okHttpClient.executeAsync(request, requestOptions).thenApply { trackClose(it) }
//ferrule:endif

                override fun close() = okHttpClient.close()

                private fun trackClose(response: HttpResponse): HttpResponse {
                    openResponseCount++
                    return object : HttpResponse {

                        private var isClosed = false

                        override fun statusCode(): Int = response.statusCode()

                        override fun headers(): Headers = response.headers()

                        override fun body(): InputStream = response.body()

                        override fun close() {
                            response.close()
                            if (isClosed) {
                                return
                            }
                            openResponseCount--
                            isClosed = true
                        }
                    }
                }
            }
        resetAllScenarios()
    }

    @ParameterizedTest
    @ValueSource(booleans = [false, true])
    fun execute(async: Boolean) {
        stubFor(post(urlPathEqualTo("/something")).willReturn(ok()))
//ferrule:if kotlin
        val retryingClient = retryingHttpClientBuilder().build()
//ferrule:else
        val sleeper = RecordingSleeper()
        val retryingClient = retryingHttpClientBuilder(sleeper).build()
//ferrule:endif

        val response =
            retryingClient.execute(
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(baseUrl)
                    .addPathSegment("something")
                    .build(),
                async,
            )

        assertThat(response.statusCode()).isEqualTo(200)
        verify(1, postRequestedFor(urlPathEqualTo("/something")))
//ferrule:if java
        assertThat(sleeper.durations).isEmpty()
//ferrule:endif
        assertNoResponseLeaks()
    }

    @ParameterizedTest
    @ValueSource(booleans = [false, true])
    fun execute_withIdempotencyHeader(async: Boolean) {
        stubFor(
            post(urlPathEqualTo("/something"))
                .withHeader("X-Some-Header", matching("stainless-java-retry-.+"))
                .willReturn(ok())
        )
//ferrule:if java
        val sleeper = RecordingSleeper()
//ferrule:endif
        val retryingClient =
//ferrule:if kotlin
            retryingHttpClientBuilder().maxRetries(2).idempotencyHeader("X-Some-Header").build()
//ferrule:else
            retryingHttpClientBuilder(sleeper)
                .maxRetries(2)
                .idempotencyHeader("X-Some-Header")
                .build()
//ferrule:endif

        val response =
            retryingClient.execute(
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(baseUrl)
                    .addPathSegment("something")
                    .build(),
                async,
            )

        assertThat(response.statusCode()).isEqualTo(200)
        verify(1, postRequestedFor(urlPathEqualTo("/something")))
//ferrule:if java
        assertThat(sleeper.durations).isEmpty()
//ferrule:endif
        assertNoResponseLeaks()
    }

    @ParameterizedTest
    @ValueSource(booleans = [false, true])
    fun execute_withRetryAfterHeader(async: Boolean) {
//ferrule:if java
        val retryAfterDate = "Wed, 21 Oct 2015 07:28:00 GMT"
//ferrule:endif
        stubFor(
            post(urlPathEqualTo("/something"))
                // First we fail with a retry after header given as a date
                .inScenario("foo")
                .whenScenarioStateIs(Scenario.STARTED)
//ferrule:if kotlin
                .willReturn(
                    serviceUnavailable().withHeader("Retry-After", "Wed, 21 Oct 2015 07:28:00 GMT")
                )
//ferrule:else
                .willReturn(serviceUnavailable().withHeader("Retry-After", retryAfterDate))
//ferrule:endif
                .willSetStateTo("RETRY_AFTER_DATE")
        )
        stubFor(
            post(urlPathEqualTo("/something"))
                // Then we fail with a retry after header given as a delay
                .inScenario("foo")
                .whenScenarioStateIs("RETRY_AFTER_DATE")
                .willReturn(serviceUnavailable().withHeader("Retry-After", "1.234"))
                .willSetStateTo("RETRY_AFTER_DELAY")
        )
        stubFor(
            post(urlPathEqualTo("/something"))
                // Then we return a success
                .inScenario("foo")
                .whenScenarioStateIs("RETRY_AFTER_DELAY")
                .willReturn(ok())
                .willSetStateTo("COMPLETED")
        )
//ferrule:if kotlin
        val retryingClient = retryingHttpClientBuilder().maxRetries(2).build()
//ferrule:else
        // Fix the clock to 5 seconds before the Retry-After date so the date-based backoff is
        // deterministic.
        val retryAfterDateTime =
            OffsetDateTime.parse(retryAfterDate, DateTimeFormatter.RFC_1123_DATE_TIME)
        val clock = Clock.fixed(retryAfterDateTime.minusSeconds(5).toInstant(), ZoneOffset.UTC)
        val sleeper = RecordingSleeper()
        val retryingClient = retryingHttpClientBuilder(sleeper, clock).maxRetries(2).build()
//ferrule:endif

        val response =
            retryingClient.execute(
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(baseUrl)
                    .addPathSegment("something")
                    .build(),
                async,
            )

        assertThat(response.statusCode()).isEqualTo(200)
        verify(
            1,
            postRequestedFor(urlPathEqualTo("/something"))
                .withHeader("x-stainless-retry-count", equalTo("0")),
        )
        verify(
            1,
            postRequestedFor(urlPathEqualTo("/something"))
                .withHeader("x-stainless-retry-count", equalTo("1")),
        )
        verify(
            1,
            postRequestedFor(urlPathEqualTo("/something"))
                .withHeader("x-stainless-retry-count", equalTo("2")),
        )
//ferrule:if java
        assertThat(sleeper.durations)
            .containsExactly(Duration.ofSeconds(5), Duration.ofMillis(1234))
//ferrule:endif
        assertNoResponseLeaks()
    }

    @ParameterizedTest
    @ValueSource(booleans = [false, true])
    fun execute_withOverwrittenRetryCountHeader(async: Boolean) {
//ferrule:if java
        val retryAfterDate = "Wed, 21 Oct 2015 07:28:00 GMT"
//ferrule:endif
        stubFor(
            post(urlPathEqualTo("/something"))
                .inScenario("foo") // first we fail with a retry after header given as a date
                .whenScenarioStateIs(Scenario.STARTED)
//ferrule:if kotlin
                .willReturn(
                    serviceUnavailable().withHeader("Retry-After", "Wed, 21 Oct 2015 07:28:00 GMT")
                )
//ferrule:else
                .willReturn(serviceUnavailable().withHeader("Retry-After", retryAfterDate))
//ferrule:endif
                .willSetStateTo("RETRY_AFTER_DATE")
        )
        stubFor(
            post(urlPathEqualTo("/something"))
                .inScenario("foo") // then we return a success
                .whenScenarioStateIs("RETRY_AFTER_DATE")
                .willReturn(ok())
                .willSetStateTo("COMPLETED")
        )
//ferrule:if kotlin
        val retryingClient = retryingHttpClientBuilder().maxRetries(2).build()
//ferrule:else
        val retryAfterDateTime =
            OffsetDateTime.parse(retryAfterDate, DateTimeFormatter.RFC_1123_DATE_TIME)
        val clock = Clock.fixed(retryAfterDateTime.minusSeconds(5).toInstant(), ZoneOffset.UTC)
        val sleeper = RecordingSleeper()
        val retryingClient = retryingHttpClientBuilder(sleeper, clock).maxRetries(2).build()
//ferrule:endif

        val response =
            retryingClient.execute(
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(baseUrl)
                    .addPathSegment("something")
                    .putHeader("x-stainless-retry-count", "42")
                    .build(),
                async,
            )

        assertThat(response.statusCode()).isEqualTo(200)
        verify(
            2,
            postRequestedFor(urlPathEqualTo("/something"))
                .withHeader("x-stainless-retry-count", equalTo("42")),
        )
//ferrule:if java
        assertThat(sleeper.durations).containsExactly(Duration.ofSeconds(5))
//ferrule:endif
        assertNoResponseLeaks()
    }

    @ParameterizedTest
    @ValueSource(booleans = [false, true])
    fun execute_withRetryAfterMsHeader(async: Boolean) {
        stubFor(
            post(urlPathEqualTo("/something"))
                .inScenario("foo")
                .whenScenarioStateIs(Scenario.STARTED)
                .willReturn(serviceUnavailable().withHeader("Retry-After-Ms", "10"))
                .willSetStateTo("RETRY_AFTER_DELAY")
        )
        stubFor(
            post(urlPathEqualTo("/something"))
                .inScenario("foo") // then we return a success
                .whenScenarioStateIs("RETRY_AFTER_DELAY")
                .willReturn(ok())
                .willSetStateTo("COMPLETED")
        )
//ferrule:if kotlin
        val retryingClient = retryingHttpClientBuilder().maxRetries(1).build()
//ferrule:else
        val sleeper = RecordingSleeper()
        val retryingClient = retryingHttpClientBuilder(sleeper).maxRetries(1).build()
//ferrule:endif

        val response =
            retryingClient.execute(
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(baseUrl)
                    .addPathSegment("something")
                    .build(),
                async,
            )

        assertThat(response.statusCode()).isEqualTo(200)
        verify(2, postRequestedFor(urlPathEqualTo("/something")))
//ferrule:if java
        assertThat(sleeper.durations).containsExactly(Duration.ofMillis(10))
//ferrule:endif
        assertNoResponseLeaks()
    }

    @ParameterizedTest
    @ValueSource(booleans = [false, true])
    fun execute_withRetryableException(async: Boolean) {
        stubFor(post(urlPathEqualTo("/something")).willReturn(ok()))

        var callCount = 0
        val failingHttpClient =
            object : HttpClient {
                override fun execute(
                    request: HttpRequest,
                    requestOptions: RequestOptions,
                ): HttpResponse {
                    callCount++
                    if (callCount == 1) {
                        throw __FerruleClient__RetryableException("Simulated retryable failure")
                    }
                    return httpClient.execute(request, requestOptions)
                }

//ferrule:if kotlin
                override suspend fun executeAsync(
//ferrule:else
                override fun executeAsync(
//ferrule:endif
                    request: HttpRequest,
                    requestOptions: RequestOptions,
//ferrule:if kotlin
                ): HttpResponse {
//ferrule:else
                ): CompletableFuture<HttpResponse> {
//ferrule:endif
                    callCount++
                    if (callCount == 1) {
//ferrule:if kotlin
                        throw __FerruleClient__RetryableException("Simulated retryable failure")
//ferrule:else
                        val future = CompletableFuture<HttpResponse>()
                        future.completeExceptionally(
                            __FerruleClient__RetryableException("Simulated retryable failure")
                        )
                        return future
//ferrule:endif
                    }
                    return httpClient.executeAsync(request, requestOptions)
                }

                override fun close() = httpClient.close()
            }

//ferrule:if java
        val sleeper = RecordingSleeper()
//ferrule:endif
        val retryingClient =
            RetryingHttpClient.builder()
                .httpClient(failingHttpClient)
                .maxRetries(2)
//ferrule:if kotlin
                .sleeper(
                    object : Sleeper {

                        override fun sleep(duration: Duration) {}

                        override suspend fun sleepAsync(duration: Duration) {}

                        override fun close() {}
                    }
                )
//ferrule:else
                .sleeper(sleeper)
//ferrule:endif
                .build()

        val response =
            retryingClient.execute(
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(baseUrl)
                    .addPathSegment("something")
                    .build(),
                async,
            )

        assertThat(response.statusCode()).isEqualTo(200)
        verify(
            1,
            postRequestedFor(urlPathEqualTo("/something"))
                .withHeader("x-stainless-retry-count", equalTo("1")),
        )
        verify(
            0,
            postRequestedFor(urlPathEqualTo("/something"))
                .withHeader("x-stainless-retry-count", equalTo("0")),
        )
//ferrule:if java
        // Exponential backoff with jitter: 0.5s * jitter where jitter is in [0.75, 1.0].
        assertThat(sleeper.durations).hasSize(1)
        assertThat(sleeper.durations[0]).isBetween(Duration.ofMillis(375), Duration.ofMillis(500))
//ferrule:endif
        assertNoResponseLeaks()
    }

//ferrule:if kotlin
    private fun retryingHttpClientBuilder() =
        RetryingHttpClient.builder()
            .httpClient(httpClient)
            // Use a no-op `Sleeper` to make the test fast.
            .sleeper(
                object : Sleeper {
//ferrule:else
    @ParameterizedTest
    @ValueSource(booleans = [false, true])
    fun execute_withExponentialBackoff(async: Boolean) {
        stubFor(post(urlPathEqualTo("/something")).willReturn(serviceUnavailable()))
        val sleeper = RecordingSleeper()
        val retryingClient = retryingHttpClientBuilder(sleeper).maxRetries(3).build()
//ferrule:endif

//ferrule:if kotlin
                    override fun sleep(duration: Duration) {}

                    override suspend fun sleepAsync(duration: Duration) {}

                    override fun close() {}
                }
//ferrule:else
        val response =
            retryingClient.execute(
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(baseUrl)
                    .addPathSegment("something")
                    .build(),
                async,
//ferrule:endif
            )

//ferrule:if java
        // All retries exhausted; the last 503 response is returned.
        assertThat(response.statusCode()).isEqualTo(503)
        verify(4, postRequestedFor(urlPathEqualTo("/something")))
        // Exponential backoff with jitter: backoff = min(0.5 * 2^(retries-1), 8) * jitter where
        // jitter is in [0.75, 1.0].
        assertThat(sleeper.durations).hasSize(3)
        // retries=1: 0.5s * [0.75, 1.0]
        assertThat(sleeper.durations[0]).isBetween(Duration.ofMillis(375), Duration.ofMillis(500))
        // retries=2: 1s * [0.75, 1.0]
        assertThat(sleeper.durations[1]).isBetween(Duration.ofMillis(750), Duration.ofMillis(1000))
        // retries=3: 2s * [0.75, 1.0]
        assertThat(sleeper.durations[2]).isBetween(Duration.ofMillis(1500), Duration.ofMillis(2000))
        assertNoResponseLeaks()
    }

    @ParameterizedTest
    @ValueSource(booleans = [false, true])
    fun execute_withExponentialBackoffCap(async: Boolean) {
        stubFor(post(urlPathEqualTo("/something")).willReturn(serviceUnavailable()))
        val sleeper = RecordingSleeper()
        val retryingClient = retryingHttpClientBuilder(sleeper).maxRetries(6).build()

        val response =
            retryingClient.execute(
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(baseUrl)
                    .addPathSegment("something")
                    .build(),
                async,
            )

        assertThat(response.statusCode()).isEqualTo(503)
        verify(7, postRequestedFor(urlPathEqualTo("/something")))
        assertThat(sleeper.durations).hasSize(6)
        // retries=5: backoff hits the 8s cap * [0.75, 1.0]
        assertThat(sleeper.durations[4]).isBetween(Duration.ofMillis(6000), Duration.ofMillis(8000))
        // retries=6: still capped at 8s * [0.75, 1.0]
        assertThat(sleeper.durations[5]).isBetween(Duration.ofMillis(6000), Duration.ofMillis(8000))
        assertNoResponseLeaks()
    }

    @ParameterizedTest
    @ValueSource(booleans = [false, true])
    fun execute_withRetryAfterMsPriorityOverRetryAfter(async: Boolean) {
        stubFor(
            post(urlPathEqualTo("/something"))
                .inScenario("foo")
                .whenScenarioStateIs(Scenario.STARTED)
                .willReturn(
                    serviceUnavailable()
                        .withHeader("Retry-After-Ms", "50")
                        .withHeader("Retry-After", "2")
                )
                .willSetStateTo("RETRY")
        )
        stubFor(
            post(urlPathEqualTo("/something"))
                .inScenario("foo")
                .whenScenarioStateIs("RETRY")
                .willReturn(ok())
                .willSetStateTo("COMPLETED")
        )
        val sleeper = RecordingSleeper()
        val retryingClient = retryingHttpClientBuilder(sleeper).maxRetries(1).build()

        val response =
            retryingClient.execute(
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(baseUrl)
                    .addPathSegment("something")
                    .build(),
                async,
            )

        assertThat(response.statusCode()).isEqualTo(200)
        // Retry-After-Ms (50ms) takes priority over Retry-After (2s).
        assertThat(sleeper.durations).containsExactly(Duration.ofMillis(50))
        assertNoResponseLeaks()
    }

    @ParameterizedTest
    @ValueSource(booleans = [false, true])
    fun execute_withRetryAfterUnparseable(async: Boolean) {
        stubFor(
            post(urlPathEqualTo("/something"))
                .inScenario("foo")
                .whenScenarioStateIs(Scenario.STARTED)
                .willReturn(serviceUnavailable().withHeader("Retry-After", "not-a-date-or-number"))
                .willSetStateTo("RETRY")
        )
        stubFor(
            post(urlPathEqualTo("/something"))
                .inScenario("foo")
                .whenScenarioStateIs("RETRY")
                .willReturn(ok())
                .willSetStateTo("COMPLETED")
        )
        val sleeper = RecordingSleeper()
        val retryingClient = retryingHttpClientBuilder(sleeper).maxRetries(1).build()

        val response =
            retryingClient.execute(
                HttpRequest.builder()
                    .method(HttpMethod.POST)
                    .baseUrl(baseUrl)
                    .addPathSegment("something")
                    .build(),
                async,
            )

        assertThat(response.statusCode()).isEqualTo(200)
        // Unparseable Retry-After falls through to exponential backoff.
        assertThat(sleeper.durations).hasSize(1)
        assertThat(sleeper.durations[0]).isBetween(Duration.ofMillis(375), Duration.ofMillis(500))
        assertNoResponseLeaks()
    }

    private fun retryingHttpClientBuilder(
        sleeper: RecordingSleeper,
        clock: Clock = Clock.systemUTC(),
    ) = RetryingHttpClient.builder().httpClient(httpClient).sleeper(sleeper).clock(clock)

//ferrule:endif
    private fun HttpClient.execute(request: HttpRequest, async: Boolean): HttpResponse =
//ferrule:if kotlin
        if (async) runBlocking { executeAsync(request) } else execute(request)
//ferrule:else
        if (async) executeAsync(request).get() else execute(request)
//ferrule:endif

    // When retrying, all failed responses should be closed. Only the final returned response should
    // be open.
    private fun assertNoResponseLeaks() = assertThat(openResponseCount).isEqualTo(1)
}
