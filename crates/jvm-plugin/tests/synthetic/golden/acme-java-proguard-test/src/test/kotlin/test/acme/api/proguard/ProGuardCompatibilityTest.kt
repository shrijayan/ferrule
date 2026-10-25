// File generated from our OpenAPI spec by Ferrule.

package test.acme.api.proguard

import com.fasterxml.jackson.module.kotlin.jacksonTypeRef
import kotlin.reflect.full.memberFunctions
import kotlin.reflect.jvm.javaMethod
import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.Test
import test.acme.api.client.okhttp.AcmeOkHttpClient
import test.acme.api.core.jsonMapper
import test.acme.api.models.ErrorInfo

internal class ProGuardCompatibilityTest {

    companion object {

        @JvmStatic
        fun main(args: Array<String>) {
            // To debug that we're using the right JAR.
            val jarPath = this::class.java.getProtectionDomain().codeSource.location
            println("JAR being used: $jarPath")

            // We have to manually run the test methods instead of using the JUnit runner because it
            // seems impossible to get working with R8.
            val test = ProGuardCompatibilityTest()
            test::class
                .memberFunctions
                .asSequence()
                .filter { function ->
                    function.javaMethod?.isAnnotationPresent(Test::class.java) == true
                }
                .forEach { it.call(test) }
        }
    }

    @Test
    fun proguardRules() {
        val rulesFile =
            javaClass.classLoader.getResourceAsStream("META-INF/proguard/acme-java-core.pro")

        assertThat(rulesFile).isNotNull()
    }

    @Test
    fun client() {
        val client = AcmeOkHttpClient.builder().apiKey("My API Key").build()

        assertThat(client).isNotNull()
        assertThat(client.widgets()).isNotNull()
    }

    @Test
    fun errorInfoRoundtrip() {
        val jsonMapper = jsonMapper()
        val errorInfo = ErrorInfo.builder().code("code").message("message").build()

        val roundtrippedErrorInfo =
            jsonMapper.readValue(
                jsonMapper.writeValueAsString(errorInfo),
                jacksonTypeRef<ErrorInfo>(),
            )

        assertThat(roundtrippedErrorInfo).isEqualTo(errorInfo)
    }
}
