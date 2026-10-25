// File generated from our OpenAPI spec by Stainless.

package __ferrule_pkg__.core

import java.time.Duration
import java.util.Objects
//ferrule:if java
import java.util.Optional
import kotlin.jvm.optionals.getOrNull
//ferrule:endif

/** A class containing timeouts for various processing phases of a request. */
class Timeout
private constructor(
    private val connect: Duration?,
    private val read: Duration?,
    private val write: Duration?,
    private val request: Duration?,
) {

    /**
     * The maximum time allowed to establish a connection with a host.
     *
     * A value of [Duration.ZERO] means there's no timeout.
     *
     * Defaults to `Duration.ofMinutes(1)`.
     */
    fun connect(): Duration = connect ?: Duration.ofMinutes(1)

    /**
     * The maximum time allowed between two data packets when waiting for the server’s response.
     *
     * A value of [Duration.ZERO] means there's no timeout.
     *
     * Defaults to `request()`.
     */
    fun read(): Duration = read ?: request()

    /**
     * The maximum time allowed between two data packets when sending the request to the server.
     *
     * A value of [Duration.ZERO] means there's no timeout.
     *
     * Defaults to `request()`.
     */
    fun write(): Duration = write ?: request()

    /**
     * The maximum time allowed for a complete HTTP call, not including retries.
     *
     * This includes resolving DNS, connecting, writing the request body, server processing, as well
     * as reading the response body.
     *
     * A value of [Duration.ZERO] means there's no timeout.
     *
     * Defaults to `Duration.ofMinutes(1)`.
     */
    fun request(): Duration = request ?: Duration.ofMinutes(1)

    fun toBuilder() = Builder().from(this)

    companion object {

//ferrule:if kotlin
        fun default() = builder().build()
//ferrule:else
        @JvmStatic fun default() = builder().build()
//ferrule:endif

        /** Returns a mutable builder for constructing an instance of [Timeout]. */
//ferrule:if kotlin
        fun builder() = Builder()
//ferrule:else
        @JvmStatic fun builder() = Builder()
//ferrule:endif
    }

    /** A builder for [Timeout]. */
    class Builder internal constructor() {

        private var connect: Duration? = null
        private var read: Duration? = null
        private var write: Duration? = null
        private var request: Duration? = null

//ferrule:if java
        @JvmSynthetic
//ferrule:endif
        internal fun from(timeout: Timeout) = apply {
            connect = timeout.connect
            read = timeout.read
            write = timeout.write
            request = timeout.request
        }

        /**
         * The maximum time allowed to establish a connection with a host.
         *
         * A value of [Duration.ZERO] means there's no timeout.
         *
         * Defaults to `Duration.ofMinutes(1)`.
         */
        fun connect(connect: Duration?) = apply { this.connect = connect }

//ferrule:if java
        /** Alias for calling [Builder.connect] with `connect.orElse(null)`. */
        fun connect(connect: Optional<Duration>) = connect(connect.getOrNull())

//ferrule:endif
        /**
         * The maximum time allowed between two data packets when waiting for the server’s response.
         *
         * A value of [Duration.ZERO] means there's no timeout.
         *
         * Defaults to `request()`.
         */
        fun read(read: Duration?) = apply { this.read = read }

//ferrule:if java
        /** Alias for calling [Builder.read] with `read.orElse(null)`. */
        fun read(read: Optional<Duration>) = read(read.getOrNull())

//ferrule:endif
        /**
         * The maximum time allowed between two data packets when sending the request to the server.
         *
         * A value of [Duration.ZERO] means there's no timeout.
         *
         * Defaults to `request()`.
         */
        fun write(write: Duration?) = apply { this.write = write }
//ferrule:if java

        /** Alias for calling [Builder.write] with `write.orElse(null)`. */
        fun write(write: Optional<Duration>) = write(write.getOrNull())
//ferrule:endif

        /**
         * The maximum time allowed for a complete HTTP call, not including retries.
         *
         * This includes resolving DNS, connecting, writing the request body, server processing, as
         * well as reading the response body.
         *
         * A value of [Duration.ZERO] means there's no timeout.
         *
         * Defaults to `Duration.ofMinutes(1)`.
         */
        fun request(request: Duration?) = apply { this.request = request }

//ferrule:if java
        /** Alias for calling [Builder.request] with `request.orElse(null)`. */
        fun request(request: Optional<Duration>) = request(request.getOrNull())

//ferrule:endif
        /**
         * Returns an immutable instance of [Timeout].
         *
         * Further updates to this [Builder] will not mutate the returned instance.
         */
        fun build(): Timeout = Timeout(connect, read, write, request)
    }

//ferrule:if java
    @JvmSynthetic
//ferrule:endif
    internal fun assign(target: Timeout): Timeout =
        target
            .toBuilder()
            .apply {
                connect?.let(this::connect)
                read?.let(this::read)
                write?.let(this::write)
                request?.let(this::request)
            }
            .build()

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }

        return other is Timeout &&
            connect == other.connect &&
            read == other.read &&
            write == other.write &&
            request == other.request
    }

    override fun hashCode(): Int = Objects.hash(connect, read, write, request)

    override fun toString() =
        "Timeout{connect=$connect, read=$read, write=$write, request=$request}"
}
