package __ferrule_pkg__.core.http
//ferrule:if java

import java.util.stream.Stream
//ferrule:endif

interface StreamResponse<T> : AutoCloseable {

//ferrule:if kotlin
    fun asSequence(): Sequence<T>
//ferrule:else
    fun stream(): Stream<T>

    /** Overridden from [AutoCloseable] to not have a checked exception in its signature. */
    override fun close()
//ferrule:endif
}

//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun <T, R> StreamResponse<T>.map(transform: (T) -> R): StreamResponse<R> =
    object : StreamResponse<R> {
//ferrule:if kotlin
        override fun asSequence(): Sequence<R> = this@map.asSequence().map(transform)
//ferrule:else
        override fun stream(): Stream<R> = this@map.stream().map(transform)
//ferrule:endif

        override fun close() = this@map.close()
    }
