package __ferrule_pkg__.core

import com.fasterxml.jackson.annotation.JacksonAnnotationsInside
import com.fasterxml.jackson.annotation.JsonCreator
import com.fasterxml.jackson.annotation.JsonInclude
import com.fasterxml.jackson.core.JsonGenerator
import com.fasterxml.jackson.core.ObjectCodec
import com.fasterxml.jackson.core.type.TypeReference
import com.fasterxml.jackson.databind.BeanProperty
import com.fasterxml.jackson.databind.DeserializationContext
import com.fasterxml.jackson.databind.JavaType
import com.fasterxml.jackson.databind.JsonDeserializer
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.SerializerProvider
import com.fasterxml.jackson.databind.annotation.JsonDeserialize
import com.fasterxml.jackson.databind.annotation.JsonSerialize
import com.fasterxml.jackson.databind.node.JsonNodeType.ARRAY
import com.fasterxml.jackson.databind.node.JsonNodeType.BINARY
import com.fasterxml.jackson.databind.node.JsonNodeType.BOOLEAN
import com.fasterxml.jackson.databind.node.JsonNodeType.MISSING
import com.fasterxml.jackson.databind.node.JsonNodeType.NULL
import com.fasterxml.jackson.databind.node.JsonNodeType.NUMBER
import com.fasterxml.jackson.databind.node.JsonNodeType.OBJECT
import com.fasterxml.jackson.databind.node.JsonNodeType.POJO
import com.fasterxml.jackson.databind.node.JsonNodeType.STRING
import com.fasterxml.jackson.databind.ser.std.NullSerializer
//ferrule:if kotlin
import com.fasterxml.jackson.module.kotlin.jacksonTypeRef
//ferrule:endif
import java.io.InputStream
//ferrule:if kotlin
import java.util.*
import kotlin.reflect.KClass
//ferrule:else
import java.util.Objects
import java.util.Optional
//ferrule:endif
import __ferrule_pkg__.errors.__FerruleClient__InvalidDataException

/**
 * A class representing a serializable JSON field.
 *
 * It can either be a [KnownValue] value of type [T], matching the type the SDK expects, or an
 * arbitrary JSON value that bypasses the type system (via [JsonValue]).
 */
@JsonDeserialize(using = JsonField.Deserializer::class)
sealed class JsonField<out T : Any> {

    /**
     * Returns whether this field is missing, which means it will be omitted from the serialized
     * JSON entirely.
     */
    fun isMissing(): Boolean = this is JsonMissing

    /** Whether this field is explicitly set to `null`. */
    fun isNull(): Boolean = this is JsonNull

    /**
//ferrule:if kotlin
     * Returns this field's "known" value, meaning it matches the type the SDK expects, or null if
     * this field contains an arbitrary [JsonValue].
//ferrule:else
     * Returns an [Optional] containing this field's "known" value, meaning it matches the type the
     * SDK expects, or an empty [Optional] if this field contains an arbitrary [JsonValue].
//ferrule:endif
     *
     * This is the opposite of [asUnknown].
     */
//ferrule:if kotlin
    fun asKnown(): T? = (this as? KnownValue)?.value
//ferrule:else
    fun asKnown():
        Optional<
            // Safe because `Optional` is effectively covariant, but Kotlin doesn't know that.
            @UnsafeVariance
            T
        > = Optional.ofNullable((this as? KnownValue)?.value)
//ferrule:endif

    /**
//ferrule:if kotlin
     * Returns this field's arbitrary [JsonValue], meaning it mismatches the type the SDK expects,
     * or null if this field contains a "known" value.
//ferrule:else
     * Returns an [Optional] containing this field's arbitrary [JsonValue], meaning it mismatches
     * the type the SDK expects, or an empty [Optional] if this field contains a "known" value.
//ferrule:endif
     *
     * This is the opposite of [asKnown].
     */
//ferrule:if kotlin
    fun asUnknown(): JsonValue? = this as? JsonValue
//ferrule:else
    fun asUnknown(): Optional<JsonValue> = Optional.ofNullable(this as? JsonValue)
//ferrule:endif

