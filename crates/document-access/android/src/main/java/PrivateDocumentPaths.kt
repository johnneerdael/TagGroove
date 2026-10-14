package org.opentag.documentaccess

import java.io.File

/** Roots must come from Context.filesDir/cacheDir, never a request or URI. */
internal class PrivateDocumentPaths(private val roots: List<File>) {
    private fun validate(path: String): File {
        val supplied = File(path)
        require(supplied.isAbsolute && !path.contains('\u0000')) { "Expected an absolute private file path" }
        require(path.split(File.separatorChar).none { it == "." || it == ".." }) { "Private path cannot contain dot components" }
        val root = roots.firstOrNull { supplied.path.startsWith(it.absolutePath + File.separator) || supplied.path.startsWith(it.canonicalPath + File.separator) }
            ?: throw IllegalArgumentException("Path must be beneath application filesDir or cacheDir")
        val prefix = if (supplied.path.startsWith(root.absolutePath + File.separator)) root.absolutePath else root.canonicalPath
        val parts = supplied.path.removePrefix(prefix + File.separator).split(File.separatorChar)
        var current = root.canonicalFile
        require(current.isDirectory) { "Private storage root is unavailable" }
        for ((index, part) in parts.withIndex()) {
            require(part.isNotEmpty()) { "Invalid private path component" }
            current = File(current, part)
            require(current.canonicalFile == current.absoluteFile) { "Private path cannot contain symlinks" }
            if (index < parts.lastIndex) require(current.isDirectory) { "Private parent must be an existing directory" }
        }
        return current
    }
    fun input(path: String): File = validate(path).also { require(it.isFile) { "Private input must be an existing regular file" } }
    fun output(path: String): File = validate(path).also { require(!it.exists()) { "Private snapshot already exists" } }
}
