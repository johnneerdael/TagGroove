package org.opentag.documentaccess

import android.content.Intent
import android.provider.DocumentsContract.Document
import org.junit.Assert.assertEquals
import org.junit.Test

class DocumentAccessStateTest {
    private val read = Intent.FLAG_GRANT_READ_URI_PERMISSION
    private val write = Intent.FLAG_GRANT_WRITE_URI_PERMISSION

    @Test fun providerWriteSupportCannotInventAWriteGrant() {
        assertEquals(DocumentAccessState(false, true), documentAccessState(read, Document.FLAG_SUPPORTS_WRITE, read, "audio/flac"))
    }
    @Test fun deleteCapabilityIsNotByteWriteCapability() {
        assertEquals(DocumentAccessState(false, true), documentAccessState(read or write, Document.FLAG_SUPPORTS_DELETE, read or write, "audio/flac"))
    }
    @Test fun temporaryWriteWithOnlyRetainedReadIsNotFullyPersisted() {
        assertEquals(DocumentAccessState(true, false), documentAccessState(read or write, Document.FLAG_SUPPORTS_WRITE, read, "audio/flac"))
    }
    @Test fun retainedReadAndWriteAreRequiredForEditablePersistentSources() {
        assertEquals(DocumentAccessState(true, true), documentAccessState(read or write, Document.FLAG_SUPPORTS_WRITE, read or write, "audio/flac"))
        assertEquals(DocumentAccessState(true, false), documentAccessState(read or write, Document.FLAG_SUPPORTS_WRITE, 0, "audio/flac"))
    }
    @Test fun virtualDocumentsAndDirectoriesCannotBeByteEdited() {
        val grants = read or write
        assertEquals(false, documentAccessState(grants, Document.FLAG_SUPPORTS_WRITE or Document.FLAG_VIRTUAL_DOCUMENT, grants, "audio/flac").writable)
        assertEquals(false, documentAccessState(grants, Document.FLAG_SUPPORTS_WRITE, grants, Document.MIME_TYPE_DIR).writable)
    }
}