    /**
//ferrule:if kotlin
     * Returns this field's boolean value, or null if it doesn't contain a boolean.
//ferrule:else
     * Returns an [Optional] containing this field's boolean value, or an empty [Optional] if it
     * doesn't contain a boolean.
//ferrule:endif
     *
     * This method checks for both a [KnownValue] containing a boolean and for [JsonBoolean].
     */
//ferrule:if kotlin
    fun asBoolean(): Boolean? =
//ferrule:else
    fun asBoolean(): Optional<Boolean> =
//ferrule:endif
        when (this) {
//ferrule:if kotlin
            is JsonBoolean -> value
            is KnownValue -> value as? Boolean
            else -> null
//ferrule:else
            is JsonBoolean -> Optional.of(value)
            is KnownValue -> Optional.ofNullable(value as? Boolean)
            else -> Optional.empty()
//ferrule:endif
        }

    /**
//ferrule:if kotlin
     * Returns this field's numerical value, or null if it doesn't contain a number.
//ferrule:else
     * Returns an [Optional] containing this field's numerical value, or an empty [Optional] if it
     * doesn't contain a number.
//ferrule:endif
     *
     * This method checks for both a [KnownValue] containing a number and for [JsonNumber].
     */
//ferrule:if kotlin
    fun asNumber(): Number? =
//ferrule:else
    fun asNumber(): Optional<Number> =
//ferrule:endif
        when (this) {
//ferrule:if kotlin
            is JsonNumber -> value
            is KnownValue -> value as? Number
            else -> null
//ferrule:else
            is JsonNumber -> Optional.of(value)
            is KnownValue -> Optional.ofNullable(value as? Number)
            else -> Optional.empty()
//ferrule:endif
        }

    /**
//ferrule:if kotlin
     * Returns this field's string value, or null if it doesn't contain a string.
//ferrule:else
     * Returns an [Optional] containing this field's string value, or an empty [Optional] if it
     * doesn't contain a string.
//ferrule:endif
     *
     * This method checks for both a [KnownValue] containing a string and for [JsonString].
     */
//ferrule:if kotlin
    fun asString(): String? =
//ferrule:else
    fun asString(): Optional<String> =
//ferrule:endif
        when (this) {
//ferrule:if kotlin
            is JsonString -> value
            is KnownValue -> value as? String
            else -> null
//ferrule:else
            is JsonString -> Optional.of(value)
            is KnownValue -> Optional.ofNullable(value as? String)
            else -> Optional.empty()
//ferrule:endif
        }

    fun asStringOrThrow(): String =
//ferrule:if kotlin
        asString() ?: throw __FerruleClient__InvalidDataException("Value is not a string")
//ferrule:else
        asString().orElseThrow { __FerruleClient__InvalidDataException("Value is not a string") }
//ferrule:endif

