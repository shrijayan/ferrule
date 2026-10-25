// File generated from our OpenAPI spec by Stainless.

package __ferrule_pkg__.errors

import __ferrule_pkg__.core.JsonValue
import __ferrule_pkg__.core.http.Headers

abstract class __FerruleClient__ServiceException
protected constructor(message: String, cause: Throwable? = null) :
    __FerruleClient__Exception(message, cause) {

    abstract fun statusCode(): Int

    abstract fun headers(): Headers

    abstract fun body(): JsonValue
}
