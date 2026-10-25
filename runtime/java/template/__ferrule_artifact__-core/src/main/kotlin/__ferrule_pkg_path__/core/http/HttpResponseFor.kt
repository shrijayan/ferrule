package __ferrule_pkg__.core.http

import java.io.InputStream

interface HttpResponseFor<T> : HttpResponse {

    fun parse(): T
}

//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal fun <T> HttpResponse.parseable(parse: () -> T): HttpResponseFor<T> =
    object : HttpResponseFor<T> {

        private val parsed: T by lazy { parse() }

        override fun parse(): T = parsed

        override fun statusCode(): Int = this@parseable.statusCode()

        override fun headers(): Headers = this@parseable.headers()

        override fun body(): InputStream = this@parseable.body()

        override fun close() = this@parseable.close()
    }
