@file:JvmName("JsonHandler")

package __ferrule_pkg__.core.handlers

import com.fasterxml.jackson.databind.json.JsonMapper
import com.fasterxml.jackson.module.kotlin.jacksonTypeRef
import __ferrule_pkg__.core.http.HttpResponse
import __ferrule_pkg__.core.http.HttpResponse.Handler
import __ferrule_pkg__.errors.__FerruleClient__InvalidDataException

//ferrule:if java
@JvmSynthetic
//ferrule:endif
internal inline fun <reified T> jsonHandler(jsonMapper: JsonMapper): Handler<T> =
    object : Handler<T> {
//ferrule:if kotlin
        override fun handle(response: HttpResponse): T {
//ferrule:else
        override fun handle(response: HttpResponse): T =
//ferrule:endif
            try {
//ferrule:if kotlin
                return jsonMapper.readValue(response.body(), jacksonTypeRef())
//ferrule:else
                jsonMapper.readValue(response.body(), jacksonTypeRef())
//ferrule:endif
            } catch (e: Exception) {
                throw __FerruleClient__InvalidDataException("Error reading response", e)
            }
//ferrule:if kotlin
        }
//ferrule:endif
    }
