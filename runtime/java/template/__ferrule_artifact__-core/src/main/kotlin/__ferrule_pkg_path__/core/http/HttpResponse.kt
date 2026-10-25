// File generated from our OpenAPI spec by Stainless.

package __ferrule_pkg__.core.http

import java.io.InputStream

interface HttpResponse : AutoCloseable {

    fun statusCode(): Int

    fun headers(): Headers

    fun body(): InputStream

//ferrule:if java
    /** Overridden from [AutoCloseable] to not have a checked exception in its signature. */
    override fun close()

//ferrule:endif
    interface Handler<T> {

        fun handle(response: HttpResponse): T
    }
}
