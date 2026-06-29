package io.agnix.jetbrains.lsp

/**
 * Pure helpers for classifying agnix-lsp process-launch failures.
 *
 * Kept free of IntelliJ Platform types so it can be unit-tested without the
 * platform test framework.
 */
object LspProcessErrors {

    // Bound the cause-chain walk so a cyclic `cause` graph cannot loop forever.
    private const val MAX_CAUSE_DEPTH = 16

    /**
     * Returns true when [error] (or any exception in its cause chain) indicates
     * the OS refused to *execute* the binary, as opposed to it being missing.
     *
     * Detection is locale-independent where possible:
     * - Windows: `CreateProcess error=5` is Win32 `ERROR_ACCESS_DENIED`. The JDK
     *   formats the message as `CreateProcess error=5, <localized text>`, so the
     *   numeric code is present regardless of the user's Windows display
     *   language (e.g. Danish "Adgang naegtet").
     * - POSIX (Linux/macOS): `errno=13` / `EACCES` / "Permission denied" when the
     *   file lacks the execute bit or is blocked by a security module.
     *
     * English phrases are matched case-insensitively as a fallback.
     */
    fun isAccessDenied(error: Throwable?): Boolean {
        var current = error
        var depth = 0
        while (current != null && depth < MAX_CAUSE_DEPTH) {
            if (messageIndicatesAccessDenied(current.message)) {
                return true
            }
            val next = current.cause
            // Defensive: a throwable can be its own cause.
            if (next === current) break
            current = next
            depth++
        }
        return false
    }

    private fun messageIndicatesAccessDenied(message: String?): Boolean {
        if (message.isNullOrBlank()) return false
        val lower = message.lowercase()
        return lower.contains("error=5") || // Win32 ERROR_ACCESS_DENIED
            lower.contains("errno=13") || // POSIX EACCES
            lower.contains("eacces") ||
            lower.contains("access is denied") ||
            lower.contains("permission denied")
    }
}
