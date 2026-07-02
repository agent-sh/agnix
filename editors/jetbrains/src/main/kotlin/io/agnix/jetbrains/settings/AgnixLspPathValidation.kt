package io.agnix.jetbrains.settings

import java.io.File

/**
 * Validation for the optional user-configured agnix-lsp executable path.
 */
object AgnixLspPathValidation {

    const val ABSOLUTE_PATH_REQUIRED_MESSAGE =
        "LSP binary path must be a full absolute executable path, for example " +
            "C:\\Program Files\\agnix\\agnix-lsp.exe. Do not use just agnix-lsp.exe."

    private val windowsDriveAbsolutePath = Regex("^[A-Za-z]:[\\\\/].+")
    private val windowsUncAbsolutePath = Regex("^\\\\\\\\[^\\\\/]+[\\\\/][^\\\\/]+.*")

    fun validate(path: String): String? {
        val trimmed = path.trim()
        if (trimmed.isEmpty()) {
            return null
        }

        return if (isAbsolutePath(trimmed)) {
            null
        } else {
            ABSOLUTE_PATH_REQUIRED_MESSAGE
        }
    }

    internal fun isAbsolutePath(path: String): Boolean {
        val trimmed = path.trim()
        return trimmed.startsWith('/') ||
            File(trimmed).isAbsolute ||
            windowsDriveAbsolutePath.matches(trimmed) ||
            windowsUncAbsolutePath.matches(trimmed)
    }
}
