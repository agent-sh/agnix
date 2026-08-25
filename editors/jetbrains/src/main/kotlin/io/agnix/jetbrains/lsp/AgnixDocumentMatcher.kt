package io.agnix.jetbrains.lsp

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.redhat.devtools.lsp4ij.AbstractDocumentMatcher
import io.agnix.jetbrains.binary.PlatformInfo
import io.agnix.jetbrains.filetype.AgnixFileTypes
import io.agnix.jetbrains.settings.AgnixSettings
import io.agnix.jetbrains.wsl.WslLaunch
import io.agnix.jetbrains.wsl.WslPaths

/**
 * Path-aware matcher to avoid attaching agnix-lsp to unrelated files that share
 * common names like settings.json or plugin.json.
 */
class AgnixDocumentMatcher : AbstractDocumentMatcher() {
    override fun match(virtualFile: VirtualFile, project: Project): Boolean {
        val settings = AgnixSettings.getInstance()
        return matchesPath(
            path = virtualFile.path,
            projectBasePath = project.basePath,
            wslEnabled = settings.wslEnabled,
            wslHostSupported = PlatformInfo.supportsWslExecution(),
            distribution = settings.wslDistribution,
            lspPath = settings.wslLspPath
        )
    }

    companion object {
        /**
         * Pure matching policy used by [match] and unit tests.
         *
         * When WSL mode is active, returning false is essential: returning true
         * would let LSP4IJ fall back to the untranslated Windows URI.
         */
        internal fun matchesPath(
            path: String,
            projectBasePath: String?,
            wslEnabled: Boolean,
            wslHostSupported: Boolean,
            distribution: String,
            lspPath: String
        ): Boolean {
            if (!AgnixFileTypes.isAgnixFilePath(path)) return false
            if (!wslEnabled || !wslHostSupported) return true

            val resolution = WslLaunch.resolve(
                enabled = true,
                distribution = distribution,
                lspPath = lspPath,
                projectBasePath = projectBasePath
            )
            val ready = resolution as? WslLaunch.Resolution.Ready ?: return false
            return WslPaths.toLinuxPath(path, ready.distribution) != null
        }
    }
}
