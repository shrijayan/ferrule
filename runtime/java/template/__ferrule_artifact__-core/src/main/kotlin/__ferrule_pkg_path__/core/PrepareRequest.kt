//ferrule:if java
@file:JvmName("PrepareRequest")

//ferrule:endif
package __ferrule_pkg__.core

//ferrule:if java
import java.util.concurrent.CompletableFuture
//ferrule:endif
import __ferrule_pkg__.core.http.HttpRequest

//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun HttpRequest.prepare(clientOptions: ClientOptions, params: Params): HttpRequest =
    toBuilder()
        .putAllQueryParams(clientOptions.queryParams)
        .replaceAllQueryParams(params._queryParams())
        .putAllHeaders(clientOptions.headers)
        .replaceAllHeaders(params._headers())
        .build()

//ferrule:if kotlin
internal suspend fun HttpRequest.prepareAsync(clientOptions: ClientOptions, params: Params) =
//ferrule:else
@JvmSynthetic
internal fun HttpRequest.prepareAsync(
    clientOptions: ClientOptions,
    params: Params,
): CompletableFuture<HttpRequest> =
//ferrule:endif
    // This async version exists to make it easier to add async specific preparation logic in the
    // future.
//ferrule:if kotlin
    prepare(clientOptions, params)
//ferrule:else
    CompletableFuture.completedFuture(prepare(clientOptions, params))
//ferrule:endif
