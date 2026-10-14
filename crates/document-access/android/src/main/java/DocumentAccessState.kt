package org.opentag.documentaccess

import android.content.Intent
import android.provider.DocumentsContract

internal data class DocumentAccessState(val writable: Boolean, val persisted: Boolean)

internal fun documentAccessState(granted: Int, providerFlags: Int, retained: Int, mime: String?): DocumentAccessState {
    val read = Intent.FLAG_GRANT_READ_URI_PERMISSION
    val write = Intent.FLAG_GRANT_WRITE_URI_PERMISSION
    val writable = granted and write != 0 &&
        providerFlags and DocumentsContract.Document.FLAG_SUPPORTS_WRITE != 0 &&
        providerFlags and DocumentsContract.Document.FLAG_VIRTUAL_DOCUMENT == 0 &&
        mime != DocumentsContract.Document.MIME_TYPE_DIR
    val persisted = retained and read != 0 && (!writable || retained and write != 0)
    return DocumentAccessState(writable, persisted)
}
