package __ferrule_pkg__.core

import java.time.Duration
//ferrule:if java
import java.util.concurrent.CompletableFuture
//ferrule:endif

/**
 * A delegating wrapper around a [Sleeper] that closes it once it's only phantom reachable.
 *
 * This class ensures the [Sleeper] is closed even if the user forgets to do it.
 */
internal class PhantomReachableSleeper(private val sleeper: Sleeper) : Sleeper {

    init {
        closeWhenPhantomReachable(this, sleeper)
    }

    override fun sleep(duration: Duration) = sleeper.sleep(duration)

//ferrule:if kotlin
    override suspend fun sleepAsync(duration: Duration) = sleeper.sleepAsync(duration)
//ferrule:else
    override fun sleepAsync(duration: Duration): CompletableFuture<Void> =
        sleeper.sleepAsync(duration)
//ferrule:endif

    override fun close() = sleeper.close()
}
