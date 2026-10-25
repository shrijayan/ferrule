@file:JvmName("EmptyHandler")

package __ferrule_pkg__.core.handlers

import __ferrule_pkg__.core.http.HttpResponse
import __ferrule_pkg__.core.http.HttpResponse.Handler

@JvmSynthetic internal fun emptyHandler(): Handler<Void?> = EmptyHandlerInternal

private object EmptyHandlerInternal : Handler<Void?> {
    override fun handle(response: HttpResponse): Void? = null
}
