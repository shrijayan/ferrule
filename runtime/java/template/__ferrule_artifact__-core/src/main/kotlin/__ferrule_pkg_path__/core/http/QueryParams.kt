// File generated from our OpenAPI spec by Stainless.

package __ferrule_pkg__.core.http

import __ferrule_pkg__.core.JsonArray
import __ferrule_pkg__.core.JsonBoolean
import __ferrule_pkg__.core.JsonMissing
import __ferrule_pkg__.core.JsonNull
import __ferrule_pkg__.core.JsonNumber
import __ferrule_pkg__.core.JsonObject
import __ferrule_pkg__.core.JsonString
import __ferrule_pkg__.core.JsonValue
import __ferrule_pkg__.core.toImmutable

//ferrule:if kotlin
class QueryParams private constructor(private val map: Map<String, List<String>>, val size: Int) {
//ferrule:else
class QueryParams
private constructor(
    private val map: Map<String, List<String>>,
    @get:JvmName("size") val size: Int,
) {
//ferrule:endif

    fun isEmpty(): Boolean = map.isEmpty()

    fun keys(): Set<String> = map.keys

    fun values(key: String): List<String> = map[key].orEmpty()

    fun toBuilder(): Builder = Builder().putAll(map)

    companion object {

//ferrule:if kotlin
        fun builder() = Builder()
//ferrule:else
        @JvmStatic fun builder() = Builder()
//ferrule:endif
    }

    class Builder internal constructor() {

        private val map: MutableMap<String, MutableList<String>> = mutableMapOf()
        private var size: Int = 0

        fun put(key: String, value: JsonValue): Builder = apply {
            when (value) {
                is JsonMissing,
                is JsonNull -> {}
                is JsonBoolean -> put(key, value.value.toString())
                is JsonNumber -> put(key, value.value.toString())
                is JsonString -> put(key, value.value)
//ferrule:if comma
                is JsonArray ->
                    put(
                        key,
                        value.values
                            .asSequence()
                            .mapNotNull {
                                when (it) {
                                    is JsonMissing,
                                    is JsonNull -> null
                                    is JsonBoolean -> it.value.toString()
                                    is JsonNumber -> it.value.toString()
                                    is JsonString -> it.value
                                    is JsonArray,
                                    is JsonObject ->
                                        throw IllegalArgumentException(
                                            "Cannot comma separate non-primitives in query params"
                                        )
                                }
                            }
                            .joinToString(","),
                    )
//ferrule:else
                is JsonArray -> value.values.forEach { put(key, it) }
//ferrule:endif
                is JsonObject ->
                    value.values.forEach { (nestedKey, value) -> put("$key[$nestedKey]", value) }
            }
        }

        fun put(key: String, value: String) = apply {
            map.getOrPut(key) { mutableListOf() }.add(value)
            size++
        }

        fun put(key: String, values: Iterable<String>) = apply { values.forEach { put(key, it) } }

        fun putAll(queryParams: Map<String, Iterable<String>>) = apply {
            queryParams.forEach(::put)
        }

        fun putAll(queryParams: QueryParams) = apply {
            queryParams.keys().forEach { put(it, queryParams.values(it)) }
        }

        fun replace(key: String, value: String) = apply {
            remove(key)
            put(key, value)
        }

        fun replace(key: String, values: Iterable<String>) = apply {
            remove(key)
            put(key, values)
        }

        fun replaceAll(queryParams: Map<String, Iterable<String>>) = apply {
            queryParams.forEach(::replace)
        }

        fun replaceAll(queryParams: QueryParams) = apply {
            queryParams.keys().forEach { replace(it, queryParams.values(it)) }
        }

        fun remove(key: String) = apply { size -= map.remove(key).orEmpty().size }

        fun removeAll(keys: Set<String>) = apply { keys.forEach(::remove) }

        fun clear() = apply {
            map.clear()
            size = 0
        }

        fun build() =
            QueryParams(map.mapValues { (_, values) -> values.toImmutable() }.toImmutable(), size)
    }

    override fun hashCode(): Int = map.hashCode()

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is QueryParams && map == other.map
    }

    override fun toString(): String = "QueryParams{map=$map}"
}