    /**
//ferrule:if kotlin
     * Returns this field's list value, or null if it doesn't contain a list.
//ferrule:else
     * Returns an [Optional] containing this field's list value, or an empty [Optional] if it
     * doesn't contain a list.
//ferrule:endif
     *
     * This method checks for both a [KnownValue] containing a list and for [JsonArray].
     */
//ferrule:if kotlin
    fun asArray(): List<JsonValue>? =
//ferrule:else
    fun asArray(): Optional<List<JsonValue>> =
//ferrule:endif
        when (this) {
//ferrule:if kotlin
            is JsonArray -> values
//ferrule:else
            is JsonArray -> Optional.of(values)
//ferrule:endif
            is KnownValue ->
//ferrule:if kotlin
                (value as? List<*>)?.map {
                    try {
                        JsonValue.from(it)
                    } catch (e: IllegalArgumentException) {
                        // The known value is a list, but not all items are convertible to
                        // `JsonValue`.
                        return null
//ferrule:else
                Optional.ofNullable(
                    (value as? List<*>)?.map {
                        try {
                            JsonValue.from(it)
                        } catch (e: IllegalArgumentException) {
                            // The known value is a list, but not all values are convertible to
                            // `JsonValue`.
                            return Optional.empty()
                        }
//ferrule:endif
                    }
//ferrule:if kotlin
                }
            else -> null
//ferrule:else
                )
            else -> Optional.empty()
//ferrule:endif
        }

    /**
//ferrule:if kotlin
     * Returns this field's map value, or null if it doesn't contain a map.
//ferrule:else
     * Returns an [Optional] containing this field's map value, or an empty [Optional] if it doesn't
     * contain a map.
//ferrule:endif
     *
     * This method checks for both a [KnownValue] containing a map and for [JsonObject].
     */
//ferrule:if kotlin
    fun asObject(): Map<String, JsonValue>? =
//ferrule:else
    fun asObject(): Optional<Map<String, JsonValue>> =
//ferrule:endif
        when (this) {
//ferrule:if kotlin
            is JsonObject -> values
//ferrule:else
            is JsonObject -> Optional.of(values)
//ferrule:endif
            is KnownValue ->
//ferrule:if kotlin
                (value as? Map<*, *>)
                    ?.map { (key, value) ->
                        if (key !is String) {
                            return null
                        }

                        val jsonValue =
                            try {
                                JsonValue.from(value)
                            } catch (e: IllegalArgumentException) {
                                // The known value is a map, but not all values are convertible to
                                // `JsonValue`.
                                return null
//ferrule:else
                Optional.ofNullable(
                    (value as? Map<*, *>)
                        ?.map { (key, value) ->
                            if (key !is String) {
                                return Optional.empty()
//ferrule:endif
                            }

//ferrule:if kotlin
                        key to jsonValue
                    }
                    ?.toMap()
            else -> null
//ferrule:else
                            val jsonValue =
                                try {
                                    JsonValue.from(value)
                                } catch (e: IllegalArgumentException) {
                                    // The known value is a map, but not all items are convertible
                                    // to `JsonValue`.
                                    return Optional.empty()
                                }

                            key to jsonValue
                        }
                        ?.toMap()
                )
            else -> Optional.empty()
//ferrule:endif
        }

//ferrule:if java
    @JvmSynthetic
//ferrule:endif
    internal fun getRequired(name: String): T =
        when (this) {
            is KnownValue -> value
            is JsonMissing -> throw __FerruleClient__InvalidDataException("`$name` is not set")
            is JsonNull -> throw __FerruleClient__InvalidDataException("`$name` is null")
            else -> throw __FerruleClient__InvalidDataException("`$name` is invalid, received $this")
        }

//ferrule:if kotlin
    internal fun getNullable(name: String): T? =
//ferrule:else
    @JvmSynthetic
    internal fun getOptional(
        name: String
    ): Optional<
        // Safe because `Optional` is effectively covariant, but Kotlin doesn't know that.
        @UnsafeVariance
        T
    > =
//ferrule:endif
        when (this) {
//ferrule:if kotlin
            is KnownValue -> value
            is JsonMissing -> null
            is JsonNull -> null
//ferrule:else
            is KnownValue -> Optional.of(value)
            is JsonMissing,
            is JsonNull -> Optional.empty()
//ferrule:endif
            else -> throw __FerruleClient__InvalidDataException("`$name` is invalid, received $this")
        }

//ferrule:if java
    @JvmSynthetic
//ferrule:endif
    internal fun <R : Any> map(transform: (T) -> R): JsonField<R> =
        when (this) {
            is KnownValue -> KnownValue.of(transform(value))
            is JsonValue -> this
        }

//ferrule:if kotlin
    internal fun accept(consume: (T) -> Unit) {
        asKnown()?.let(consume)
    }
//ferrule:else
    @JvmSynthetic internal fun accept(consume: (T) -> Unit) = asKnown().ifPresent(consume)
//ferrule:endif

    /** Returns the result of calling the [visitor] method corresponding to this field's state. */
    fun <R> accept(visitor: Visitor<T, R>): R =
        when (this) {
            is KnownValue -> visitor.visitKnown(value)
            is JsonValue -> accept(visitor as JsonValue.Visitor<R>)
        }

    /**
     * An interface that defines how to map each possible state of a `JsonField<T>` to a value of
     * type [R].
     */
    interface Visitor<in T, out R> : JsonValue.Visitor<R> {

        fun visitKnown(value: T): R = visitDefault()
    }

    companion object {

        /** Returns a [JsonField] containing the given "known" [value]. */
//ferrule:if kotlin
        fun <T : Any> of(value: T): JsonField<T> = KnownValue.of(value)
//ferrule:else
        @JvmStatic fun <T : Any> of(value: T): JsonField<T> = KnownValue.of(value)
//ferrule:endif

        /**
         * Returns a [JsonField] containing the given "known" [value], or [JsonNull] if [value] is
         * null.
         */
//ferrule:if java
        @JvmStatic
//ferrule:endif
        fun <T : Any> ofNullable(value: T?): JsonField<T> =
            when (value) {
                null -> JsonNull.of()
                else -> KnownValue.of(value)
            }
    }

    /**
     * This class is a Jackson filter that can be used to exclude missing properties from objects.
     * This filter should not be used directly and should instead use the @ExcludeMissing
     * annotation.
     */
    class IsMissing {

        override fun equals(other: Any?): Boolean = other is JsonMissing

        override fun hashCode(): Int = Objects.hash()
    }

    class Deserializer(private val type: JavaType? = null) :
        BaseDeserializer<JsonField<*>>(JsonField::class) {

        override fun createContextual(
            context: DeserializationContext,
            property: BeanProperty?,
        ): JsonDeserializer<JsonField<*>> = Deserializer(context.contextualType?.containedType(0))

        override fun ObjectCodec.deserialize(node: JsonNode): JsonField<*> =
            type?.let { tryDeserialize<Any>(node, type) }?.let { of(it) }
                ?: JsonValue.fromJsonNode(node)

        override fun getNullValue(context: DeserializationContext): JsonField<*> = JsonNull.of()
    }
}

/**
 * A class representing an arbitrary JSON value.
 *
 * It is immutable and assignable to any [JsonField], regardless of its expected type (i.e. its
 * generic type argument).
 */
@JsonDeserialize(using = JsonValue.Deserializer::class)
sealed class JsonValue : JsonField<Nothing>() {

//ferrule:if kotlin
    inline fun <reified R : Any> convert(): R? = convert(jacksonTypeRef())

//ferrule:endif
    fun <R : Any> convert(type: TypeReference<R>): R? = JSON_MAPPER.convertValue(this, type)

//ferrule:if kotlin
    fun <R : Any> convert(type: KClass<R>): R? = JSON_MAPPER.convertValue(this, type.java)
//ferrule:else
    fun <R : Any> convert(type: Class<R>): R? = JSON_MAPPER.convertValue(this, type)
//ferrule:endif

    /** Returns the result of calling the [visitor] method corresponding to this value's variant. */
    fun <R> accept(visitor: Visitor<R>): R =
        when (this) {
            is JsonMissing -> visitor.visitMissing()
            is JsonNull -> visitor.visitNull()
            is JsonBoolean -> visitor.visitBoolean(value)
            is JsonNumber -> visitor.visitNumber(value)
            is JsonString -> visitor.visitString(value)
            is JsonArray -> visitor.visitArray(values)
            is JsonObject -> visitor.visitObject(values)
        }

    /**
     * An interface that defines how to map each variant state of a [JsonValue] to a value of type
     * [R].
     */
    interface Visitor<out R> {

        fun visitNull(): R = visitDefault()

        fun visitMissing(): R = visitDefault()

        fun visitBoolean(value: Boolean): R = visitDefault()

        fun visitNumber(value: Number): R = visitDefault()

        fun visitString(value: String): R = visitDefault()

        fun visitArray(values: List<JsonValue>): R = visitDefault()

        fun visitObject(values: Map<String, JsonValue>): R = visitDefault()

        /**
         * The default implementation for unimplemented visitor methods.
         *
         * @throws IllegalArgumentException in the default implementation.
         */
        fun visitDefault(): R = throw IllegalArgumentException("Unexpected value")
    }

    companion object {

        private val JSON_MAPPER = jsonMapper()

        /**
         * Converts the given [value] to a [JsonValue].
         *
         * This method works best on primitive types, [List] values, [Map] values, and nested
         * combinations of these. For example:
//ferrule:if kotlin
         * ```kotlin
//ferrule:else
         * ```java
//ferrule:endif
         * // Create primitive JSON values
//ferrule:if kotlin
         * val nullValue: JsonValue = JsonValue.from(null)
         * val booleanValue: JsonValue = JsonValue.from(true)
         * val numberValue: JsonValue = JsonValue.from(42)
         * val stringValue: JsonValue = JsonValue.from("Hello World!")
//ferrule:else
         * JsonValue nullValue = JsonValue.from(null);
         * JsonValue booleanValue = JsonValue.from(true);
         * JsonValue numberValue = JsonValue.from(42);
         * JsonValue stringValue = JsonValue.from("Hello World!");
//ferrule:endif
         *
         * // Create a JSON array value equivalent to `["Hello", "World"]`
//ferrule:if kotlin
         * val arrayValue: JsonValue = JsonValue.from(listOf("Hello", "World"))
//ferrule:else
         * JsonValue arrayValue = JsonValue.from(List.of("Hello", "World"));
//ferrule:endif
         *
         * // Create a JSON object value equivalent to `{ "a": 1, "b": 2 }`
//ferrule:if kotlin
         * val objectValue: JsonValue = JsonValue.from(mapOf(
         *   "a" to 1,
         *   "b" to 2,
         * ))
//ferrule:else
         * JsonValue objectValue = JsonValue.from(Map.of(
         *   "a", 1,
         *   "b", 2
         * ));
//ferrule:endif
         *
         * // Create an arbitrarily nested JSON equivalent to:
         * // {
         * //   "a": [1, 2],
         * //   "b": [3, 4]
         * // }
//ferrule:if kotlin
         * val complexValue: JsonValue = JsonValue.from(mapOf(
         *   "a" to listOf(1, 2),
         *   "b" to listOf(3, 4),
         * ))
//ferrule:else
         * JsonValue complexValue = JsonValue.from(Map.of(
         *   "a", List.of(1, 2),
         *   "b", List.of(3, 4)
         * ));
//ferrule:endif
         * ```
         *
         * @throws IllegalArgumentException if [value] is not JSON serializable.
         */
//ferrule:if java
        @JvmStatic
//ferrule:endif
        fun from(value: Any?): JsonValue =
            when (value) {
                null -> JsonNull.of()
                is JsonValue -> value
                else -> JSON_MAPPER.convertValue(value, JsonValue::class.java)
            }

        /**
         * Returns a [JsonValue] converted from the given Jackson [JsonNode].
         *
         * @throws IllegalStateException for unsupported node types.
         */
//ferrule:if java
        @JvmStatic
//ferrule:endif
        fun fromJsonNode(node: JsonNode): JsonValue =
            when (node.nodeType) {
                MISSING -> JsonMissing.of()
                NULL -> JsonNull.of()
                BOOLEAN -> JsonBoolean.of(node.booleanValue())
                NUMBER -> JsonNumber.of(node.numberValue())
                STRING -> JsonString.of(node.textValue())
                ARRAY ->
                    JsonArray.of(node.elements().asSequence().map { fromJsonNode(it) }.toList())
                OBJECT ->
                    JsonObject.of(
                        node.fields().asSequence().map { it.key to fromJsonNode(it.value) }.toMap()
                    )
                BINARY,
                POJO,
                null -> throw IllegalStateException("Unexpected JsonNode type: ${node.nodeType}")
            }
    }

    class Deserializer : BaseDeserializer<JsonValue>(JsonValue::class) {

        override fun ObjectCodec.deserialize(node: JsonNode): JsonValue = fromJsonNode(node)

        override fun getNullValue(context: DeserializationContext?): JsonValue = JsonNull.of()
    }
}

/**
 * A class representing a "known" JSON serializable value of type [T], matching the type the SDK
 * expects.
 *
 * It is assignable to `JsonField<T>`.
 */
class KnownValue<T : Any>
private constructor(
    @com.fasterxml.jackson.annotation.JsonValue @get:JvmName("value") val value: T
) : JsonField<T>() {

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is KnownValue<*> && value contentEquals other.value
    }

    override fun hashCode() = contentHash(value)

    override fun toString() = value.contentToString()

    companion object {

        /** Returns a [KnownValue] containing the given [value]. */
//ferrule:if kotlin
        @JsonCreator fun <T : Any> of(value: T) = KnownValue(value)
//ferrule:else
        @JsonCreator @JvmStatic fun <T : Any> of(value: T) = KnownValue(value)
//ferrule:endif
    }
}

/**
 * A [JsonValue] representing an omitted JSON field.
 *
 * An instance of this class will cause a JSON field to be omitted from the serialized JSON
 * entirely.
 */
@JsonSerialize(using = JsonMissing.Serializer::class)
class JsonMissing : JsonValue() {

