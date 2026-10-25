// File generated from our OpenAPI spec by Stainless.
//ferrule:if java

@file:JvmName("ErrorHandler")
//ferrule:endif

package __ferrule_pkg__.core.handlers

import com.fasterxml.jackson.databind.json.JsonMapper
import __ferrule_pkg__.core.JsonMissing
import __ferrule_pkg__.core.JsonValue
import __ferrule_pkg__.core.http.HttpResponse
import __ferrule_pkg__.core.http.HttpResponse.Handler
import __ferrule_pkg__.errors.BadRequestException
import __ferrule_pkg__.errors.InternalServerException
import __ferrule_pkg__.errors.NotFoundException
import __ferrule_pkg__.errors.PermissionDeniedException
import __ferrule_pkg__.errors.RateLimitException
import __ferrule_pkg__.errors.UnauthorizedException
import __ferrule_pkg__.errors.UnexpectedStatusCodeException
import __ferrule_pkg__.errors.UnprocessableEntityException

//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun errorBodyHandler(jsonMapper: JsonMapper): Handler<JsonValue> {
    val handler = jsonHandler<JsonValue>(jsonMapper)

    return object : Handler<JsonValue> {
        override fun handle(response: HttpResponse): JsonValue =
            try {
                handler.handle(response)
            } catch (e: Exception) {
                JsonMissing.of()
            }
    }
}

//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun errorHandler(errorBodyHandler: Handler<JsonValue>): Handler<HttpResponse> =
    object : Handler<HttpResponse> {
        override fun handle(response: HttpResponse): HttpResponse =
            when (val statusCode = response.statusCode()) {
                in 200..299 -> response
                400 ->
                    throw BadRequestException.builder()
                        .headers(response.headers())
                        .body(errorBodyHandler.handle(response))
                        .build()
                401 ->
                    throw UnauthorizedException.builder()
                        .headers(response.headers())
                        .body(errorBodyHandler.handle(response))
                        .build()
                403 ->
                    throw PermissionDeniedException.builder()
                        .headers(response.headers())
                        .body(errorBodyHandler.handle(response))
                        .build()
                404 ->
                    throw NotFoundException.builder()
                        .headers(response.headers())
                        .body(errorBodyHandler.handle(response))
                        .build()
                422 ->
                    throw UnprocessableEntityException.builder()
                        .headers(response.headers())
                        .body(errorBodyHandler.handle(response))
                        .build()
                429 ->
                    throw RateLimitException.builder()
                        .headers(response.headers())
                        .body(errorBodyHandler.handle(response))
                        .build()
                in 500..599 ->
                    throw InternalServerException.builder()
                        .statusCode(statusCode)
                        .headers(response.headers())
                        .body(errorBodyHandler.handle(response))
                        .build()
                else ->
                    throw UnexpectedStatusCodeException.builder()
                        .statusCode(statusCode)
                        .headers(response.headers())
                        .body(errorBodyHandler.handle(response))
                        .build()
            }
    }
