// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.models.widgets

import com.fasterxml.jackson.annotation.JsonAnyGetter
import com.fasterxml.jackson.annotation.JsonAnySetter
import com.fasterxml.jackson.annotation.JsonCreator
import com.fasterxml.jackson.annotation.JsonProperty
import java.time.OffsetDateTime
import java.util.Collections
import java.util.Objects
import java.util.Optional
import kotlin.jvm.optionals.getOrNull
import test.acme.api.core.Enum
import test.acme.api.core.ExcludeMissing
import test.acme.api.core.JsonField
import test.acme.api.core.JsonMissing
import test.acme.api.core.JsonValue
import test.acme.api.core.checkKnown
import test.acme.api.core.checkRequired
import test.acme.api.core.toImmutable
import test.acme.api.errors.AcmeInvalidDataException

/** A widget in the catalogue. */
class Widget
@JsonCreator(mode = JsonCreator.Mode.DISABLED)
private constructor(
    private val id: JsonField<String>,
    private val createdAt: JsonField<OffsetDateTime>,
    private val name: JsonField<String>,
    private val status: JsonField<Status>,
    private val homepageUrl: JsonField<String>,
    private val parts: JsonField<List<Part>>,
    private val sku: JsonField<String>,
    private val weightKg: JsonField<Double>,
    private val additionalProperties: MutableMap<String, JsonValue>,
) {

    @JsonCreator
    private constructor(
        @JsonProperty("id") @ExcludeMissing id: JsonField<String> = JsonMissing.of(),
        @JsonProperty("created_at")
        @ExcludeMissing
        createdAt: JsonField<OffsetDateTime> = JsonMissing.of(),
        @JsonProperty("name") @ExcludeMissing name: JsonField<String> = JsonMissing.of(),
        @JsonProperty("status") @ExcludeMissing status: JsonField<Status> = JsonMissing.of(),
        @JsonProperty("homepage_url")
        @ExcludeMissing
        homepageUrl: JsonField<String> = JsonMissing.of(),
        @JsonProperty("parts") @ExcludeMissing parts: JsonField<List<Part>> = JsonMissing.of(),
        @JsonProperty("sku") @ExcludeMissing sku: JsonField<String> = JsonMissing.of(),
        @JsonProperty("weight_kg") @ExcludeMissing weightKg: JsonField<Double> = JsonMissing.of(),
    ) : this(id, createdAt, name, status, homepageUrl, parts, sku, weightKg, mutableMapOf())

    /**
     * @throws AcmeInvalidDataException if the JSON field has an unexpected type or is unexpectedly
     *   missing or null (e.g. if the server responded with an unexpected value).
     */
    fun id(): String = id.getRequired("id")

    /**
     * @throws AcmeInvalidDataException if the JSON field has an unexpected type or is unexpectedly
     *   missing or null (e.g. if the server responded with an unexpected value).
     */
    fun createdAt(): OffsetDateTime = createdAt.getRequired("created_at")

    /**
     * @throws AcmeInvalidDataException if the JSON field has an unexpected type or is unexpectedly
     *   missing or null (e.g. if the server responded with an unexpected value).
     */
    fun name(): String = name.getRequired("name")

    /**
     * Where the widget is in its life cycle.
     *
     * @throws AcmeInvalidDataException if the JSON field has an unexpected type or is unexpectedly
     *   missing or null (e.g. if the server responded with an unexpected value).
     */
    fun status(): Status = status.getRequired("status")

    /**
     * @throws AcmeInvalidDataException if the JSON field has an unexpected type (e.g. if the server
     *   responded with an unexpected value).
     */
    fun homepageUrl(): Optional<String> = homepageUrl.getOptional("homepage_url")

    /**
     * @throws AcmeInvalidDataException if the JSON field has an unexpected type (e.g. if the server
     *   responded with an unexpected value).
     */
    fun parts(): Optional<List<Part>> = parts.getOptional("parts")

    /**
     * @throws AcmeInvalidDataException if the JSON field has an unexpected type (e.g. if the server
     *   responded with an unexpected value).
     */
    fun sku(): Optional<String> = sku.getOptional("sku")

    /**
     * @throws AcmeInvalidDataException if the JSON field has an unexpected type (e.g. if the server
     *   responded with an unexpected value).
     */
    fun weightKg(): Optional<Double> = weightKg.getOptional("weight_kg")

    /**
     * Returns the raw JSON value of [id].
     *
     * Unlike [id], this method doesn't throw if the JSON field has an unexpected type.
     */
    @JsonProperty("id") @ExcludeMissing fun _id(): JsonField<String> = id

    /**
     * Returns the raw JSON value of [createdAt].
     *
     * Unlike [createdAt], this method doesn't throw if the JSON field has an unexpected type.
     */
    @JsonProperty("created_at")
    @ExcludeMissing
    fun _createdAt(): JsonField<OffsetDateTime> = createdAt

    /**
     * Returns the raw JSON value of [name].
     *
     * Unlike [name], this method doesn't throw if the JSON field has an unexpected type.
     */
    @JsonProperty("name") @ExcludeMissing fun _name(): JsonField<String> = name

    /**
     * Returns the raw JSON value of [status].
     *
     * Unlike [status], this method doesn't throw if the JSON field has an unexpected type.
     */
    @JsonProperty("status") @ExcludeMissing fun _status(): JsonField<Status> = status

    /**
     * Returns the raw JSON value of [homepageUrl].
     *
     * Unlike [homepageUrl], this method doesn't throw if the JSON field has an unexpected type.
     */
    @JsonProperty("homepage_url")
    @ExcludeMissing
    fun _homepageUrl(): JsonField<String> = homepageUrl

    /**
     * Returns the raw JSON value of [parts].
     *
     * Unlike [parts], this method doesn't throw if the JSON field has an unexpected type.
     */
    @JsonProperty("parts") @ExcludeMissing fun _parts(): JsonField<List<Part>> = parts

    /**
     * Returns the raw JSON value of [sku].
     *
     * Unlike [sku], this method doesn't throw if the JSON field has an unexpected type.
     */
    @JsonProperty("sku") @ExcludeMissing fun _sku(): JsonField<String> = sku

    /**
     * Returns the raw JSON value of [weightKg].
     *
     * Unlike [weightKg], this method doesn't throw if the JSON field has an unexpected type.
     */
    @JsonProperty("weight_kg") @ExcludeMissing fun _weightKg(): JsonField<Double> = weightKg

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
         * Returns a mutable builder for constructing an instance of [Widget].
         *
         * The following fields are required:
         * ```java
         * .id()
         * .createdAt()
         * .name()
         * .status()
         * ```
         */
        @JvmStatic fun builder() = Builder()
    }

    /** A builder for [Widget]. */
    class Builder internal constructor() {

        private var id: JsonField<String>? = null
        private var createdAt: JsonField<OffsetDateTime>? = null
        private var name: JsonField<String>? = null
        private var status: JsonField<Status>? = null
        private var homepageUrl: JsonField<String> = JsonMissing.of()
        private var parts: JsonField<MutableList<Part>>? = null
        private var sku: JsonField<String> = JsonMissing.of()
        private var weightKg: JsonField<Double> = JsonMissing.of()
        private var additionalProperties: MutableMap<String, JsonValue> = mutableMapOf()

        @JvmSynthetic
        internal fun from(widget: Widget) = apply {
            id = widget.id
            createdAt = widget.createdAt
            name = widget.name
            status = widget.status
            homepageUrl = widget.homepageUrl
            parts = widget.parts.map { it.toMutableList() }
            sku = widget.sku
            weightKg = widget.weightKg
            additionalProperties = widget.additionalProperties.toMutableMap()
        }

        fun id(id: String) = id(JsonField.of(id))

        /**
         * Sets [Builder.id] to an arbitrary JSON value.
         *
         * You should usually call [Builder.id] with a well-typed [String] value instead. This
         * method is primarily for setting the field to an undocumented or not yet supported value.
         */
        fun id(id: JsonField<String>) = apply { this.id = id }

        fun createdAt(createdAt: OffsetDateTime) = createdAt(JsonField.of(createdAt))

        /**
         * Sets [Builder.createdAt] to an arbitrary JSON value.
         *
         * You should usually call [Builder.createdAt] with a well-typed [OffsetDateTime] value
         * instead. This method is primarily for setting the field to an undocumented or not yet
         * supported value.
         */
        fun createdAt(createdAt: JsonField<OffsetDateTime>) = apply { this.createdAt = createdAt }

        fun name(name: String) = name(JsonField.of(name))

        /**
         * Sets [Builder.name] to an arbitrary JSON value.
         *
         * You should usually call [Builder.name] with a well-typed [String] value instead. This
         * method is primarily for setting the field to an undocumented or not yet supported value.
         */
        fun name(name: JsonField<String>) = apply { this.name = name }

        /** Where the widget is in its life cycle. */
        fun status(status: Status) = status(JsonField.of(status))

        /**
         * Sets [Builder.status] to an arbitrary JSON value.
         *
         * You should usually call [Builder.status] with a well-typed [Status] value instead. This
         * method is primarily for setting the field to an undocumented or not yet supported value.
         */
        fun status(status: JsonField<Status>) = apply { this.status = status }

        fun homepageUrl(homepageUrl: String) = homepageUrl(JsonField.of(homepageUrl))

        /**
         * Sets [Builder.homepageUrl] to an arbitrary JSON value.
         *
         * You should usually call [Builder.homepageUrl] with a well-typed [String] value instead.
         * This method is primarily for setting the field to an undocumented or not yet supported
         * value.
         */
        fun homepageUrl(homepageUrl: JsonField<String>) = apply { this.homepageUrl = homepageUrl }

        fun parts(parts: List<Part>) = parts(JsonField.of(parts))

        /**
         * Sets [Builder.parts] to an arbitrary JSON value.
         *
         * You should usually call [Builder.parts] with a well-typed `List<Part>` value instead.
         * This method is primarily for setting the field to an undocumented or not yet supported
         * value.
         */
        fun parts(parts: JsonField<List<Part>>) = apply {
            this.parts = parts.map { it.toMutableList() }
        }

        /**
         * Adds a single [Part] to [parts].
         *
         * @throws IllegalStateException if the field was previously set to a non-list.
         */
        fun addPart(part: Part) = apply {
            parts =
                (parts ?: JsonField.of(mutableListOf())).also { checkKnown("parts", it).add(part) }
        }

        fun sku(sku: String?) = sku(JsonField.ofNullable(sku))

        /** Alias for calling [Builder.sku] with `sku.orElse(null)`. */
        fun sku(sku: Optional<String>) = sku(sku.getOrNull())

        /**
         * Sets [Builder.sku] to an arbitrary JSON value.
         *
         * You should usually call [Builder.sku] with a well-typed [String] value instead. This
         * method is primarily for setting the field to an undocumented or not yet supported value.
         */
        fun sku(sku: JsonField<String>) = apply { this.sku = sku }

        fun weightKg(weightKg: Double) = weightKg(JsonField.of(weightKg))

        /**
         * Sets [Builder.weightKg] to an arbitrary JSON value.
         *
         * You should usually call [Builder.weightKg] with a well-typed [Double] value instead. This
         * method is primarily for setting the field to an undocumented or not yet supported value.
         */
        fun weightKg(weightKg: JsonField<Double>) = apply { this.weightKg = weightKg }

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
         * Returns an immutable instance of [Widget].
         *
         * Further updates to this [Builder] will not mutate the returned instance.
         *
         * The following fields are required:
         * ```java
         * .id()
         * .createdAt()
         * .name()
         * .status()
         * ```
         *
         * @throws IllegalStateException if any required field is unset.
         */
        fun build(): Widget =
            Widget(
                checkRequired("id", id),
                checkRequired("createdAt", createdAt),
                checkRequired("name", name),
                checkRequired("status", status),
                homepageUrl,
                (parts ?: JsonMissing.of()).map { it.toImmutable() },
                sku,
                weightKg,
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
    fun validate(): Widget = apply {
        if (validated) {
            return@apply
        }

        id()
        createdAt()
        name()
        status().validate()
        homepageUrl()
        parts().ifPresent { it.forEach { it.validate() } }
        sku()
        weightKg()
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
            (if (createdAt.asKnown().isPresent) 1 else 0) +
            (if (name.asKnown().isPresent) 1 else 0) +
            (status.asKnown().getOrNull()?.validity() ?: 0) +
            (if (homepageUrl.asKnown().isPresent) 1 else 0) +
            (parts.asKnown().getOrNull()?.sumOf { it.validity().toInt() } ?: 0) +
            (if (sku.asKnown().isPresent) 1 else 0) +
            (if (weightKg.asKnown().isPresent) 1 else 0)

    /** Where the widget is in its life cycle. */
    class Status @JsonCreator private constructor(private val value: JsonField<String>) : Enum {

        /**
         * Returns this class instance's raw value.
         *
         * This is usually only useful if this instance was deserialized from data that doesn't
         * match any known member, and you want to know that value. For example, if the SDK is on an
         * older version than the API, then the API may respond with new members that the SDK is
         * unaware of.
         */
        @com.fasterxml.jackson.annotation.JsonValue fun _value(): JsonField<String> = value

        companion object {

            @JvmField val DRAFT = of("draft")

            @JvmField val ACTIVE = of("active")

            @JvmField val RETIRED = of("retired")

            @JvmStatic fun of(value: String) = Status(JsonField.of(value))
        }

        /** An enum containing [Status]'s known values. */
        enum class Known {
            DRAFT,
            ACTIVE,
            RETIRED,
        }

        /**
         * An enum containing [Status]'s known values, as well as an [_UNKNOWN] member.
         *
         * An instance of [Status] can contain an unknown value in a couple of cases:
         * - It was deserialized from data that doesn't match any known member. For example, if the
         *   SDK is on an older version than the API, then the API may respond with new members that
         *   the SDK is unaware of.
         * - It was constructed with an arbitrary value using the [of] method.
         */
        enum class Value {
            DRAFT,
            ACTIVE,
            RETIRED,
            /** An enum member indicating that [Status] was instantiated with an unknown value. */
            _UNKNOWN,
        }

        /**
         * Returns an enum member corresponding to this class instance's value, or [Value._UNKNOWN]
         * if the class was instantiated with an unknown value.
         *
         * Use the [known] method instead if you're certain the value is always known or if you want
         * to throw for the unknown case.
         */
        fun value(): Value =
            when (this) {
                DRAFT -> Value.DRAFT
                ACTIVE -> Value.ACTIVE
                RETIRED -> Value.RETIRED
                else -> Value._UNKNOWN
            }

        /**
         * Returns an enum member corresponding to this class instance's value.
         *
         * Use the [value] method instead if you're uncertain the value is always known and don't
         * want to throw for the unknown case.
         *
         * @throws AcmeInvalidDataException if this class instance's value is a not a known member.
         */
        fun known(): Known =
            when (this) {
                DRAFT -> Known.DRAFT
                ACTIVE -> Known.ACTIVE
                RETIRED -> Known.RETIRED
                else -> throw AcmeInvalidDataException("Unknown Status: $value")
            }

        /**
         * Returns this class instance's primitive wire representation.
         *
         * This differs from the [toString] method because that method is primarily for debugging
         * and generally doesn't throw.
         *
         * @throws AcmeInvalidDataException if this class instance's value does not have the
         *   expected primitive type.
         */
        fun asString(): String =
            _value().asString().orElseThrow { AcmeInvalidDataException("Value is not a String") }

        private var validated: Boolean = false

        /**
         * Validates that the types of all values in this object match their expected types
         * recursively.
         *
         * This method is _not_ forwards compatible with new types from the API for existing fields.
         *
         * @throws AcmeInvalidDataException if any value type in this object doesn't match its
         *   expected type.
         */
        fun validate(): Status = apply {
            if (validated) {
                return@apply
            }

            known()
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
         * Returns a score indicating how many valid values are contained in this object
         * recursively.
         *
         * Used for best match union deserialization.
         */
        @JvmSynthetic internal fun validity(): Int = if (value() == Value._UNKNOWN) 0 else 1

        override fun equals(other: Any?): Boolean {
            if (this === other) {
                return true
            }

            return other is Status && value == other.value
        }

        override fun hashCode() = value.hashCode()

        override fun toString() = value.toString()
    }

    class Part
    @JsonCreator(mode = JsonCreator.Mode.DISABLED)
    private constructor(
        private val label: JsonField<String>,
        private val partNumber: JsonField<Long>,
        private val additionalProperties: MutableMap<String, JsonValue>,
    ) {

        @JsonCreator
        private constructor(
            @JsonProperty("label") @ExcludeMissing label: JsonField<String> = JsonMissing.of(),
            @JsonProperty("part_number")
            @ExcludeMissing
            partNumber: JsonField<Long> = JsonMissing.of(),
        ) : this(label, partNumber, mutableMapOf())

        /**
         * @throws AcmeInvalidDataException if the JSON field has an unexpected type (e.g. if the
         *   server responded with an unexpected value).
         */
        fun label(): Optional<String> = label.getOptional("label")

        /**
         * @throws AcmeInvalidDataException if the JSON field has an unexpected type (e.g. if the
         *   server responded with an unexpected value).
         */
        fun partNumber(): Optional<Long> = partNumber.getOptional("part_number")

        /**
         * Returns the raw JSON value of [label].
         *
         * Unlike [label], this method doesn't throw if the JSON field has an unexpected type.
         */
        @JsonProperty("label") @ExcludeMissing fun _label(): JsonField<String> = label

        /**
         * Returns the raw JSON value of [partNumber].
         *
         * Unlike [partNumber], this method doesn't throw if the JSON field has an unexpected type.
         */
        @JsonProperty("part_number") @ExcludeMissing fun _partNumber(): JsonField<Long> = partNumber

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

            /** Returns a mutable builder for constructing an instance of [Part]. */
            @JvmStatic fun builder() = Builder()
        }

        /** A builder for [Part]. */
        class Builder internal constructor() {

            private var label: JsonField<String> = JsonMissing.of()
            private var partNumber: JsonField<Long> = JsonMissing.of()
            private var additionalProperties: MutableMap<String, JsonValue> = mutableMapOf()

            @JvmSynthetic
            internal fun from(part: Part) = apply {
                label = part.label
                partNumber = part.partNumber
                additionalProperties = part.additionalProperties.toMutableMap()
            }

            fun label(label: String) = label(JsonField.of(label))

            /**
             * Sets [Builder.label] to an arbitrary JSON value.
             *
             * You should usually call [Builder.label] with a well-typed [String] value instead.
             * This method is primarily for setting the field to an undocumented or not yet
             * supported value.
             */
            fun label(label: JsonField<String>) = apply { this.label = label }

            fun partNumber(partNumber: Long) = partNumber(JsonField.of(partNumber))

            /**
             * Sets [Builder.partNumber] to an arbitrary JSON value.
             *
             * You should usually call [Builder.partNumber] with a well-typed [Long] value instead.
             * This method is primarily for setting the field to an undocumented or not yet
             * supported value.
             */
            fun partNumber(partNumber: JsonField<Long>) = apply { this.partNumber = partNumber }

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
             * Returns an immutable instance of [Part].
             *
             * Further updates to this [Builder] will not mutate the returned instance.
             */
            fun build(): Part = Part(label, partNumber, additionalProperties.toMutableMap())
        }

        private var validated: Boolean = false

        /**
         * Validates that the types of all values in this object match their expected types
         * recursively.
         *
         * This method is _not_ forwards compatible with new types from the API for existing fields.
         *
         * @throws AcmeInvalidDataException if any value type in this object doesn't match its
         *   expected type.
         */
        fun validate(): Part = apply {
            if (validated) {
                return@apply
            }

            label()
            partNumber()
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
         * Returns a score indicating how many valid values are contained in this object
         * recursively.
         *
         * Used for best match union deserialization.
         */
        @JvmSynthetic
        internal fun validity(): Int =
            (if (label.asKnown().isPresent) 1 else 0) +
                (if (partNumber.asKnown().isPresent) 1 else 0)

        override fun equals(other: Any?): Boolean {
            if (this === other) {
                return true
            }

            return other is Part &&
                label == other.label &&
                partNumber == other.partNumber &&
                additionalProperties == other.additionalProperties
        }

        private val hashCode: Int by lazy { Objects.hash(label, partNumber, additionalProperties) }

        override fun hashCode(): Int = hashCode

        override fun toString() =
            "Part{label=$label, partNumber=$partNumber, additionalProperties=$additionalProperties}"
    }

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is Widget &&
            id == other.id &&
            createdAt == other.createdAt &&
            name == other.name &&
            status == other.status &&
            homepageUrl == other.homepageUrl &&
            parts == other.parts &&
            sku == other.sku &&
            weightKg == other.weightKg &&
            additionalProperties == other.additionalProperties
    }

    private val hashCode: Int by lazy {
        Objects.hash(
            id,
            createdAt,
            name,
            status,
            homepageUrl,
            parts,
            sku,
            weightKg,
            additionalProperties,
        )
    }

    override fun hashCode(): Int = hashCode

    override fun toString() =
        "Widget{id=$id, createdAt=$createdAt, name=$name, status=$status, homepageUrl=$homepageUrl, parts=$parts, sku=$sku, weightKg=$weightKg, additionalProperties=$additionalProperties}"
}