    override fun toString() = ""

    companion object {

        private val INSTANCE: JsonMissing = JsonMissing()

        /** Returns the singleton instance of [JsonMissing]. */
//ferrule:if kotlin
        fun of() = INSTANCE
//ferrule:else
        @JvmStatic fun of() = INSTANCE
//ferrule:endif
    }

    class Serializer : BaseSerializer<JsonMissing>(JsonMissing::class) {

        override fun serialize(
            value: JsonMissing,
            generator: JsonGenerator,
            provider: SerializerProvider,
        ) {
            throw IllegalStateException("JsonMissing cannot be serialized")
        }
    }
}

/** A [JsonValue] representing a JSON `null` value. */
@JsonSerialize(using = NullSerializer::class)
class JsonNull : JsonValue() {

    override fun toString() = "null"

    companion object {

        private val INSTANCE: JsonNull = JsonNull()

        /** Returns the singleton instance of [JsonMissing]. */
//ferrule:if kotlin
        @JsonCreator fun of() = INSTANCE
//ferrule:else
        @JsonCreator @JvmStatic fun of() = INSTANCE
//ferrule:endif
    }
}

/** A [JsonValue] representing a JSON boolean value. */
class JsonBoolean
private constructor(
    @get:com.fasterxml.jackson.annotation.JsonValue @get:JvmName("value") val value: Boolean
) : JsonValue() {

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is JsonBoolean && value == other.value
    }

