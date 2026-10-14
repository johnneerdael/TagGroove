package org.opentag.documentaccess

import java.io.InputStream
import java.io.OutputStream
import java.security.MessageDigest

internal data class DocumentVersion(val name: String, val mime: String?, val size: Long?, val modified: Long?, val writable: Boolean)
internal data class StreamSnapshot(val document: DocumentVersion, val sha256: String, val bytes: Long)
internal interface DocumentStreams {
    fun metadata(): DocumentVersion
    fun openRead(): InputStream
    fun openTruncatingWrite(): OutputStream
}

/** Streaming transport only. SAF has no compare-and-swap or atomic replace guarantee. */
internal object DocumentTransport {
    private data class Digest(val sha256: String, val bytes: Long)

    private fun transfer(input: InputStream, output: OutputStream? = null): Digest {
        val hash = MessageDigest.getInstance("SHA-256")
        val buffer = ByteArray(64 * 1024)
        var bytes = 0L
        while (true) {
            val count = input.read(buffer)
            if (count < 0) break
            check(count > 0) { "Document stream made no progress" }
            output?.write(buffer, 0, count)
            hash.update(buffer, 0, count)
            bytes = Math.addExact(bytes, count.toLong())
        }
        return Digest(hash.digest().joinToString("") { "%02x".format(it) }, bytes)
    }

    fun snapshot(provider: DocumentStreams, output: () -> OutputStream): StreamSnapshot {
        val before = provider.metadata()
        // The output factory creates exclusively. Never delete a partial snapshot on failure.
        val copied = output().use { out -> provider.openRead().use { transfer(it, out) } }
        val after = provider.metadata()
        check(before == after && (after.size == null || after.size == copied.bytes)) {
            "Document changed while snapshotting"
        }
        return StreamSnapshot(after, copied.sha256, copied.bytes)
    }

    fun publish(provider: DocumentStreams, input: () -> InputStream, expectedSha256: String): StreamSnapshot {
        require(expectedSha256.matches(Regex("[0-9a-fA-F]{64}"))) { "Expected SHA-256 must contain exactly 64 hexadecimal characters" }
        // Validate and open the private input before any provider output can be opened.
        val copied = input().use { edited ->
            val before = provider.metadata()
            require(before.writable) { "Document is not writable" }
            val source = provider.openRead().use { transfer(it) }
            val checked = provider.metadata()
            check(source.sha256.equals(expectedSha256, ignoreCase = true) && before == checked &&
                (checked.size == null || checked.size == source.bytes)) { "Document changed before publication" }
            require(checked.writable) { "Document is not writable" }
            // The provider may change after this check. A failed write must never trigger rollback.
            provider.openTruncatingWrite().use { transfer(edited, it) }
        }
        // Close output before opening a fresh provider stream, including cloud-backed streams.
        val verified = provider.openRead().use { transfer(it) }
        check(copied == verified) { "Document publication verification failed; provider bytes may be partial" }
        val after = provider.metadata()
        check(after.size == null || after.size == verified.bytes) { "Document publication verification failed: size changed" }
        return StreamSnapshot(after, verified.sha256, verified.bytes)
    }
}
