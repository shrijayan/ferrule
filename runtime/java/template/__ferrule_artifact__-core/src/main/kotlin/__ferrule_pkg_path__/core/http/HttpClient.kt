package __ferrule_pkg__.core.http

import java.lang.AutoCloseable
//ferrule:if java
import java.util.concurrent.CompletableFuture
//ferrule:endif
import __ferrule_pkg__.core.RequestOptions

interface HttpClient : AutoCloseable {

    fun execute(
        request: HttpRequest,
        requestOptions: RequestOptions = RequestOptions.none(),
    ): HttpResponse

//ferrule:if kotlin
    suspend fun executeAsync(
//ferrule:else
    fun execute(request: HttpRequest): HttpResponse = execute(request, RequestOptions.none())

    fun executeAsync(
//ferrule:endif
        request: HttpRequest,
        requestOptions: RequestOptions = RequestOptions.none(),
//ferrule:if kotlin
    ): HttpResponse
//ferrule:else
    ): CompletableFuture<HttpResponse>

    fun executeAsync(request: HttpRequest): CompletableFuture<HttpResponse> =
        executeAsync(request, RequestOptions.none())

    /** Overridden from [AutoCloseable] to not have a checked exception in its signature. */
    override fun close()
//ferrule:endif
}
