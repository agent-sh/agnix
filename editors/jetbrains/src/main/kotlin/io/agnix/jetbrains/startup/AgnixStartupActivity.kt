package io.agnix.jetbrains.startup

import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.ProjectActivity
import io.agnix.jetbrains.binary.AgnixBinaryResolver
import io.agnix.jetbrains.binary.PlatformInfo
import io.agnix.jetbrains.notifications.AgnixNotifications
import io.agnix.jetbrains.settings.AgnixSettings
import io.agnix.jetbrains.wsl.WslLaunch

/**
 * Startup activity for the agnix plugin.
 *
 * Runs when a project is opened to:
 * 1. Check if the LSP binary is available
 * 2. Notify if binary is missing and auto-download is disabled
 * 3. Log startup information
 */
class AgnixStartupActivity : ProjectActivity {

    private val logger = Logger.getInstance(AgnixStartupActivity::class.java)

    override suspend fun execute(project: Project) {
        val settings = AgnixSettings.getInstance()

        // Check if plugin is enabled
        if (!settings.enabled) {
            logger.info("agnix plugin is disabled")
            return
        }

        if (!shouldCheckHostBinary(
                pluginEnabled = settings.enabled,
                wslEnabled = settings.wslEnabled,
                wslHostSupported = PlatformInfo.supportsWslExecution()
            )
        ) {
            val resolution = WslLaunch.resolve(
                enabled = true,
                distribution = settings.wslDistribution,
                lspPath = settings.wslLspPath,
                projectBasePath = project.basePath
            )
            when (resolution) {
                is WslLaunch.Resolution.Invalid -> {
                    logger.warn("WSL execution mode is misconfigured: ${resolution.message}")
                    AgnixNotifications.notifyWslLaunchProblem(project, resolution.message)
                }
                is WslLaunch.Resolution.Ready ->
                    logger.info("WSL execution mode enabled; skipping host-local agnix-lsp startup probe")
                WslLaunch.Resolution.Disabled -> Unit
            }
            return
        }

        logger.info("agnix startup activity running for project: ${project.name}")

        // Check for LSP binary using cached resolver
        val existingBinary = AgnixBinaryResolver.resolve()

        if (existingBinary != null) {
            logger.info("agnix-lsp binary found at: $existingBinary")
            return
        }

        // Binary not found
        logger.warn("agnix-lsp binary not found")

        // LSP4IJ installer will auto-download on first server start when enabled.
        if (settings.autoDownload) {
            logger.info("Auto-download enabled; installer will download agnix-lsp when language server starts")
        } else {
            // Show notification to user
            AgnixNotifications.notifyBinaryNotFound(project)
        }
    }

    companion object {
        internal fun shouldCheckHostBinary(
            pluginEnabled: Boolean,
            wslEnabled: Boolean,
            wslHostSupported: Boolean
        ): Boolean = pluginEnabled && !(wslEnabled && wslHostSupported)
    }
}
