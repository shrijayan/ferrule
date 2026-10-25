package __ferrule_pkg__.core.http

//ferrule:if java
import java.util.stream.Stream
//ferrule:endif
import __ferrule_pkg__.core.closeWhenPhantomReachable

/**
 * A delegating wrapper around a `StreamResponse` that closes it once it's only phantom reachable.
 *
 * This class ensures the `StreamResponse` is closed even if the user forgets to close it.
 */
internal class PhantomReachableClosingStreamResponse<T>(
    private val streamResponse: StreamResponse<T>
) : StreamResponse<T> {
    init {
        closeWhenPhantomReachable(this, streamResponse)
    }

//ferrule:if kotlin
    override fun asSequence(): Sequence<T> = streamResponse.asSequence()
//ferrule:else
    override fun stream(): Stream<T> = streamResponse.stream()
//ferrule:endif

    override fun close() = streamResponse.close()
}