    override fun hashCode() = value.hashCode()

    override fun toString() = value.toString()

    companion object {

        /** Returns a [JsonBoolean] containing the given [value]. */
//ferrule:if kotlin
        @JsonCreator fun of(value: Boolean) = JsonBoolean(value)
//ferrule:else
        @JsonCreator @JvmStatic fun of(value: Boolean) = JsonBoolean(value)
//ferrule:endif
    }
}

/** A [JsonValue] representing a JSON number value. */
class JsonNumber
private constructor(
    @get:com.fasterxml.jackson.annotation.JsonValue @get:JvmName("value") val value: Number
) : JsonValue() {

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is JsonNumber && value == other.value
    }

    override fun hashCode() = value.hashCode()

    override fun toString() = value.toString()

    companion object {

        /** Returns a [JsonNumber] containing the given [value]. */
//ferrule:if kotlin
        @JsonCreator fun of(value: Number) = JsonNumber(value)
//ferrule:else
        @JsonCreator @JvmStatic fun of(value: Number) = JsonNumber(value)
//ferrule:endif
    }
}

/** A [JsonValue] representing a JSON string value. */
class JsonString
private constructor(
    @get:com.fasterxml.jackson.annotation.JsonValue @get:JvmName("value") val value: String
) : JsonValue() {

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is JsonString && value == other.value
    }

    override fun hashCode() = value.hashCode()

    override fun toString() = value

    companion object {

        /** Returns a [JsonString] containing the given [value]. */
//ferrule:if kotlin
        @JsonCreator fun of(value: String) = JsonString(value)
//ferrule:else
        @JsonCreator @JvmStatic fun of(value: String) = JsonString(value)
//ferrule:endif
    }
}

