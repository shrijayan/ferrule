package __ferrule_pkg__.core.http

//ferrule:if java
import java.util.concurrent.CompletableFuture
//ferrule:endif
import __ferrule_pkg__.core.RequestOptions
import __ferrule_pkg__.core.closeWhenPhantomReachable

/**
 * A delegating wrapper around an `HttpClient` that closes it once it's only phantom reachable.
 *
 * This class ensures the `HttpClient` is closed even if the user forgets to close it.
 */
internal class PhantomReachableClosingHttpClient(private val httpClient: HttpClient) : HttpClient {
    init {
        closeWhenPhantomReachable(this, httpClient)
    }

    override fun execute(request: HttpRequest, requestOptions: RequestOptions): HttpResponse =
        httpClient.execute(request, requestOptions)

//ferrule:if kotlin
    override suspend fun executeAsync(
//ferrule:else
    override fun executeAsync(
//ferrule:endif
        request: HttpRequest,
        requestOptions: RequestOptions,
//ferrule:if kotlin
    ): HttpResponse = httpClient.executeAsync(request, requestOptions)
//ferrule:else
    ): CompletableFuture<HttpResponse> = httpClient.executeAsync(request, requestOptions)
//ferrule:endif

    override fun close() = httpClient.close()
}
