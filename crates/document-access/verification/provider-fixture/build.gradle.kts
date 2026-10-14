import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

plugins { id("com.android.application") version "9.3.1" }
repositories { google(); mavenCentral() }

val generatedAssets = layout.buildDirectory.dir("generated/fixture-assets")
val generateFixture = tasks.register("generateFixture") {
    outputs.dir(generatedAssets)
    doLast {
        fun le(value: Int, bytes: Int = 4) = ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putInt(value).array().copyOf(bytes)
        fun chunk(name: String, data: ByteArray) = name.toByteArray(Charsets.US_ASCII) + le(data.size) + data + if (data.size % 2 == 1) byteArrayOf(0) else byteArrayOf()
        val format = le(1, 2) + le(1, 2) + le(44100) + le(88200) + le(2, 2) + le(16, 2)
        val pcm = ByteArrayOutputStream()
        repeat(44100) { index -> pcm.write(le((Math.sin(index * 2.0 * Math.PI * 440.0 / 44100.0) * 6000).toInt(), 2)) }
        val info = "INFO".toByteArray() + chunk("INAM", "Fixture Original\u0000".toByteArray()) + chunk("IART", "OpenTag Fixture\u0000".toByteArray())
        val body = "WAVE".toByteArray() + chunk("fmt ", format) + chunk("data", pcm.toByteArray()) + chunk("LIST", info)
        generatedAssets.get().file("seed.wav").asFile.apply { parentFile.mkdirs(); writeBytes("RIFF".toByteArray() + le(body.size) + body) }
    }
}
android {
    namespace = "org.opentag.fixture.documents"
    compileSdk = 36
    defaultConfig {
        applicationId = "org.opentag.fixture.documents"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "1.0-test-only"
        testInstrumentationRunner = "org.opentag.fixture.documents.ProviderCheck"
    }
    sourceSets.getByName("main").assets.directories.add(generatedAssets.get().asFile.absolutePath)
}
tasks.named("preBuild").configure { dependsOn(generateFixture) }
