package __ferrule_pkg__.errors

//ferrule:if kotlin
open class __FerruleClient__Exception(message: String? = null, cause: Throwable? = null) :
    RuntimeException(message, cause)
//ferrule:else
open class __FerruleClient__Exception
@JvmOverloads
constructor(message: String? = null, cause: Throwable? = null) : RuntimeException(message, cause)
//ferrule:endif
