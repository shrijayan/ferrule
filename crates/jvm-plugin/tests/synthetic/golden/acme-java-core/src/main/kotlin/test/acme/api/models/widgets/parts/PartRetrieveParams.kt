// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models.widgets.parts

import java.util.Objects
import java.util.Optional
import kotlin.jvm.optionals.getOrNull
import test.acme.api.core.Params
import test.acme.api.core.checkRequired
import test.acme.api.core.http.Headers
import test.acme.api.core.http.QueryParams

/** Retrieve one part of a widget */
class PartRetrieveParams
private constructor(
    private val widgetId: String,
    private val partNumber: Long?,
    private val additionalHeaders: Headers,
    private val additionalQueryParams: QueryParams,
) : Params {

    fun widgetId(): String = widgetId

    fun partNumber(): Optional<Long> = Optional.ofNullable(partNumber)

    /** Additional headers to send with the request. */
    fun _additionalHeaders(): Headers = additionalHeaders

    /** Additional query param to send with the request. */
    fun _additionalQueryParams(): QueryParams = additionalQueryParams

    fun toBuilder() = Builder().from(this)

    companion object {

        /**
         * Returns a mutable builder for constructing an instance of [PartRetrieveParams].
         *
         * The following fields are required:
         * ```java
         * .widgetId()
         * ```
         */
        @JvmStatic fun builder() = Builder()
    }

    /** A builder for [PartRetrieveParams]. */
    class Builder internal constructor() {

        private var widgetId: String? = null
        private var partNumber: Long? = null
        private var additionalHeaders: Headers.Builder = Headers.builder()
        private var additionalQueryParams: QueryParams.Builder = QueryParams.builder()

        @JvmSynthetic
        internal fun from(partRetrieveParams: PartRetrieveParams) = apply {
            widgetId = partRetrieveParams.widgetId
            partNumber = partRetrieveParams.partNumber
            additionalHeaders = partRetrieveParams.additionalHeaders.toBuilder()
            additionalQueryParams = partRetrieveParams.additionalQueryParams.toBuilder()
        }

        fun widgetId(widgetId: String) = apply { this.widgetId = widgetId }

        fun partNumber(partNumber: Long?) = apply { this.partNumber = partNumber }

        /**
         * Alias for [Builder.partNumber].
         *
         * This unboxed primitive overload exists for backwards compatibility.
         */
        fun partNumber(partNumber: Long) = partNumber(partNumber as Long?)

        /** Alias for calling [Builder.partNumber] with `partNumber.orElse(null)`. */
        fun partNumber(partNumber: Optional<Long>) = partNumber(partNumber.getOrNull())

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
         * Returns an immutable instance of [PartRetrieveParams].
         *
         * Further updates to this [Builder] will not mutate the returned instance.
         *
         * The following fields are required:
         * ```java
         * .widgetId()
         * ```
         *
         * @throws IllegalStateException if any required field is unset.
         */
        fun build(): PartRetrieveParams =
            PartRetrieveParams(
                checkRequired("widgetId", widgetId),
                partNumber,
                additionalHeaders.build(),
                additionalQueryParams.build(),
            )
    }

    fun _pathParam(index: Int): String =
        when (index) {
            0 -> widgetId
            1 -> partNumber?.toString() ?: ""
            else -> ""
        }

    override fun _headers(): Headers = additionalHeaders

    override fun _queryParams(): QueryParams = additionalQueryParams

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is PartRetrieveParams &&
            widgetId == other.widgetId &&
            partNumber == other.partNumber &&
            additionalHeaders == other.additionalHeaders &&
            additionalQueryParams == other.additionalQueryParams
    }

    override fun hashCode(): Int =
        Objects.hash(widgetId, partNumber, additionalHeaders, additionalQueryParams)

    override fun toString() =
        "PartRetrieveParams{widgetId=$widgetId, partNumber=$partNumber, additionalHeaders=$additionalHeaders, additionalQueryParams=$additionalQueryParams}"
}