/** A [JsonValue] representing a JSON array value. */
class JsonArray
private constructor(
    @get:com.fasterxml.jackson.annotation.JsonValue
    @get:JvmName("values")
    val values: List<JsonValue>
) : JsonValue() {

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is JsonArray && values == other.values
    }

    override fun hashCode() = values.hashCode()

    override fun toString() = values.toString()

    companion object {

        /** Returns a [JsonArray] containing the given [values]. */
//ferrule:if kotlin
        @JsonCreator fun of(values: List<JsonValue>) = JsonArray(values.toImmutable())
//ferrule:else
        @JsonCreator @JvmStatic fun of(values: List<JsonValue>) = JsonArray(values.toImmutable())
//ferrule:endif
    }
}

/** A [JsonValue] representing a JSON object value. */
class JsonObject
private constructor(
    @get:com.fasterxml.jackson.annotation.JsonValue
    @get:JvmName("values")
    val values: Map<String, JsonValue>
) : JsonValue() {

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is JsonObject && values == other.values
    }

    override fun hashCode() = values.hashCode()

    override fun toString() = values.toString()

    companion object {

        /** Returns a [JsonObject] containing the given [values]. */
//ferrule:if kotlin
        @JsonCreator fun of(values: Map<String, JsonValue>) = JsonObject(values.toImmutable())
//ferrule:else
        @JsonCreator
        @JvmStatic
        fun of(values: Map<String, JsonValue>) = JsonObject(values.toImmutable())
//ferrule:endif
    }
}

/** A Jackson annotation for excluding fields set to [JsonMissing] from the serialized JSON. */
@JacksonAnnotationsInside
@JsonInclude(JsonInclude.Include.CUSTOM, valueFilter = JsonField.IsMissing::class)
annotation class ExcludeMissing

