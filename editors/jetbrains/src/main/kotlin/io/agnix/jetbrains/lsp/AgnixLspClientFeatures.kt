package io.agnix.jetbrains.lsp

import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFile
import com.redhat.devtools.lsp4ij.client.features.LSPClientFeatures
import io.agnix.jetbrains.settings.AgnixSettings
import io.agnix.jetbrains.wsl.WslLaunch
import io.agnix.jetbrains.wsl.WslPaths
import org.eclipse.lsp4j.InitializeParams
import java.net.URI

/**
 * Client features for agnix-lsp.
 *
 * When WSL execution mode is active the server runs inside the distribution and
 * only understands Linux paths, while the Windows IDE addresses the same files
 * through `\\wsl.localhost\<distro>\...`. This class rewrites every path that
 * crosses the LSP boundary: outgoing document URIs and the initialize workspace
 * root, and incoming URIs (diagnostics, locations) on the way back.
 *
 * With WSL execution mode off, all hooks return null / leave params untouched, so
 * LSP4IJ keeps its default URI handling.
 */
class AgnixLspClientFeatures : LSPClientFeatures() {

    private val logger = Logger.getInstance(AgnixLspClientFeatures::class.java)

    /** IDE file -> Linux `file:` URI. Null falls back to `FileUriSupport.DEFAULT`. */
    override fun getFileUri(file: VirtualFile): URI? {
        val distribution = wslDistribution() ?: return null
        val linuxPath = WslPaths.toLinuxPath(file.path, distribution)
        if (linuxPath == null) {
            logger.warn("Cannot map ${file.path} into WSL distribution $distribution")
            return null
        }
        return WslPaths.toLinuxFileUri(linuxPath)
    }

    /** Linux `file:` URI -> IDE file. Null falls back to `FileUriSupport.DEFAULT`. */
    override fun findFileByUri(fileUri: String): VirtualFile? {
        val distribution = wslDistribution() ?: return null
        val linuxPath = WslPaths.linuxPathFromFileUri(fileUri) ?: return null
        val windowsPath = WslPaths.toWindowsPath(linuxPath, distribution) ?: return null
        return LocalFileSystem.getInstance().findFileByPath(WslPaths.toIdeaPath(windowsPath))
    }

    /**
     * Translate the workspace root sent at initialize.
     *
     * agnix-lsp derives the project root from `rootUri`, so a Windows UNC root
     * would leave project-level rules pointing at a path that does not exist
     * inside the distribution.
     */
    // rootUri/rootPath are deprecated in LSP but are what agnix-lsp reads to find the
    // workspace root, so both are translated alongside workspaceFolders.
    @Suppress("DEPRECATION")
    override fun initializeParams(params: InitializeParams) {
        super.initializeParams(params)

        val distribution = wslDistribution() ?: return

        params.rootUri?.let { rootUri ->
            WslPaths.toLinuxFileUri(rootUri, distribution)?.let { params.rootUri = it.toString() }
        }
        params.rootPath?.let { rootPath ->
            WslPaths.toLinuxPath(rootPath, distribution)?.let { params.rootPath = it }
        }
        params.workspaceFolders?.forEach { folder ->
            folder.uri?.let { folderUri ->
                WslPaths.toLinuxFileUri(folderUri, distribution)?.let { folder.uri = it.toString() }
            }
        }
    }

    /**
     * Distribution to translate paths for, or null when WSL execution mode is off
     * or misconfigured (the descriptor reports misconfiguration at launch).
     */
    private fun wslDistribution(): String? {
        val settings = AgnixSettings.getInstance()
        if (!settings.wslEnabled) return null

        val basePath = runCatching { project.basePath }.getOrNull()
        val resolution = WslLaunch.resolve(
            enabled = true,
            distribution = settings.wslDistribution,
            lspPath = settings.wslLspPath,
            projectBasePath = basePath
        )
        return (resolution as? WslLaunch.Resolution.Ready)?.distribution
    }
}
