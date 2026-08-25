package io.agnix.jetbrains.wsl

/**
 * Pure `wsl.exe` command construction and WSL settings validation.
 *
 * The command is always built as an argument *list* and handed to
 * `GeneralCommandLine` unquoted, so distribution names and Linux paths that
 * contain spaces cannot be re-split or injected: `--exec` runs the binary
 * directly without a Linux shell, so no second round of quoting applies inside
 * the distribution.
 *
 * The JetBrains `WSLDistribution.patchCommandLine` API is deliberately not used:
 * it is `@ApiStatus.Experimental` and the plugin ships for build range 233 to
 * 262.*, where a moved or changed experimental API would break the plugin
 * verifier. `wsl.exe --distribution <name> --exec <binary>` is documented CLI
 * surface and stable across WSL 1 and WSL 2.
 *
 * Kept free of IntelliJ Platform types so it can be unit-tested without the
 * platform test framework.
 */
object WslLaunch {

    /** Launcher used on the Windows side. Resolved through `PATH` (System32). */
    const val WSL_EXECUTABLE = "wsl.exe"

    const val DISTRIBUTION_REQUIRED_MESSAGE =
        "WSL distribution is required. Run 'wsl --list --quiet' in a Windows terminal and " +
            "copy the exact name, for example Ubuntu. Leave it empty only when the project " +
            "itself is opened from a \\\\wsl.localhost or \\\\wsl$ path."

    const val DISTRIBUTION_INVALID_MESSAGE =
        "WSL distribution name must not start with '-' or contain slashes, quotes, or control characters."

    const val LINUX_PATH_REQUIRED_MESSAGE =
        "WSL agnix-lsp path must be an absolute Linux path inside the distribution, " +
            "for example /home/you/.cargo/bin/agnix-lsp."

    private val forbiddenInDistribution = charArrayOf('/', '\\', '"', '\'')

    /**
     * Outcome of turning the persisted settings into a launch decision.
     */
    sealed class Resolution {
        /** WSL execution mode is off; the native launch path is used unchanged. */
        object Disabled : Resolution()

        /** WSL execution mode is on but misconfigured; [message] is user-facing. */
        data class Invalid(val message: String) : Resolution()

        /** WSL execution mode is on and usable. */
        data class Ready(val distribution: String, val lspPath: String) : Resolution()
    }

    /**
     * Resolve WSL execution settings.
     *
     * An empty distribution falls back to the one encoded in [projectBasePath]
     * when the IDE already opened the project from a WSL UNC path.
     */
    fun resolve(
        enabled: Boolean,
        distribution: String,
        lspPath: String,
        projectBasePath: String?
    ): Resolution {
        if (!enabled) return Resolution.Disabled

        val effectiveDistribution = distribution.trim()
            .ifEmpty { WslPaths.distributionFromUncPath(projectBasePath).orEmpty() }
        validateDistribution(effectiveDistribution)?.let { return Resolution.Invalid(it) }

        val effectiveLspPath = lspPath.trim()
        validateLspPath(effectiveLspPath)?.let { return Resolution.Invalid(it) }

        return Resolution.Ready(effectiveDistribution, effectiveLspPath)
    }

    /** Returns an error message for an unusable distribution name, or null. */
    fun validateDistribution(distribution: String): String? {
        val trimmed = distribution.trim()
        if (trimmed.isEmpty()) return DISTRIBUTION_REQUIRED_MESSAGE
        // A leading '-' would be parsed as a wsl.exe option instead of a value.
        if (trimmed.startsWith("-")) return DISTRIBUTION_INVALID_MESSAGE
        if (trimmed.any { it in forbiddenInDistribution || it.isControlChar() }) {
            return DISTRIBUTION_INVALID_MESSAGE
        }
        return null
    }

    /** Returns an error message for an unusable Linux binary path, or null. */
    fun validateLspPath(lspPath: String): String? {
        val trimmed = lspPath.trim()
        if (!trimmed.startsWith("/")) return LINUX_PATH_REQUIRED_MESSAGE
        if (trimmed.any { it.isControlChar() }) return LINUX_PATH_REQUIRED_MESSAGE
        return null
    }

    /** Returns the first validation error for the pair, or null when both are usable. */
    fun validate(distribution: String, lspPath: String): String? =
        validateDistribution(distribution) ?: validateLspPath(lspPath)

    /**
     * Build the `wsl.exe` argument list that starts agnix-lsp over stdio.
     *
     * No working directory is passed: `wsl.exe --cd` only exists on newer WSL
     * builds, and agnix-lsp derives its workspace from the LSP `rootUri` rather
     * than the process working directory.
     *
     * @throws IllegalArgumentException when [distribution] or [lspPath] is unusable.
     */
    fun buildCommand(distribution: String, lspPath: String): List<String> {
        val trimmedDistribution = distribution.trim()
        val trimmedLspPath = lspPath.trim()
        validate(trimmedDistribution, trimmedLspPath)?.let { throw IllegalArgumentException(it) }

        return listOf(
            WSL_EXECUTABLE,
            "--distribution",
            trimmedDistribution,
            "--exec",
            trimmedLspPath
        )
    }

    private fun Char.isControlChar(): Boolean = this.code < 0x20 || this.code == 0x7F
}
