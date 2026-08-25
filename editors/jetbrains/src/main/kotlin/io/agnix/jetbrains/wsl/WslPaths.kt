package io.agnix.jetbrains.wsl

import java.net.URI
import java.util.Locale

/**
 * Pure Windows <-> WSL path and `file:` URI translation.
 *
 * A Windows IDE that opens a WSL-hosted project sees it through one of the two
 * WSL UNC roots (`\\wsl$\<distro>\...` or `\\wsl.localhost\<distro>\...`), while
 * agnix-lsp running inside the distribution only understands Linux paths. Every
 * path that crosses the LSP boundary therefore has to be rewritten in both
 * directions.
 *
 * Kept free of IntelliJ Platform types so it can be unit-tested without the
 * platform test framework (same rationale as [io.agnix.jetbrains.lsp.LspProcessErrors]).
 */
object WslPaths {

    /** UNC hosts IntelliJ uses for WSL shares; both are accepted on input. */
    private const val WSL_DOLLAR_HOST = "wsl$"
    private const val WSL_LOCALHOST_HOST = "wsl.localhost"

    /**
     * Default WSL automount root for Windows drives. Configurable per distribution
     * via `/etc/wsl.conf` (`[automount] root=`); only paths outside the distribution
     * (for example a file on `C:`) are mapped through it.
     */
    private const val AUTOMOUNT_ROOT = "/mnt"

    private val driveRootPath = Regex("^([A-Za-z]):(/.*)?$")
    private val mountedDrivePath = Regex("^$AUTOMOUNT_ROOT/([a-zA-Z])(/.*)?$")
    private val leadingDriveInUriPath = Regex("^/[A-Za-z]:(/.*)?$")

    /** A WSL UNC path split into its distribution and Linux parts. */
    data class WslUncPath(val distribution: String, val linuxPath: String)

    /**
     * Parse a WSL UNC path into distribution + Linux path.
     *
     * Accepts backslash form (`\\wsl.localhost\Ubuntu\home\me`) and the
     * forward-slash form IntelliJ uses for `VirtualFile.getPath()`
     * (`//wsl.localhost/Ubuntu/home/me`). Returns null for anything else.
     */
    fun parseUncPath(path: String): WslUncPath? {
        val normalized = path.trim().replace('\\', '/')
        if (!normalized.startsWith("//")) return null

        val segments = normalized.removePrefix("//").split('/')
        if (segments.size < 2) return null

        val host = segments[0].lowercase(Locale.ROOT)
        if (host != WSL_DOLLAR_HOST && host != WSL_LOCALHOST_HOST) return null

        val distribution = segments[1]
        if (distribution.isEmpty()) return null

        val linuxSegments = segments.drop(2).filter { it.isNotEmpty() }
        return WslUncPath(distribution, "/" + linuxSegments.joinToString("/"))
    }

    /**
     * Distribution name encoded in a WSL UNC path, or null if [path] is not one.
     *
     * Used to default the configured distribution to whatever the IDE already
     * opened the project from.
     */
    fun distributionFromUncPath(path: String?): String? =
        path?.let { parseUncPath(it)?.distribution }

    /**
     * Translate a Windows-side path to the Linux path agnix-lsp expects.
     *
     * - WSL UNC path for [distribution] -> its Linux path
     * - WSL UNC path for a *different* distribution -> null (not reachable)
     * - Windows drive path -> automount path (`C:\src` -> `/mnt/c/src`)
     * - already-absolute Linux path -> unchanged
     */
    fun toLinuxPath(windowsPath: String, distribution: String): String? {
        val trimmed = windowsPath.trim()
        if (trimmed.isEmpty()) return null

        parseUncPath(trimmed)?.let { unc ->
            return if (unc.distribution.equals(distribution, ignoreCase = true)) unc.linuxPath else null
        }

        val normalized = trimmed.replace('\\', '/')
        // A regular Windows UNC share is not a Linux absolute path and is not
        // reachable merely because the language server runs inside WSL.
        if (normalized.startsWith("//")) return null
        driveRootPath.matchEntire(normalized)?.let { match ->
            val drive = match.groupValues[1].lowercase(Locale.ROOT)
            val rest = match.groupValues[2].trimStart('/')
            return if (rest.isEmpty()) "$AUTOMOUNT_ROOT/$drive" else "$AUTOMOUNT_ROOT/$drive/$rest"
        }

        return if (normalized.startsWith("/")) normalized else null
    }

