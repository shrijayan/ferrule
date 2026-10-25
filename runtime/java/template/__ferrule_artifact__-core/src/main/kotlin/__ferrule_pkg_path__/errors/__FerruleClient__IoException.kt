package __ferrule_pkg__.errors

//ferrule:if kotlin
class __FerruleClient__IoException(message: String? = null, cause: Throwable? = null) :
    __FerruleClient__Exception(message, cause)
//ferrule:else
class __FerruleClient__IoException
@JvmOverloads
//ferrule:if fits
constructor(message: String? = null, cause: Throwable? = null) : __FerruleClient__Exception(message, cause)
//ferrule:else
constructor(message: String? = null, cause: Throwable? = null) :
    __FerruleClient__Exception(message, cause)
//ferrule:endif
//ferrule:endif