/** A class representing a field in a `multipart/form-data` request. */
class MultipartField<T : Any>
private constructor(
    /** A [JsonField] value, which will be serialized to zero or more parts. */
//ferrule:if kotlin
    @get:com.fasterxml.jackson.annotation.JsonValue val value: JsonField<T>,
//ferrule:else
    @get:com.fasterxml.jackson.annotation.JsonValue @get:JvmName("value") val value: JsonField<T>,
//ferrule:endif
    /** A content type for the serialized parts. */
//ferrule:if kotlin
    val contentType: String,
    /** Returns the filename directive that will be included in the serialized field. */
    val filename: String?,
//ferrule:else
    @get:JvmName("contentType") val contentType: String,
    private val filename: String?,
//ferrule:endif
) {

    companion object {

        /**
         * Returns a [MultipartField] containing the given [value] as a [KnownValue].
         *
         * [contentType] will be set to `application/octet-stream` if [value] is binary data, or
         * `text/plain; charset=utf-8` otherwise.
         */
//ferrule:if kotlin
        fun <T : Any> of(value: T?) = builder<T>().value(value).build()
//ferrule:else
        @JvmStatic fun <T : Any> of(value: T?) = builder<T>().value(value).build()
//ferrule:endif

        /**
         * Returns a [MultipartField] containing the given [value].
         *
         * [contentType] will be set to `application/octet-stream` if [value] is binary data, or
         * `text/plain; charset=utf-8` otherwise.
         */
//ferrule:if kotlin
        fun <T : Any> of(value: JsonField<T>) = builder<T>().value(value).build()
//ferrule:else
        @JvmStatic fun <T : Any> of(value: JsonField<T>) = builder<T>().value(value).build()
//ferrule:endif

        /**
         * Returns a mutable builder for constructing an instance of [MultipartField].
         *
         * The following fields are required:
//ferrule:if kotlin
         * ```kotlin
//ferrule:else
         * ```java
//ferrule:endif
         * .value()
         * ```
         *
         * If [contentType] is unset, then it will be set to `application/octet-stream` if [value]
         * is binary data, or `text/plain; charset=utf-8` otherwise.
         */
//ferrule:if kotlin
        fun <T : Any> builder() = Builder<T>()
//ferrule:else
        @JvmStatic fun <T : Any> builder() = Builder<T>()
//ferrule:endif
    }

//ferrule:if java
    /** Returns the filename directive that will be included in the serialized field. */
    fun filename(): Optional<String> = Optional.ofNullable(filename)

    @JvmSynthetic
//ferrule:endif
    internal fun <R : Any> map(transform: (T) -> R): MultipartField<R> =
        builder<R>().value(value.map(transform)).contentType(contentType).filename(filename).build()

    /** A builder for [MultipartField]. */
    class Builder<T : Any> internal constructor() {

        private var value: JsonField<T>? = null
        private var contentType: String? = null
        private var filename: String? = null

        fun value(value: JsonField<T>) = apply { this.value = value }

        fun value(value: T?) = value(JsonField.ofNullable(value))

        fun contentType(contentType: String) = apply { this.contentType = contentType }

        fun filename(filename: String?) = apply { this.filename = filename }

//ferrule:if java
        /** Alias for calling [Builder.filename] with `filename.orElse(null)`. */
        fun filename(filename: Optional<String>) = filename(filename.orElse(null))

//ferrule:endif
        /**
         * Returns an immutable instance of [MultipartField].
         *
         * Further updates to this [Builder] will not mutate the returned instance.
         *
         * The following fields are required:
//ferrule:if kotlin
         * ```kotlin
//ferrule:else
         * ```java
//ferrule:endif
         * .value()
         * ```
         *
         * If [contentType] is unset, then it will be set to `application/octet-stream` if [value]
         * is binary data, or `text/plain; charset=utf-8` otherwise.
         *
         * @throws IllegalStateException if any required field is unset.
         */
        fun build(): MultipartField<T> {
            val value = checkRequired("value", value)
            return MultipartField(
                value,
                contentType
                    ?: if (
                        value is KnownValue &&
                            (value.value is InputStream || value.value is ByteArray)
                    )
                        "application/octet-stream"
                    else "text/plain; charset=utf-8",
                filename,
            )
        }
    }

    private val hashCode: Int by lazy { contentHash(value, contentType, filename) }

    override fun hashCode(): Int = hashCode

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is MultipartField<*> &&
            value == other.value &&
            contentType == other.contentType &&
            filename == other.filename
    }

    override fun toString(): String =
        "MultipartField{value=$value, contentType=$contentType, filename=$filename}"
}