    /**
     * Translate a Linux path back to a Windows path the IDE can resolve.
     *
     * Automount paths map back to their drive (`/mnt/c/src` -> `C:\src`); anything
     * else maps to the `\\wsl.localhost\<distribution>\...` share.
     */
    fun toWindowsPath(linuxPath: String, distribution: String): String? {
        val trimmed = linuxPath.trim()
        if (!trimmed.startsWith("/")) return null

        mountedDrivePath.matchEntire(trimmed)?.let { match ->
            val drive = match.groupValues[1].uppercase(Locale.ROOT)
            val rest = match.groupValues[2]
            return "$drive:" + rest.replace('/', '\\').ifEmpty { "\\" }
        }

        if (distribution.isBlank()) return null
        val rest = trimmed.trim('/').replace('/', '\\')
        val root = "\\\\$WSL_LOCALHOST_HOST\\$distribution"
        return if (rest.isEmpty()) root else "$root\\$rest"
    }

    /**
     * IntelliJ VFS spelling of a Windows path (forward slashes, UNC keeps `//`).
     */
    fun toIdeaPath(windowsPath: String): String = windowsPath.replace('\\', '/')

    /** `file:` URI for an absolute Linux path, or null if [linuxPath] is not absolute. */
    fun toLinuxFileUri(linuxPath: String): URI? {
        if (!linuxPath.startsWith("/")) return null
        // Empty host keeps the canonical `file:///path` form; the constructor
        // percent-encodes characters that are illegal in a URI path.
        return runCatching { URI("file", "", linuxPath, null) }.getOrNull()
    }

    /**
     * Windows-side path carried by a `file:` URI.
     *
     * Handles the UNC spellings a Windows IDE can emit (`file://wsl.localhost/...`,
     * `file:////wsl.localhost/...`, `file://wsl$/...`) and drive URIs
     * (`file:///C:/src`).
     */
    fun windowsPathFromFileUri(fileUri: String): String? {
        val uri = runCatching { URI(fileUri.trim()) }.getOrNull() ?: return null
        if (!"file".equals(uri.scheme, ignoreCase = true)) return null

        val path = uri.path ?: return null
        if (path.isEmpty()) return null

        val host = uri.host ?: uri.authority
        if (!host.isNullOrEmpty()) return "//$host$path"

        // `file:///C:/src` -> `C:/src`
        return if (leadingDriveInUriPath.matches(path)) path.substring(1) else path
    }

    /** Absolute Linux path carried by a plain `file:///...` URI. */
    fun linuxPathFromFileUri(fileUri: String): String? {
        val uri = runCatching { URI(fileUri.trim()) }.getOrNull() ?: return null
        if (!"file".equals(uri.scheme, ignoreCase = true)) return null
        if (!uri.host.isNullOrEmpty() || !uri.authority.isNullOrEmpty()) return null

        val path = uri.path ?: return null
        return if (path.startsWith("/") && !leadingDriveInUriPath.matches(path)) path else null
    }

    /**
     * Rewrite a Windows-side `file:` URI into the Linux `file:` URI for
     * [distribution], or null when it cannot be reached from that distribution.
     */
    fun toLinuxFileUri(fileUri: String, distribution: String): URI? {
        val windowsPath = windowsPathFromFileUri(fileUri) ?: return null
        val linuxPath = toLinuxPath(windowsPath, distribution) ?: return null
        return toLinuxFileUri(linuxPath)
    }
}
