package org.opentag.documentaccess

import android.system.Os
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.*
import java.security.MessageDigest
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AndroidPrivateStreamsTest {
    private fun fixture(work: (File, AndroidPrivateStreams) -> Unit) {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val directory = File(context.cacheDir, "private-stream-test-${System.nanoTime()}").apply { check(mkdir()) }
        try { work(directory, AndroidPrivateStreams(PrivateDocumentPaths(listOf(context.filesDir, context.cacheDir)))) }
        finally { directory.deleteRecursively() }
    }
    private fun assertNoPrivateDescriptors(directory: File) {
        val prefix = directory.canonicalPath + File.separator
        val open = File("/proc/self/fd").listFiles()!!.mapNotNull { descriptor ->
            try { Os.readlink(descriptor.path) } catch (_: Exception) { null }
        }.filter { File(it).canonicalPath.startsWith(prefix) }
        assertTrue("private descriptors remain open: $open", open.isEmpty())
    }
    private fun hash(bytes: ByteArray) = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }
    private fun failure(work: () -> Unit) { try { work() } catch (_: Exception) { return }; fail("Expected failure") }
    private class Provider(var bytes: ByteArray, val failWrite: Boolean = false) : DocumentStreams {
        override fun metadata() = DocumentVersion("fixture.wav", "audio/wav", bytes.size.toLong(), null, true)
        override fun openRead(): InputStream = bytes.inputStream()
        override fun openTruncatingWrite(): OutputStream = object : ByteArrayOutputStream() {
            override fun write(buffer: ByteArray, offset: Int, count: Int) {
                if (failWrite) throw IOException("injected provider write failure")
                super.write(buffer, offset, count)
            }
            override fun close() { bytes = toByteArray(); super.close() }
        }
    }
    @Test fun snapshotClosesEveryNativeOutputIncludingTransferFailure() = fixture { dir, streams ->
        repeat(32) { index ->
            val file = File(dir, "snapshot-$index.wav")
            var descriptor: FileDescriptor? = null
            val provider = Provider("source".toByteArray())
            DocumentTransport.snapshot(provider) {
                (streams.output(file.path) as FileOutputStream).also { descriptor = it.fd }
            }
            assertFalse("snapshot leaked native output at iteration $index", descriptor!!.valid())
            assertEquals("source", file.readText())
            assertNoPrivateDescriptors(dir)
        }
        val failed = File(dir, "failed.wav")
        var descriptor: FileDescriptor? = null
        val provider = object : DocumentStreams {
            override fun metadata() = DocumentVersion("fixture.wav", "audio/wav", null, null, true)
            override fun openRead(): InputStream = object : InputStream() {
                var first = true
                override fun read(): Int = throw IOException("injected read failure")
                override fun read(b: ByteArray, off: Int, len: Int): Int {
                    if (!first) throw IOException("injected read failure")
                    first = false; b[off] = 42; return 1
                }
            }
            override fun openTruncatingWrite(): OutputStream = throw AssertionError("unexpected write")
        }
        failure { DocumentTransport.snapshot(provider) { (streams.output(failed.path) as FileOutputStream).also { descriptor = it.fd } } }
        assertFalse("failed snapshot leaked native output", descriptor!!.valid())
        assertArrayEquals(byteArrayOf(42), failed.readBytes())
        assertNoPrivateDescriptors(dir)
    }
    @Test fun publicationClosesEveryNativeInputOnSuccessConflictAndWriteFailure() = fixture { dir, streams ->
        val edited = File(dir, "edited.wav").apply { writeText("edited") }
        repeat(32) { index ->
            for (mode in 0..2) {
                val provider = Provider("source".toByteArray(), failWrite = mode == 2)
                var descriptor: FileDescriptor? = null
                val work = {
                    DocumentTransport.publish(provider, {
                        (streams.input(edited.path) as FileInputStream).also { descriptor = it.fd }
                    }, hash(if (mode == 1) "stale".toByteArray() else provider.bytes))
                    Unit
                }
                if (mode == 0) work() else failure(work)
                assertFalse("publication leaked native input at iteration $index mode $mode", descriptor!!.valid())
                assertEquals("edited", edited.readText())
                assertNoPrivateDescriptors(dir)
            }
        }
    }
    @Test fun privateOpensRemainExclusiveAndRejectLinksAndDirectories() = fixture { dir, streams ->
        val file = File(dir, "original.wav").apply { writeText("preserve") }
        failure { streams.output(file.path).close() }
        assertEquals("preserve", file.readText())
        failure { streams.input(dir.path).close() }
        val link = File(dir, "alias.wav")
        Os.symlink(file.path, link.path)
        failure { streams.input(link.path).close() }
        failure { streams.output(link.path).close() }
        val directoryLink = File(dir, "directory-alias")
        Os.symlink(dir.path, directoryLink.path)
        try { failure { streams.output(File(directoryLink, "escaped.wav").path).close() } }
        finally { directoryLink.delete() }
    }
}
