// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models.widgets.parts

import com.fasterxml.jackson.annotation.JsonAnyGetter
import com.fasterxml.jackson.annotation.JsonAnySetter
import com.fasterxml.jackson.annotation.JsonCreator
import com.fasterxml.jackson.annotation.JsonProperty
import java.util.Collections
import java.util.Objects
import java.util.Optional
import kotlin.jvm.optionals.getOrNull
import test.acme.api.core.ExcludeMissing
import test.acme.api.core.JsonField
import test.acme.api.core.JsonMissing
import test.acme.api.core.JsonValue
import test.acme.api.core.checkRequired
import test.acme.api.errors.AcmeInvalidDataException
import test.acme.api.models.ErrorInfo

class PartRetrieveResponse
@JsonCreator(mode = JsonCreator.Mode.DISABLED)
private constructor(
    private val id: JsonField<String>,
    private val sku: JsonField<String>,
    private val error: JsonField<ErrorInfo>,
    private val additionalProperties: MutableMap<String, JsonValue>,
) {

    @JsonCreator
    private constructor(
        @JsonProperty("id") @ExcludeMissing id: JsonField<String> = JsonMissing.of(),
        @JsonProperty("sku") @ExcludeMissing sku: JsonField<String> = JsonMissing.of(),
        @JsonProperty("error") @ExcludeMissing error: JsonField<ErrorInfo> = JsonMissing.of(),
    ) : this(id, sku, error, mutableMapOf())

    /**
     * @throws AcmeInvalidDataException if the JSON field has an unexpected type or is unexpectedly
     *   missing or null (e.g. if the server responded with an unexpected value).
     */
    fun id(): String = id.getRequired("id")

    /**
     * The part's stock keeping unit.
     *
     * @throws AcmeInvalidDataException if the JSON field has an unexpected type or is unexpectedly
     *   missing or null (e.g. if the server responded with an unexpected value).
     */
    fun sku(): String = sku.getRequired("sku")

    /**
     * @throws AcmeInvalidDataException if the JSON field has an unexpected type (e.g. if the server
     *   responded with an unexpected value).
     */
    fun error(): Optional<ErrorInfo> = error.getOptional("error")

    /**
     * Returns the raw JSON value of [id].
     *
     * Unlike [id], this method doesn't throw if the JSON field has an unexpected type.
     */
    @JsonProperty("id") @ExcludeMissing fun _id(): JsonField<String> = id

    /**
     * Returns the raw JSON value of [sku].
     *
     * Unlike [sku], this method doesn't throw if the JSON field has an unexpected type.
     */
    @JsonProperty("sku") @ExcludeMissing fun _sku(): JsonField<String> = sku

    /**
     * Returns the raw JSON value of [error].
     *
     * Unlike [error], this method doesn't throw if the JSON field has an unexpected type.
     */
    @JsonProperty("error") @ExcludeMissing fun _error(): JsonField<ErrorInfo> = error

    @JsonAnySetter
    private fun putAdditionalProperty(key: String, value: JsonValue) {
        additionalProperties.put(key, value)
    }

    @JsonAnyGetter
    @ExcludeMissing
    fun _additionalProperties(): Map<String, JsonValue> =
        Collections.unmodifiableMap(additionalProperties)

    fun toBuilder() = Builder().from(this)

    companion object {

        /**
         * Returns a mutable builder for constructing an instance of [PartRetrieveResponse].
         *
         * The following fields are required:
         * ```java
         * .id()
         * .sku()
         * ```
         */
        @JvmStatic fun builder() = Builder()
    }

    /** A builder for [PartRetrieveResponse]. */
    class Builder internal constructor() {

        private var id: JsonField<String>? = null
        private var sku: JsonField<String>? = null
        private var error: JsonField<ErrorInfo> = JsonMissing.of()
        private var additionalProperties: MutableMap<String, JsonValue> = mutableMapOf()

        @JvmSynthetic
        internal fun from(partRetrieveResponse: PartRetrieveResponse) = apply {
            id = partRetrieveResponse.id
            sku = partRetrieveResponse.sku
            error = partRetrieveResponse.error
            additionalProperties = partRetrieveResponse.additionalProperties.toMutableMap()
        }

        fun id(id: String) = id(JsonField.of(id))

        /**
         * Sets [Builder.id] to an arbitrary JSON value.
         *
         * You should usually call [Builder.id] with a well-typed [String] value instead. This
         * method is primarily for setting the field to an undocumented or not yet supported value.
         */
        fun id(id: JsonField<String>) = apply { this.id = id }

        /** The part's stock keeping unit. */
        fun sku(sku: String) = sku(JsonField.of(sku))

        /**
         * Sets [Builder.sku] to an arbitrary JSON value.
         *
         * You should usually call [Builder.sku] with a well-typed [String] value instead. This
         * method is primarily for setting the field to an undocumented or not yet supported value.
         */
        fun sku(sku: JsonField<String>) = apply { this.sku = sku }

        fun error(error: ErrorInfo) = error(JsonField.of(error))

        /**
         * Sets [Builder.error] to an arbitrary JSON value.
         *
         * You should usually call [Builder.error] with a well-typed [ErrorInfo] value instead. This
         * method is primarily for setting the field to an undocumented or not yet supported value.
         */
        fun error(error: JsonField<ErrorInfo>) = apply { this.error = error }

        fun additionalProperties(additionalProperties: Map<String, JsonValue>) = apply {
            this.additionalProperties.clear()
            putAllAdditionalProperties(additionalProperties)
        }

        fun putAdditionalProperty(key: String, value: JsonValue) = apply {
            additionalProperties.put(key, value)
        }

        fun putAllAdditionalProperties(additionalProperties: Map<String, JsonValue>) = apply {
            this.additionalProperties.putAll(additionalProperties)
        }

        fun removeAdditionalProperty(key: String) = apply { additionalProperties.remove(key) }

        fun removeAllAdditionalProperties(keys: Set<String>) = apply {
            keys.forEach(::removeAdditionalProperty)
        }

        /**
         * Returns an immutable instance of [PartRetrieveResponse].
         *
         * Further updates to this [Builder] will not mutate the returned instance.
         *
         * The following fields are required:
         * ```java
         * .id()
         * .sku()
         * ```
         *
         * @throws IllegalStateException if any required field is unset.
         */
        fun build(): PartRetrieveResponse =
            PartRetrieveResponse(
                checkRequired("id", id),
                checkRequired("sku", sku),
                error,
                additionalProperties.toMutableMap(),
            )
    }

    private var validated: Boolean = false

    /**
     * Validates that the types of all values in this object match their expected types recursively.
     *
     * This method is _not_ forwards compatible with new types from the API for existing fields.
     *
     * @throws AcmeInvalidDataException if any value type in this object doesn't match its expected
     *   type.
     */
    fun validate(): PartRetrieveResponse = apply {
        if (validated) {
            return@apply
        }

        id()
        sku()
        error().ifPresent { it.validate() }
        validated = true
    }

    fun isValid(): Boolean =
        try {
            validate()
            true
        } catch (e: AcmeInvalidDataException) {
            false
        }

    /**
     * Returns a score indicating how many valid values are contained in this object recursively.
     *
     * Used for best match union deserialization.
     */
    @JvmSynthetic
    internal fun validity(): Int =
        (if (id.asKnown().isPresent) 1 else 0) +
            (if (sku.asKnown().isPresent) 1 else 0) +
            (error.asKnown().getOrNull()?.validity() ?: 0)

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is PartRetrieveResponse &&
            id == other.id &&
            sku == other.sku &&
            error == other.error &&
            additionalProperties == other.additionalProperties
    }

    private val hashCode: Int by lazy { Objects.hash(id, sku, error, additionalProperties) }

    override fun hashCode(): Int = hashCode

    override fun toString() =
        "PartRetrieveResponse{id=$id, sku=$sku, error=$error, additionalProperties=$additionalProperties}"
}
