@file:JvmName("StringHandler")

package __ferrule_pkg__.core.handlers

import __ferrule_pkg__.core.http.HttpResponse
import __ferrule_pkg__.core.http.HttpResponse.Handler

//ferrule:if kotlin
internal fun stringHandler(): Handler<String> = StringHandlerInternal
//ferrule:else
@JvmSynthetic internal fun stringHandler(): Handler<String> = StringHandlerInternal
//ferrule:endif

private object StringHandlerInternal : Handler<String> {
    override fun handle(response: HttpResponse): String =
        response.body().readBytes().toString(Charsets.UTF_8)
}
