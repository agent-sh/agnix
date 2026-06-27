package io.agnix.jetbrains.binary

import com.intellij.openapi.application.PathManager
import com.intellij.openapi.diagnostic.Logger
import java.io.File
import java.util.jar.JarFile

/**
 * Resolves the location of the agnix-lsp binary.
 *
 * Searches for the binary in multiple locations:
 * 1. Plugin storage directory (downloaded binary)
 * 2. System PATH
 * 3. Common installation directories
 *
 * Uses caching to avoid repeated file system checks.
 */
object AgnixBinaryResolver {

    private val logger = Logger.getInstance(AgnixBinaryResolver::class.java)

    const val BINARY_NAME = "agnix-lsp"
    const val BINARY_NAME_WINDOWS = "agnix-lsp.exe"
    const val VERSION_MARKER_FILE = ".agnix-lsp-version"

    // Build-time generated resource carrying the plugin version (see build.gradle.kts).
    private const val VERSION_RESOURCE = "/agnix-plugin-version.properties"

    // Cache for resolved binary path - cleared when settings change or binary is downloaded
    @Volatile
    private var cachedBinaryPath: String? = null

    // Cache for PATH directories - lazy initialized
    private val pathDirectories: List<String> by lazy {
        System.getenv("PATH")?.split(File.pathSeparator) ?: emptyList()
    }

    /**
     * Clear the cached binary path. Call when settings change or after download.
     */
    fun clearCache() {
        cachedBinaryPath = null
    }

    /**
     * Get the plugin storage directory for downloaded binaries.
     */
    fun getStorageDirectory(): File {
        val pluginDir = PathManager.getPluginsPath()
        return File(pluginDir, "agnix/bin")
    }

    /**
     * Get the path to the downloaded binary, if it exists and its version matches the plugin.
     * Returns null if the binary is missing or its version marker doesn't match,
     * signalling that a re-download is needed.
     */
    fun getDownloadedBinaryPath(): String? {
        val binaryInfo = PlatformInfo.getBinaryInfo() ?: return null
        val binaryFile = File(getStorageDirectory(), binaryInfo.binaryName)

        if (!binaryFile.exists() || !binaryFile.canExecute()) {
            return null
        }

        val pluginVersion = getPluginVersion()
        val installedVersion = readVersionMarker()
        if (pluginVersion != null && installedVersion != pluginVersion) {
            logger.info(
                if (installedVersion != null)
                    "agnix-lsp version mismatch: binary=$installedVersion, plugin=$pluginVersion"
                else
                    "No version marker found for agnix-lsp, plugin=$pluginVersion"
            )
            return null
        }

        return binaryFile.absolutePath
    }

    /**
     * Get the current plugin version.
     *
     * Resolution order, chosen to be IntelliJ Plugin Verifier safe on platform builds up to
     * 262.* (where the descriptor APIs - PluginManagerCore.getPlugin / PluginManager.getPluginByClass -
     * are @ApiStatus.Internal and would be flagged):
     *
     * 1. A build-time generated resource (`/agnix-plugin-version.properties`). This works
     *    identically in local development, `runIde`, unit tests, and the installed plugin
     *    because it ships inside the classpath (jar or exploded build directory).
     * 2. The plugin jar manifest, as a fallback for the installed plugin if the resource is
     *    ever missing. (This returns null in dev/runIde where classes load from a directory.)
     */
    fun getPluginVersion(): String? {
        readPluginVersionFromResource()?.let { return it }
        return readPluginVersionFromJarLocation()
    }

    internal fun readPluginVersionFromResource(): String? {
        return try {
            AgnixBinaryResolver::class.java.getResourceAsStream(VERSION_RESOURCE)?.use { stream ->
                val props = java.util.Properties().apply { load(stream) }
                props.getProperty("plugin.version")?.trim()?.takeIf { it.isNotEmpty() }
            }
        } catch (_: Exception) {
            null
        }
    }

    private fun readPluginVersionFromJarLocation(): String? {
        return try {
            val location = AgnixBinaryResolver::class.java.protectionDomain?.codeSource?.location ?: return null
            readPluginVersionFromJar(File(location.toURI()))
        } catch (_: Exception) {
            null
        }
    }

