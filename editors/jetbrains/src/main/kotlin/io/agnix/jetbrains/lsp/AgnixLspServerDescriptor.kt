package io.agnix.jetbrains.lsp

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.server.OSProcessStreamConnectionProvider
import io.agnix.jetbrains.binary.AgnixBinaryResolver
import io.agnix.jetbrains.binary.PlatformInfo
import io.agnix.jetbrains.notifications.AgnixNotifications
import io.agnix.jetbrains.settings.AgnixSettings
import io.agnix.jetbrains.wsl.WslLaunch
import java.io.File

/**
 * LSP server descriptor for agnix.
 *
 * Manages the lifecycle of the agnix-lsp process using stdio transport.
 * Handles binary resolution, download if needed, and process startup.
 */
class AgnixLspServerDescriptor(
    private val project: Project
) : OSProcessStreamConnectionProvider() {

    private val logger = Logger.getInstance(AgnixLspServerDescriptor::class.java)

    init {
        when (val resolution = resolveWslLaunch()) {
            is WslLaunch.Resolution.Ready -> configureWslCommandLine(resolution)
            is WslLaunch.Resolution.Invalid ->
                // Reported to the user in start(); the descriptor must not show UI from init.
                logger.warn("WSL execution mode is misconfigured: ${resolution.message}")
            WslLaunch.Resolution.Disabled -> {
                // Resolve binary path without blocking download - only check existing locations
                val binaryPath = resolveBinaryPathNonBlocking()
                if (binaryPath != null) {
                    configureCommandLine(binaryPath)
                }
            }
        }
    }

    private fun configureCommandLine(binaryPath: String) {
        val commandLine = GeneralCommandLine(binaryPath)
            .withWorkDirectory(project.basePath ?: System.getProperty("user.home"))
        setCommandLine(commandLine)
    }

    /**
     * Launch agnix-lsp inside WSL.
     *
     * The Windows-side working directory is left at the IDE default: a Linux path
     * is not a valid Windows working directory, and agnix-lsp takes its workspace
     * from the LSP `rootUri` (translated by [AgnixLspClientFeatures]).
     */
    private fun configureWslCommandLine(launch: WslLaunch.Resolution.Ready) {
        val command = WslLaunch.buildCommand(launch.distribution, launch.lspPath)
        logger.info("Starting agnix-lsp through WSL distribution ${launch.distribution}: ${launch.lspPath}")
        setCommandLine(GeneralCommandLine(command))
    }

    /** WSL launch decision from the current settings and project location. */
    private fun resolveWslLaunch(): WslLaunch.Resolution {
        val settings = AgnixSettings.getInstance()
        return WslLaunch.resolve(
            enabled = settings.wslEnabled && PlatformInfo.supportsWslExecution(),
            distribution = settings.wslDistribution,
            lspPath = settings.wslLspPath,
            projectBasePath = project.basePath
        )
    }

    /**
     * Resolve the path to the agnix-lsp binary without blocking.
     *
     * Checks existing locations only - does NOT trigger download.
     * Download is handled by the LSP4IJ server installer flow.
     */
    private fun resolveBinaryPathNonBlocking(): String? {
        val settings = AgnixSettings.getInstance()

        // Check user-configured path first
        val configuredPath = settings.lspPath
        if (configuredPath.isNotBlank()) {
            val file = File(configuredPath)
            if (file.exists() && file.canExecute()) {
                logger.info("Using configured LSP path: $configuredPath")
                return configuredPath
            }
        }

        // Use cached resolver to check existing binary locations
        val downloadedPath = AgnixBinaryResolver.getDownloadedBinaryPath()
        if (downloadedPath != null) {
            logger.info("Using downloaded LSP binary: $downloadedPath")
            return downloadedPath
        }

        val systemPath = AgnixBinaryResolver.findInPath()
        if (systemPath != null) {
            logger.info("Using system PATH LSP binary: $systemPath")
            return systemPath
        }

        // Binary not found - notify user but do NOT block with download
        logger.warn("agnix-lsp binary not found")
        return null
    }

    override fun start() {
        // Re-resolve WSL settings here: they can change between descriptor creation
        // and a server (re)start.
        when (val resolution = resolveWslLaunch()) {
            is WslLaunch.Resolution.Ready -> {
                configureWslCommandLine(resolution)
                // If a policy blocks the launch it is wsl.exe that was refused, not
                // the Linux binary, so report the launcher in the failure path.
                startProcess("${WslLaunch.WSL_EXECUTABLE} (${resolution.lspPath})")
                return
            }
            is WslLaunch.Resolution.Invalid -> {
                logger.error("WSL execution mode is misconfigured: ${resolution.message}")
                AgnixNotifications.notifyWslLaunchProblem(project, resolution.message)
                return
            }
            WslLaunch.Resolution.Disabled -> Unit
        }

        var commandLine = getCommandLine()

        // On first run, LSP4IJ installer may download agnix-lsp after descriptor init.
        // Re-resolve here so the freshly installed binary can be used immediately.
        if (commandLine == null || !File(commandLine.exePath).exists()) {
            val resolvedPath = resolveBinaryPathNonBlocking()
            if (resolvedPath != null) {
                configureCommandLine(resolvedPath)
                commandLine = getCommandLine()
            }
        }

        if (commandLine == null) {
            logger.error("No LSP command configured - binary not found")
            AgnixNotifications.notifyBinaryNotFound(project)
            return
        }

        val binaryPath = commandLine.exePath
        if (!File(binaryPath).exists()) {
            logger.error("LSP binary not found: $binaryPath")
            AgnixNotifications.notifyBinaryNotFound(project)
            return
        }

        startProcess(binaryPath)
    }

    /**
     * Start the configured process, turning an OS-level execution block into an
     * actionable notification.
     */
    private fun startProcess(binaryPath: String) {
        logger.info("Starting agnix-lsp: ${getCommandLine()?.commandLineString}")
        try {
            super.start()
        } catch (e: Exception) {
            // On locked-down machines (corporate AppLocker/WDAC/EDR/AV), the OS can
            // refuse to *execute* the auto-downloaded binary even though it exists,
            // surfacing as `CreateProcess error=5` (Windows) / EACCES (POSIX). Turn
            // that opaque stack trace into an actionable hint, then re-throw so
            // LSP4IJ's own lifecycle handling is unchanged.
            if (LspProcessErrors.isAccessDenied(e)) {
                logger.warn("agnix-lsp launch denied by OS/security policy: $binaryPath", e)
                AgnixNotifications.notifyBinaryAccessDenied(project, binaryPath)
            }
            throw e
        }
    }

    override fun stop() {
        logger.info("Stopping agnix-lsp")
        super.stop()
    }

    override fun isAlive(): Boolean {
        return super.isAlive()
    }
}
