package __ferrule_pkg__.core

import java.time.Duration
//ferrule:if kotlin
import kotlin.time.toKotlinDuration
import kotlinx.coroutines.delay
//ferrule:else
import java.util.Timer
import java.util.TimerTask
import java.util.concurrent.CompletableFuture
//ferrule:endif

class DefaultSleeper : Sleeper {

//ferrule:if java
    private val timer = Timer("DefaultSleeper", true)

//ferrule:endif
    override fun sleep(duration: Duration) = Thread.sleep(duration.toMillis())

//ferrule:if kotlin
    override suspend fun sleepAsync(duration: Duration) = delay(duration.toKotlinDuration())
//ferrule:else
    override fun sleepAsync(duration: Duration): CompletableFuture<Void> {
        val future = CompletableFuture<Void>()
        timer.schedule(
            object : TimerTask() {
                override fun run() {
                    future.complete(null)
                }
            },
            duration.toMillis(),
        )
        return future
    }
//ferrule:endif

//ferrule:if kotlin
    override fun close() {}
//ferrule:else
    override fun close() = timer.cancel()
//ferrule:endif
}
