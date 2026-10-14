package org.opentag.documentaccess

import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.database.Cursor
import android.net.Uri
import android.os.Process
import android.system.Os
import android.system.OsConstants
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.InputStream
import java.io.OutputStream
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import androidx.activity.result.ActivityResult
import androidx.appcompat.app.AppCompatActivity
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import org.json.JSONObject
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

@InvokeArg
class PickDocumentsArgs { var multiple: Boolean = false }
@InvokeArg
class DocumentUriArgs { lateinit var uri: String }
@InvokeArg
class RenameDocumentArgs { lateinit var uri: String; lateinit var name: String }

@InvokeArg
class SnapshotDocumentArgs { lateinit var uri: String; lateinit var outputPath: String }
@InvokeArg
class PublishDocumentArgs { lateinit var uri: String; lateinit var inputPath: String; lateinit var expectedSha256: String }
@InvokeArg
class CreateDocumentArgs { lateinit var name: String; var mimeType: String? = null }

@TauriPlugin
class DocumentAccessPlugin(private val activity: Activity) : Plugin(activity) {
    private val resolver get() = activity.contentResolver
    private val pickerRunning = AtomicBoolean(false)
    private val pendingPicker = AtomicReference<Invoke?>(null)
    private val workers = ThreadPoolExecutor(2, 2, 0L, TimeUnit.MILLISECONDS, ArrayBlockingQueue(32))
    private val readFlag = Intent.FLAG_GRANT_READ_URI_PERMISSION
    private val writeFlag = Intent.FLAG_GRANT_WRITE_URI_PERMISSION
    private val accessFlags = readFlag or writeFlag

    @Command
    fun pickDocuments(invoke: Invoke) {
        val args = try { invoke.parseArgs(PickDocumentsArgs::class.java) }
        catch (error: Exception) { reject(invoke, error); return }
        if (!pickerRunning.compareAndSet(false, true)) {
            invoke.reject("A document picker is already open")
            return
        }
        pendingPicker.set(invoke)
        activity.runOnUiThread {
            try {
                check(!activity.isFinishing && !activity.isDestroyed) { "The activity is closing" }
                val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    type = "*/*"
                    putExtra(Intent.EXTRA_ALLOW_MULTIPLE, args.multiple)
                    addFlags(accessFlags or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
                }
                startActivityForResult(invoke, intent, "documentsPicked")
            } catch (error: Exception) {
                pendingPicker.compareAndSet(invoke, null)
                pickerRunning.set(false)
                reject(invoke, error)
            }
        }
    }

    @ActivityCallback
    fun documentsPicked(invoke: Invoke, result: ActivityResult) {
        if (!pendingPicker.compareAndSet(invoke, null)) return
        if (result.resultCode == Activity.RESULT_CANCELED) {
            pickerRunning.set(false)
            invoke.resolve(JSObject().put("documents", JSArray()))
            return
        }
        if (result.resultCode != Activity.RESULT_OK || result.data == null) {
            pickerRunning.set(false)
            invoke.reject("The document picker returned no document result")
            return
        }
        val data = result.data!!
        val selected = linkedSetOf<Uri>()
        data.clipData?.let { clips ->
            for (index in 0 until clips.itemCount) selected.add(clips.getItemAt(index).uri)
        }
        data.data?.let { selected.add(it) }
        if (selected.isEmpty()) {
            pickerRunning.set(false)
            invoke.reject("The document provider returned an empty selection")
            return
        }
        background(invoke, picker = true) {
            // Persist only the rights actually returned, never the requested mask.
            val granted = data.flags and accessFlags
            if (data.flags and Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION != 0 && granted != 0) {
                selected.forEach { retainWhenOffered(it, granted) }
            }
            val documents = JSArray()
            selected.forEach { uri -> documents.put(query(uri).json) }
            JSObject().put("documents", documents)
        }
    }

    @Command
    fun createDocument(invoke: Invoke) {
        val args = try {
            invoke.parseArgs(CreateDocumentArgs::class.java).also { validateName(it.name) }
        } catch (error: Exception) { reject(invoke, error); return }
        if (!pickerRunning.compareAndSet(false, true)) {
            invoke.reject("A document picker is already open")
            return
        }
        pendingPicker.set(invoke)
        activity.runOnUiThread {
            try {
                check(!activity.isFinishing && !activity.isDestroyed) { "The activity is closing" }
                val intent = Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    type = args.mimeType?.takeIf { it.isNotBlank() } ?: "application/octet-stream"
                    putExtra(Intent.EXTRA_TITLE, args.name)
                    addFlags(accessFlags or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
                }
                startActivityForResult(invoke, intent, "documentCreated")
            } catch (error: Exception) {
                pendingPicker.compareAndSet(invoke, null)
                pickerRunning.set(false)
                reject(invoke, error)
            }
        }
    }

