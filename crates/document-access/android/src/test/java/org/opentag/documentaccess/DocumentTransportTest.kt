package org.opentag.documentaccess

import java.io.*
import java.nio.file.Files
import java.security.MessageDigest
import org.junit.Assert.*
import org.junit.Test

class DocumentTransportTest {
    private fun hash(bytes: ByteArray) = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }
    private class Provider(var bytes: ByteArray) : DocumentStreams {
        var opens = 0
        var reads = 0
        var writable = true
        var dropLast = false
        var failAfter = Int.MAX_VALUE
        var changeMetadata = false
        var unknown = false
        var failReadBack = false
        var failClose = false
        var declaredSize: Long? = null
        var maxRead = 0
        override fun metadata() = DocumentVersion("track.wav", "audio/wav", if (unknown) null else declaredSize ?: bytes.size.toLong(), if (changeMetadata && reads > 0) 2L else 1L, writable)
        override fun openRead(): InputStream {
            reads++
            if (failReadBack && reads > 1) throw IOException("fresh provider read failed")
            return object : ByteArrayInputStream(bytes) {
                override fun read(b: ByteArray, off: Int, len: Int): Int { maxRead = maxOf(maxRead, len); return super.read(b, off, minOf(len, 997)) }
            }
        }
        override fun openTruncatingWrite(): OutputStream {
            opens++
            bytes = byteArrayOf()
            return object : ByteArrayOutputStream() {
                override fun write(b: ByteArray, off: Int, len: Int) {
                    val n = minOf(len, failAfter - size()).coerceAtLeast(0)
                    super.write(b, off, n)
                    bytes = toByteArray()
                    if (n < len) throw IOException("provider write failed")
                }
                override fun close() { if (failClose) throw IOException("provider close failed"); bytes = toByteArray().let { if (dropLast) it.dropLast(1).toByteArray() else it }; super.close() }
            }
        }
    }
    private fun failure(block: () -> Unit): Exception { try { block() } catch (e: Exception) { return e }; throw AssertionError("Expected failure") }
    @Test fun changedSourceNeverOpensOutput() {
        val p = Provider("external change".toByteArray())
        val error = failure { DocumentTransport.publish(p, { "edited".byteInputStream() }, hash("original".toByteArray())) }
        assertTrue(error.message!!.contains("changed"))
        assertEquals(0, p.opens); assertEquals("external change", String(p.bytes))
    }
    @Test fun truncatesAndVerifiesFreshBytesUsingBoundedReads() {
        val p = Provider(ByteArray(200000) { 3 }); val edited = ByteArray(100000) { (it % 239).toByte() }
        val result = DocumentTransport.publish(p, { edited.inputStream() }, hash(p.bytes))
        assertArrayEquals(edited, p.bytes); assertEquals(hash(edited), result.sha256)
        assertEquals(edited.size.toLong(), result.bytes); assertEquals(2, p.reads); assertTrue(p.maxRead <= 65536)
    }
    @Test fun silentlyShortProviderWriteIsRejectedByFreshHash() {
        val p = Provider("original".toByteArray()); p.dropLast = true
        val e = failure { DocumentTransport.publish(p, { "edited".byteInputStream() }, hash(p.bytes)) }
        assertTrue(e.message!!.contains("verification")); assertEquals("edite", String(p.bytes))
    }
    @Test fun partialFailurePreservesEditedInputAndDoesNotRollback() {
        val p = Provider("original".toByteArray()); p.failAfter = 3
        val edited = "edited".toByteArray()
        failure { DocumentTransport.publish(p, { edited.inputStream() }, hash(p.bytes)) }
        assertEquals("edi", String(p.bytes)); assertEquals("edited", String(edited)); assertEquals(1, p.opens)
    }
    @Test fun invalidHashOrReadonlySourceCannotTruncate() {
        val p = Provider("original".toByteArray())
        assertTrue(failure { DocumentTransport.publish(p, { "edit".byteInputStream() }, "bad") }.message!!.contains("SHA-256"))
        p.writable = false
        assertTrue(failure { DocumentTransport.publish(p, { "edit".byteInputStream() }, hash(p.bytes)) }.message!!.contains("writable"))
        assertEquals(0, p.opens)
    }
    @Test fun snapshotRejectsMetadataChangeAndKeepsCopiedEvidence() {
        val p = Provider("original".toByteArray()); p.changeMetadata = true
        val out = ByteArrayOutputStream()
        failure { DocumentTransport.snapshot(p, { out }) }
        assertEquals("original", out.toString())
    }
    @Test fun snapshotPreservesUnknownSizeAndHashesFullStream() {
        val p = Provider(ByteArray(180001) { (it % 211).toByte() }); p.unknown = true
        val out = ByteArrayOutputStream(); val result = DocumentTransport.snapshot(p, { out })
        assertNull(result.document.size); assertEquals(hash(p.bytes), result.sha256)
        assertArrayEquals(p.bytes, out.toByteArray()); assertTrue(p.maxRead <= 65536)
    }
    @Test fun closeAndFreshReadFailuresNeverReportSuccessOrRollback() {
        for (onClose in listOf(true, false)) {
            val p = Provider("original".toByteArray()); p.failClose = onClose; p.failReadBack = !onClose
            val error = failure { DocumentTransport.publish(p, { "edited".byteInputStream() }, hash(p.bytes)) }
            assertTrue(error.message!!.contains(if (onClose) "close" else "fresh provider read"))
            assertEquals("edited", String(p.bytes)); assertEquals(1, p.opens)
        }
    }
    @Test fun invalidPrivateInputPreventsProviderTruncation() {
        val p = Provider("original".toByteArray())
        failure { DocumentTransport.publish(p, { throw IOException("private input unavailable") }, hash(p.bytes)) }
        assertEquals(0, p.opens); assertEquals("original", String(p.bytes))
    }
    @Test fun knownSizeMismatchRetainsSnapshotAndRejectsPublication() {
        val p = Provider("original".toByteArray()); p.declaredSize = 100L
        val out = ByteArrayOutputStream()
        failure { DocumentTransport.snapshot(p, { out }) }
        assertEquals("original", out.toString())
        failure { DocumentTransport.publish(p, { "edited".byteInputStream() }, hash(p.bytes)) }
        assertEquals(0, p.opens)
    }
    @Test fun privatePathsRejectEscapesSymlinksDirectoriesAndExistingSnapshots() {
        val temp = Files.createTempDirectory("document-path-test").toFile()
        try {
            val root = File(temp, "files").apply { mkdir() }; val cache = File(temp, "cache").apply { mkdir() }
            val policy = PrivateDocumentPaths(listOf(root, cache))
            val audio = File(root, "audio.wav").apply { writeText("original") }
            assertEquals(audio.canonicalFile, policy.input(audio.path))
            assertEquals(File(cache, "snapshot.wav").canonicalFile, policy.output(File(cache, "snapshot.wav").path))
            for (path in listOf(root.path, audio.path, File(root, "../outside").path, File(temp, "files-other/out").path, File(root, "audio.wav/child").path)) failure { policy.output(path) }
            failure { policy.input(File(root, "absent").path) }
            Files.createSymbolicLink(File(root, "alias").toPath(), audio.toPath())
            failure { policy.input(File(root, "alias").path) }
            Files.createSymbolicLink(File(root, "diralias").toPath(), cache.toPath())
            failure { policy.output(File(root, "diralias/out").path) }
            assertEquals("original", audio.readText())
        } finally { temp.deleteRecursively() }
    }
}
