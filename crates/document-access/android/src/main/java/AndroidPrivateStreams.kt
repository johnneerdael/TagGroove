package org.opentag.documentaccess

import android.os.ParcelFileDescriptor
import android.system.Os
import android.system.OsConstants
import java.io.Closeable
import java.io.File
import java.io.InputStream
import java.io.OutputStream

internal class AndroidPrivateStreams(private val paths: PrivateDocumentPaths) {
    fun input(path: String): InputStream = open(
        paths.input(path), OsConstants.O_RDONLY or OsConstants.O_NOFOLLOW or OsConstants.O_NONBLOCK,
        regularInput = true
    ) { ParcelFileDescriptor.AutoCloseInputStream(it) }

    fun output(path: String): OutputStream = open(
        paths.output(path), OsConstants.O_WRONLY or OsConstants.O_CREAT or OsConstants.O_EXCL or OsConstants.O_NOFOLLOW,
        regularInput = false
    ) { ParcelFileDescriptor.AutoCloseOutputStream(it) }

    private fun <T : Closeable> open(
        file: File, flags: Int, regularInput: Boolean, stream: (ParcelFileDescriptor) -> T
    ): T {
        val raw = Os.open(file.path, flags, if (regularInput) 0 else 384)
        var owned: ParcelFileDescriptor? = null
        try {
            // Android File{Input,Output}Stream(FileDescriptor) does not own its FD.
            // Keep the original scoped, then give an owning duplicate to AutoClose.
            Closeable { Os.close(raw) }.use {
                if (regularInput) require(OsConstants.S_ISREG(Os.fstat(raw).st_mode)) {
                    "Private input must be a regular file"
                }
                owned = ParcelFileDescriptor.dup(raw)
            }
            return stream(checkNotNull(owned))
        } catch (error: Throwable) {
            // Covers validation/duplication, original-close and stream construction failures.
            try { owned?.close() }
            catch (closeError: Throwable) { error.addSuppressed(closeError) }
            throw error
        }
    }
}