    @ActivityCallback
    fun documentCreated(invoke: Invoke, result: ActivityResult) {
        if (!pendingPicker.compareAndSet(invoke, null)) return
        if (result.resultCode == Activity.RESULT_CANCELED) {
            pickerRunning.set(false)
            invoke.resolve(JSObject().put("document", JSONObject.NULL))
            return
        }
        val data = result.data
        val uri = data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) {
            pickerRunning.set(false)
            invoke.reject("The document picker returned no created document")
            return
        }
        background(invoke, picker = true) {
            val granted = data.flags and accessFlags
            if (data.flags and Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION != 0 && granted != 0) retainWhenOffered(uri, granted)
            JSObject().put("document", query(uri).json)
        }
    }

    private val privatePaths get() = PrivateDocumentPaths(listOf(activity.filesDir, activity.cacheDir))

    private fun privateInput(path: String): InputStream {
        val file = privatePaths.input(path)
        val fd = Os.open(file.path, OsConstants.O_RDONLY or OsConstants.O_NOFOLLOW or OsConstants.O_NONBLOCK, 0)
        try {
            require(OsConstants.S_ISREG(Os.fstat(fd).st_mode)) { "Private input must be a regular file" }
            return FileInputStream(fd)
        } catch (error: Exception) { Os.close(fd); throw error }
    }

    private fun privateOutput(path: String): OutputStream {
        val file = privatePaths.output(path)
        val fd = Os.open(file.path, OsConstants.O_WRONLY or OsConstants.O_CREAT or OsConstants.O_EXCL or OsConstants.O_NOFOLLOW, 384)
        return FileOutputStream(fd)
    }

    private inner class ProviderStreams(private val uri: Uri) : DocumentStreams {
        lateinit var latest: Metadata
        override fun metadata(): DocumentVersion {
            latest = query(uri)
            return latest.version
        }
        override fun openRead(): InputStream = resolver.openInputStream(uri)
            ?: throw IllegalStateException("Provider returned no input stream")
        override fun openTruncatingWrite(): OutputStream = resolver.openOutputStream(uri, "wt")
            ?: throw IllegalStateException("Provider returned no truncating output stream")
    }

    @Command
    fun snapshot(invoke: Invoke) {
        val args = try { invoke.parseArgs(SnapshotDocumentArgs::class.java) }
        catch (error: Exception) { reject(invoke, error); return }
        background(invoke) {
            try {
                privatePaths.output(args.outputPath)
                val provider = ProviderStreams(parseUri(args.uri))
                val result = DocumentTransport.snapshot(provider) { privateOutput(args.outputPath) }
                JSObject().put("document", provider.latest.json).put("sha256", result.sha256).put("bytes", result.bytes)
            } catch (error: Exception) {
                throw IllegalStateException("Snapshot failed; any partial private file is retained at ${args.outputPath}: ${error.message}", error)
            }
        }
    }

    @Command
    fun publish(invoke: Invoke) {
        val args = try { invoke.parseArgs(PublishDocumentArgs::class.java) }
        catch (error: Exception) { reject(invoke, error); return }
        background(invoke) {
            val provider = ProviderStreams(parseUri(args.uri))
            val result = DocumentTransport.publish(provider, { privateInput(args.inputPath) }, args.expectedSha256)
            JSObject().put("document", provider.latest.json).put("sha256", result.sha256).put("bytes", result.bytes)
        }
    }

    private fun validateName(name: String) {
        require(name.isNotBlank() && name != "." && name != ".." &&
            !name.contains('/') && !name.contains('\\') && !name.contains('\u0000')) {
            "Document name must be one nonempty filename without path separators"
        }
    }

    @Command
    fun info(invoke: Invoke) {
        val args = try { invoke.parseArgs(DocumentUriArgs::class.java) }
        catch (error: Exception) { reject(invoke, error); return }
        background(invoke) { query(parseUri(args.uri)).json }
    }

    @Command
    fun rename(invoke: Invoke) {
        val args = try { invoke.parseArgs(RenameDocumentArgs::class.java) }
        catch (error: Exception) { reject(invoke, error); return }
        background(invoke) {
            require(args.name.isNotBlank() && args.name != "." && args.name != ".." &&
                !args.name.contains('/') && !args.name.contains('\\') && !args.name.contains('\u0000')) {
                "Document name must be one nonempty filename without path separators"
            }
            val uri = parseUri(args.uri)
            require(DocumentsContract.isDocumentUri(activity, uri)) { "Provider does not support document renames" }
            val before = query(uri)
            require(before.writeGranted && before.flags and DocumentsContract.Document.FLAG_SUPPORTS_RENAME != 0) {
                "This document does not have a write grant and provider rename support"
            }
            val retained = retainedFlags(uri)
            val renamed = DocumentsContract.renameDocument(resolver, uri, args.name)
                ?: throw IllegalStateException("The document provider did not complete the rename")
            // Some providers assign a new identity and offer new grants on rename.
            // Preserve only previously retained rights that remain actually granted.
            if (renamed != uri && retained != 0) {
                val actual = grantedFlags(renamed) and retained
                if (actual != 0) retainWhenOffered(renamed, actual)
            }
            try { query(renamed).json }
            catch (error: Exception) {
                throw IllegalStateException("Document renamed to $renamed, but metadata refresh failed: ${error.message}", error)
            }
        }
    }

    private data class Metadata(val json: JSObject, val flags: Int, val writeGranted: Boolean, val version: DocumentVersion)

    private fun parseUri(value: String): Uri {
        val uri = Uri.parse(value)
        require(uri.scheme == "content" && !uri.authority.isNullOrBlank() && !value.contains('\u0000')) {
            "Expected an Android content document URI"
        }
        return uri
    }

    private fun grantedFlags(uri: Uri): Int {
        var flags = 0
        for (flag in listOf(readFlag, writeFlag)) {
            if (activity.checkUriPermission(uri, Process.myPid(), Process.myUid(), flag) == PackageManager.PERMISSION_GRANTED) flags = flags or flag
        }
        return flags
    }

    private fun retainedFlags(uri: Uri): Int {
        var flags = 0
        resolver.persistedUriPermissions.filter { it.uri == uri }.forEach { permission ->
            if (permission.isReadPermission) flags = flags or readFlag
            if (permission.isWritePermission) flags = flags or writeFlag
        }
        return flags
    }

    private fun retainWhenOffered(uri: Uri, flags: Int) {
        try { resolver.takePersistableUriPermission(uri, flags and accessFlags) }
        catch (_: SecurityException) { /* Temporary access remains usable; report persisted=false. */ }
        catch (_: UnsupportedOperationException) { /* Nonconforming provider: never invent retained rights. */ }
    }

    private fun Cursor.text(column: String): String? {
        val index = getColumnIndex(column)
        return if (index >= 0 && !isNull(index)) getString(index) else null
    }
    private fun Cursor.number(column: String): Long? {
        val index = getColumnIndex(column)
        return if (index >= 0 && !isNull(index)) getLong(index) else null
    }

    private fun query(uri: Uri): Metadata {
        parseUri(uri.toString())
        val grants = grantedFlags(uri)
        require(grants and readFlag != 0) { "Read permission is not available for document $uri; select it again" }
        var name: String? = null
        var mime: String? = null
        var size: Long? = null
        var modified: Long? = null
        var flags = 0
        val cursor = resolver.query(uri, null, null, null, null)
            ?: throw IllegalStateException("Document provider returned no metadata cursor for $uri")
        cursor.use {
            check(it.moveToFirst()) { "Document is no longer available: $uri" }
            name = it.text(OpenableColumns.DISPLAY_NAME)
            mime = it.text(DocumentsContract.Document.COLUMN_MIME_TYPE)
            size = it.number(OpenableColumns.SIZE)?.takeIf { value -> value >= 0 }
            modified = it.number(DocumentsContract.Document.COLUMN_LAST_MODIFIED)?.takeIf { value -> value > 0 }
            flags = it.number(DocumentsContract.Document.COLUMN_FLAGS)?.toInt() ?: 0
        }
        check(name != null) { "Document provider did not return a display name for $uri" }
        if (mime == null) mime = resolver.getType(uri)
        val writeGranted = grants and writeFlag != 0
        val access = documentAccessState(grants, flags, retainedFlags(uri), mime)
        val json = JSObject().put("uri", uri.toString()).put("name", name)
            .put("mime_type", mime ?: JSONObject.NULL).put("size", size ?: JSONObject.NULL)
            .put("modified_ms", modified ?: JSONObject.NULL).put("writable", access.writable).put("persisted", access.persisted)
        return Metadata(json, flags, writeGranted, DocumentVersion(name!!, mime, size, modified, access.writable))
    }

    private fun background(invoke: Invoke, picker: Boolean = false, work: () -> JSObject) {
        try {
            workers.execute {
                try {
                    val result = work()
                    activity.runOnUiThread { if (picker) pickerRunning.set(false); invoke.resolve(result) }
                } catch (error: Exception) {
                    if (picker) pickerRunning.set(false)
                    reject(invoke, error)
                }
            }
        } catch (error: RejectedExecutionException) {
            if (picker) pickerRunning.set(false)
            reject(invoke, error)
        }
    }
    private fun reject(invoke: Invoke, error: Exception) {
        activity.runOnUiThread { invoke.reject(error.message ?: "Document provider operation failed") }
    }
    override fun onDestroy(activity: AppCompatActivity) {
        pickerRunning.set(false)
        pendingPicker.getAndSet(null)?.let { it.reject("The document picker activity closed") }
        // Already queued provider requests must still complete their Rust callers.
        workers.shutdown()
    }
}
