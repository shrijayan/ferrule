@file:JvmName("Utils")

package __ferrule_pkg__.core

import java.util.Collections
import java.util.SortedMap
import java.util.SortedSet
//ferrule:if java
import java.util.concurrent.CompletableFuture
//ferrule:endif
import java.util.concurrent.locks.Lock
import __ferrule_pkg__.errors.__FerruleClient__InvalidDataException

//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun <T : Any> T?.getOrThrow(name: String): T =
    this ?: throw __FerruleClient__InvalidDataException("`${name}` is not present")

//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun <T> List<T>.toImmutable(): List<T> =
    if (isEmpty()) Collections.emptyList() else Collections.unmodifiableList(toList())

//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun <V : Comparable<V>> SortedSet<V>.toImmutable(): SortedSet<V> =
    if (isEmpty()) Collections.emptySortedSet()
    else Collections.unmodifiableSortedSet(toSortedSet(comparator() ?: Comparator.naturalOrder()))

//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun <K, V> Map<K, V>.toImmutable(): Map<K, V> =
    if (isEmpty()) immutableEmptyMap() else Collections.unmodifiableMap(toMap())

//ferrule:if kotlin
internal fun <K, V> immutableEmptyMap(): Map<K, V> = Collections.emptyMap()
//ferrule:else
@JvmSynthetic internal fun <K, V> immutableEmptyMap(): Map<K, V> = Collections.emptyMap()
//ferrule:endif

//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun <K : Comparable<K>, V> SortedMap<K, V>.toImmutable(): SortedMap<K, V> =
    if (isEmpty()) Collections.emptySortedMap()
    else Collections.unmodifiableSortedMap(toSortedMap(comparator()))

/**
 * Returns all elements that yield the largest value for the given function, or an empty list if
 * there are zero elements.
 *
 * This is similar to [Sequence.maxByOrNull] except it returns _all_ elements that yield the largest
 * value; not just the first one.
 */
//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun <T, R : Comparable<R>> Sequence<T>.allMaxBy(selector: (T) -> R): List<T> {
    var maxValue: R? = null
    val maxElements = mutableListOf<T>()

    val iterator = iterator()
    while (iterator.hasNext()) {
        val element = iterator.next()
        val value = selector(element)
        if (maxValue == null || value > maxValue) {
            maxValue = value
            maxElements.clear()
            maxElements.add(element)
        } else if (value == maxValue) {
            maxElements.add(element)
        }
    }

    return maxElements
}

/**
 * Returns whether [this] is equal to [other].
 *
 * This differs from [Object.equals] because it also deeply equates arrays based on their contents,
 * even when there are arrays directly nested within other arrays.
 */
//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal infix fun Any?.contentEquals(other: Any?): Boolean =
    arrayOf(this).contentDeepEquals(arrayOf(other))

/**
 * Returns a hash of the given sequence of [values].
 *
 * This differs from [java.util.Objects.hash] because it also deeply hashes arrays based on their
 * contents, even when there are arrays directly nested within other arrays.
 */
//ferrule:if kotlin
internal fun contentHash(vararg values: Any?): Int = values.contentDeepHashCode()
//ferrule:else
@JvmSynthetic internal fun contentHash(vararg values: Any?): Int = values.contentDeepHashCode()
//ferrule:endif

/**
 * Returns a [String] representation of [this].
 *
 * This differs from [Object.toString] because it also deeply stringifies arrays based on their
 * contents, even when there are arrays directly nested within other arrays.
 */
//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun Any?.contentToString(): String {
    var string = arrayOf(this).contentDeepToString()
    if (string.startsWith('[')) {
        string = string.substring(1)
    }
    if (string.endsWith(']')) {
        string = string.substring(0, string.length - 1)
    }
    return string
}

internal interface Enum

/**
//ferrule:if kotlin
 * Executes a suspending block of code while holding this lock.
//ferrule:else
 * Executes the given [action] while holding the lock, returning a [CompletableFuture] with the
 * result.
//ferrule:endif
 *
//ferrule:if kotlin
 * @param T the return type of the action
 * @param action the suspending function to execute while holding the lock
 * @return the result of executing the action
//ferrule:else
 * @param action The asynchronous action to execute while holding the lock
 * @return A [CompletableFuture] that completes with the result of the action
//ferrule:endif
 */
//ferrule:if kotlin
internal suspend fun <T> Lock.withLockAsync(action: suspend () -> T): T {
//ferrule:else
@JvmSynthetic
internal fun <T> Lock.withLockAsync(action: () -> CompletableFuture<T>): CompletableFuture<T> {
//ferrule:endif
    lock()
//ferrule:if kotlin
    return try {
        action()
    } finally {
        unlock()
    }
//ferrule:else
    val future =
        try {
            action()
        } catch (e: Throwable) {
            unlock()
            throw e
        }
    future.whenComplete { _, _ -> unlock() }
    return future
//ferrule:endif
}
