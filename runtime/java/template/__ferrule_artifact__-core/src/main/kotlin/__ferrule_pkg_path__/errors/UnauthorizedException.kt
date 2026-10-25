// File generated from our OpenAPI spec by Stainless.

package __ferrule_pkg__.errors

//ferrule:if java
import java.util.Optional
import kotlin.jvm.optionals.getOrNull
//ferrule:endif
import __ferrule_pkg__.core.JsonValue
import __ferrule_pkg__.core.checkRequired
import __ferrule_pkg__.core.http.Headers
import __ferrule_pkg__.core.jsonMapper

class UnauthorizedException
private constructor(private val headers: Headers, private val body: JsonValue, cause: Throwable?) :
    __FerruleClient__ServiceException(
        "401: ${if (body.isMissing()) "Unknown" else jsonMapper().writeValueAsString(body)}",
        cause,
    ) {

    override fun statusCode(): Int = 401

    override fun headers(): Headers = headers

    override fun body(): JsonValue = body

    fun toBuilder() = Builder().from(this)

    companion object {

        /**
         * Returns a mutable builder for constructing an instance of [UnauthorizedException].
         *
         * The following fields are required:
//ferrule:if kotlin
         * ```kotlin
//ferrule:else
         * ```java
//ferrule:endif
         * .headers()
         * .body()
         * ```
         */
//ferrule:if kotlin
        fun builder() = Builder()
//ferrule:else
        @JvmStatic fun builder() = Builder()
//ferrule:endif
    }

    /** A builder for [UnauthorizedException]. */
    class Builder internal constructor() {

        private var headers: Headers? = null
        private var body: JsonValue? = null
        private var cause: Throwable? = null

//ferrule:if java
        @JvmSynthetic
//ferrule:endif
        internal fun from(unauthorizedException: UnauthorizedException) = apply {
            headers = unauthorizedException.headers
            body = unauthorizedException.body
            cause = unauthorizedException.cause
        }

        fun headers(headers: Headers) = apply { this.headers = headers }

        fun body(body: JsonValue) = apply { this.body = body }

        fun cause(cause: Throwable?) = apply { this.cause = cause }

//ferrule:if java
        /** Alias for calling [Builder.cause] with `cause.orElse(null)`. */
        fun cause(cause: Optional<Throwable>) = cause(cause.getOrNull())

//ferrule:endif
        /**
         * Returns an immutable instance of [UnauthorizedException].
         *
         * Further updates to this [Builder] will not mutate the returned instance.
         *
         * The following fields are required:
//ferrule:if kotlin
         * ```kotlin
//ferrule:else
         * ```java
//ferrule:endif
         * .headers()
         * .body()
         * ```
         *
         * @throws IllegalStateException if any required field is unset.
         */
        fun build(): UnauthorizedException =
            UnauthorizedException(
                checkRequired("headers", headers),
                checkRequired("body", body),
                cause,
            )
    }
}
