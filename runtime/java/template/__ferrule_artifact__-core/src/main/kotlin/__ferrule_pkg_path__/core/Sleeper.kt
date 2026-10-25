package __ferrule_pkg__.core

import java.time.Duration
//ferrule:if java
import java.util.concurrent.CompletableFuture
//ferrule:endif

/**
 * An interface for delaying execution for a specified amount of time.
 *
 * Useful for testing and cleaning up resources.
 */
interface Sleeper : AutoCloseable {

    /** Synchronously pauses execution for the given [duration]. */
    fun sleep(duration: Duration)

    /** Asynchronously pauses execution for the given [duration]. */
//ferrule:if kotlin
    suspend fun sleepAsync(duration: Duration)
//ferrule:else
    fun sleepAsync(duration: Duration): CompletableFuture<Void>

    /** Overridden from [AutoCloseable] to not have a checked exception in its signature. */
    override fun close()
//ferrule:endif
}