    internal fun readPluginVersionFromJar(jarFile: File): String? {
        if (!jarFile.isFile) return null

        return JarFile(jarFile).use {
            it.manifest?.mainAttributes?.getValue("Version")?.trim()?.takeIf { v -> v.isNotEmpty() }
        }
    }

    /**
     * Read the version marker from the storage directory.
     */
    fun readVersionMarker(): String? {
        val markerFile = File(getStorageDirectory(), VERSION_MARKER_FILE)
        return try {
            if (markerFile.exists()) markerFile.readText().trim() else null
        } catch (_: Exception) {
            null
        }
    }

    /**
     * Write the version marker to the storage directory.
     */
    fun writeVersionMarker(version: String) {
        val markerFile = File(getStorageDirectory(), VERSION_MARKER_FILE)
        try {
            markerFile.writeText(version)
        } catch (e: Exception) {
            logger.warn("Failed to write version marker", e)
        }
    }

    /**
     * Find agnix-lsp in the system PATH.
     */
    fun findInPath(): String? {
        val binaryName = getBinaryName()
        val extensions = if (PlatformInfo.getOS() == PlatformInfo.OS.WINDOWS) {
            listOf("", ".exe", ".cmd", ".bat")
        } else {
            listOf("")
        }

        for (dir in pathDirectories) {
            for (ext in extensions) {
                val file = File(dir, binaryName + ext)
                if (file.exists() && file.canExecute()) {
                    logger.info("Found agnix-lsp in PATH: ${file.absolutePath}")
                    return file.absolutePath
                }
            }
        }

        return null
    }

    /**
     * Find agnix-lsp in common installation directories.
     */
    fun findInCommonLocations(): String? {
        val binaryName = getBinaryName()
        val homeDir = System.getProperty("user.home")

        val locations = when (PlatformInfo.getOS()) {
            PlatformInfo.OS.MACOS -> listOf(
                "/usr/local/bin/$binaryName",
                "/opt/homebrew/bin/$binaryName",
                "$homeDir/.cargo/bin/$binaryName",
                "$homeDir/.local/bin/$binaryName"
            )
            PlatformInfo.OS.LINUX -> listOf(
                "/usr/local/bin/$binaryName",
                "/usr/bin/$binaryName",
                "$homeDir/.cargo/bin/$binaryName",
                "$homeDir/.local/bin/$binaryName"
            )
            PlatformInfo.OS.WINDOWS -> listOf(
                "$homeDir\\.cargo\\bin\\$binaryName",
                "C:\\Program Files\\agnix\\$binaryName"
            )
            PlatformInfo.OS.UNKNOWN -> emptyList()
        }

        for (location in locations) {
            val file = File(location)
            if (file.exists() && file.canExecute()) {
                logger.info("Found agnix-lsp at: ${file.absolutePath}")
                return file.absolutePath
            }
        }

        return null
    }

    /**
     * Resolve the binary path using all available methods with caching.
     *
     * Priority:
     * 1. Cached path (if still valid)
     * 2. Downloaded binary in plugin storage
     * 3. System PATH
     * 4. Common installation locations
     */
    fun resolve(): String? {
        // Check cache first
        cachedBinaryPath?.let { cached ->
            if (File(cached).exists()) {
                return cached
            }
            // Cache is stale, clear it
            cachedBinaryPath = null
        }

        // Check downloaded binary
        getDownloadedBinaryPath()?.let {
            cachedBinaryPath = it
            return it
        }

        // Check PATH
        findInPath()?.let {
            cachedBinaryPath = it
            return it
        }

        // Check common locations
        findInCommonLocations()?.let {
            cachedBinaryPath = it
            return it
        }

        logger.warn("agnix-lsp binary not found")
        return null
    }

    /**
     * Get the expected binary name for the current platform.
     */
    fun getBinaryName(): String {
        return if (PlatformInfo.getOS() == PlatformInfo.OS.WINDOWS) {
            BINARY_NAME_WINDOWS
        } else {
            BINARY_NAME
        }
    }

    /**
     * Check if a binary exists at the given path and is executable.
     */
    fun isValidBinary(path: String): Boolean {
        val file = File(path)
        return file.exists() && file.canExecute()
    }
}
