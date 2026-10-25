// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models.widgets

import java.time.OffsetDateTime
import java.time.format.DateTimeFormatter
import java.util.Objects
import java.util.Optional
import kotlin.jvm.optionals.getOrNull
import test.acme.api.core.Params
import test.acme.api.core.http.Headers
import test.acme.api.core.http.QueryParams

/** List widgets */
class WidgetListParams
private constructor(
    private val colors: List<String>?,
    private val createdAfter: OffsetDateTime?,
    private val limit: Long?,
    private val additionalHeaders: Headers,
    private val additionalQueryParams: QueryParams,
) : Params {

    fun colors(): Optional<List<String>> = Optional.ofNullable(colors)

    fun createdAfter(): Optional<OffsetDateTime> = Optional.ofNullable(createdAfter)

    fun limit(): Optional<Long> = Optional.ofNullable(limit)

    /** Additional headers to send with the request. */
    fun _additionalHeaders(): Headers = additionalHeaders

    /** Additional query param to send with the request. */
    fun _additionalQueryParams(): QueryParams = additionalQueryParams

    fun toBuilder() = Builder().from(this)

    companion object {

        @JvmStatic fun none(): WidgetListParams = builder().build()

        /** Returns a mutable builder for constructing an instance of [WidgetListParams]. */
        @JvmStatic fun builder() = Builder()
    }

    /** A builder for [WidgetListParams]. */
    class Builder internal constructor() {

        private var colors: List<String>? = null
        private var createdAfter: OffsetDateTime? = null
        private var limit: Long? = null
        private var additionalHeaders: Headers.Builder = Headers.builder()
        private var additionalQueryParams: QueryParams.Builder = QueryParams.builder()

        @JvmSynthetic
        internal fun from(widgetListParams: WidgetListParams) = apply {
            colors = widgetListParams.colors
            createdAfter = widgetListParams.createdAfter
            limit = widgetListParams.limit
            additionalHeaders = widgetListParams.additionalHeaders.toBuilder()
            additionalQueryParams = widgetListParams.additionalQueryParams.toBuilder()
        }

        fun colors(colors: List<String>?) = apply { this.colors = colors }

        /** Alias for calling [Builder.colors] with `colors.orElse(null)`. */
        fun colors(colors: Optional<List<String>>) = colors(colors.getOrNull())

        fun createdAfter(createdAfter: OffsetDateTime?) = apply { this.createdAfter = createdAfter }

        /** Alias for calling [Builder.createdAfter] with `createdAfter.orElse(null)`. */
        fun createdAfter(createdAfter: Optional<OffsetDateTime>) =
            createdAfter(createdAfter.getOrNull())

        fun limit(limit: Long?) = apply { this.limit = limit }

        /**
         * Alias for [Builder.limit].
         *
         * This unboxed primitive overload exists for backwards compatibility.
         */
        fun limit(limit: Long) = limit(limit as Long?)

        /** Alias for calling [Builder.limit] with `limit.orElse(null)`. */
        fun limit(limit: Optional<Long>) = limit(limit.getOrNull())

        fun additionalHeaders(additionalHeaders: Headers) = apply {
            this.additionalHeaders.clear()
            putAllAdditionalHeaders(additionalHeaders)
        }

        fun additionalHeaders(additionalHeaders: Map<String, Iterable<String>>) = apply {
            this.additionalHeaders.clear()
            putAllAdditionalHeaders(additionalHeaders)
        }

        fun putAdditionalHeader(name: String, value: String) = apply {
            additionalHeaders.put(name, value)
        }

        fun putAdditionalHeaders(name: String, values: Iterable<String>) = apply {
            additionalHeaders.put(name, values)
        }

        fun putAllAdditionalHeaders(additionalHeaders: Headers) = apply {
            this.additionalHeaders.putAll(additionalHeaders)
        }

        fun putAllAdditionalHeaders(additionalHeaders: Map<String, Iterable<String>>) = apply {
            this.additionalHeaders.putAll(additionalHeaders)
        }

        fun replaceAdditionalHeaders(name: String, value: String) = apply {
            additionalHeaders.replace(name, value)
        }

        fun replaceAdditionalHeaders(name: String, values: Iterable<String>) = apply {
            additionalHeaders.replace(name, values)
        }

        fun replaceAllAdditionalHeaders(additionalHeaders: Headers) = apply {
            this.additionalHeaders.replaceAll(additionalHeaders)
        }

        fun replaceAllAdditionalHeaders(additionalHeaders: Map<String, Iterable<String>>) = apply {
            this.additionalHeaders.replaceAll(additionalHeaders)
        }

        fun removeAdditionalHeaders(name: String) = apply { additionalHeaders.remove(name) }

        fun removeAllAdditionalHeaders(names: Set<String>) = apply {
            additionalHeaders.removeAll(names)
        }

        fun additionalQueryParams(additionalQueryParams: QueryParams) = apply {
            this.additionalQueryParams.clear()
            putAllAdditionalQueryParams(additionalQueryParams)
        }

        fun additionalQueryParams(additionalQueryParams: Map<String, Iterable<String>>) = apply {
            this.additionalQueryParams.clear()
            putAllAdditionalQueryParams(additionalQueryParams)
        }

        fun putAdditionalQueryParam(key: String, value: String) = apply {
            additionalQueryParams.put(key, value)
        }

        fun putAdditionalQueryParams(key: String, values: Iterable<String>) = apply {
            additionalQueryParams.put(key, values)
        }

        fun putAllAdditionalQueryParams(additionalQueryParams: QueryParams) = apply {
            this.additionalQueryParams.putAll(additionalQueryParams)
        }

        fun putAllAdditionalQueryParams(additionalQueryParams: Map<String, Iterable<String>>) =
            apply {
                this.additionalQueryParams.putAll(additionalQueryParams)
            }

        fun replaceAdditionalQueryParams(key: String, value: String) = apply {
            additionalQueryParams.replace(key, value)
        }

        fun replaceAdditionalQueryParams(key: String, values: Iterable<String>) = apply {
            additionalQueryParams.replace(key, values)
        }

        fun replaceAllAdditionalQueryParams(additionalQueryParams: QueryParams) = apply {
            this.additionalQueryParams.replaceAll(additionalQueryParams)
        }

        fun replaceAllAdditionalQueryParams(additionalQueryParams: Map<String, Iterable<String>>) =
            apply {
                this.additionalQueryParams.replaceAll(additionalQueryParams)
            }

        fun removeAdditionalQueryParams(key: String) = apply { additionalQueryParams.remove(key) }

        fun removeAllAdditionalQueryParams(keys: Set<String>) = apply {
            additionalQueryParams.removeAll(keys)
        }

        /**
         * Returns an immutable instance of [WidgetListParams].
         *
         * Further updates to this [Builder] will not mutate the returned instance.
         */
        fun build(): WidgetListParams =
            WidgetListParams(
                colors,
                createdAfter,
                limit,
                additionalHeaders.build(),
                additionalQueryParams.build(),
            )
    }

    override fun _headers(): Headers = additionalHeaders

    override fun _queryParams(): QueryParams =
        QueryParams.builder()
            .apply {
                colors?.let { put("colors", it.joinToString(",")) }
                createdAfter?.let {
                    put("created_after", DateTimeFormatter.ISO_OFFSET_DATE_TIME.format(it))
                }
                limit?.let { put("limit", it.toString()) }
                putAll(additionalQueryParams)
            }
            .build()

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is WidgetListParams &&
            colors == other.colors &&
            createdAfter == other.createdAfter &&
            limit == other.limit &&
            additionalHeaders == other.additionalHeaders &&
            additionalQueryParams == other.additionalQueryParams
    }

    override fun hashCode(): Int =
        Objects.hash(colors, createdAfter, limit, additionalHeaders, additionalQueryParams)

    override fun toString() =
        "WidgetListParams{colors=$colors, createdAfter=$createdAfter, limit=$limit, additionalHeaders=$additionalHeaders, additionalQueryParams=$additionalQueryParams}"
}
