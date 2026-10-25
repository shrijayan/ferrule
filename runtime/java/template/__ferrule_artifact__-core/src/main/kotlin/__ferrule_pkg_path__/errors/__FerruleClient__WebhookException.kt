package __ferrule_pkg__.errors

class __FerruleClient__WebhookException
@JvmOverloads
//ferrule:if fits
constructor(message: String? = null, cause: Throwable? = null) : __FerruleClient__Exception(message, cause)
//ferrule:else
constructor(message: String? = null, cause: Throwable? = null) :
    __FerruleClient__Exception(message, cause)
//ferrule:endif
