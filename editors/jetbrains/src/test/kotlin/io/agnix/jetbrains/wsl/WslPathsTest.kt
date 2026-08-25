package io.agnix.jetbrains.wsl

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test

/**
 * Tests for Windows <-> WSL path and URI translation.
 */
class WslPathsTest {

    @Test
    fun `parses wsl localhost unc path in idea forward slash form`() {
        val parsed = WslPaths.parseUncPath("//wsl.localhost/Ubuntu/home/me/project")
        assertEquals("Ubuntu", parsed?.distribution)
        assertEquals("/home/me/project", parsed?.linuxPath)
    }

    @Test
    fun `parses legacy wsl dollar unc path in backslash form`() {
        val parsed = WslPaths.parseUncPath("\\\\wsl$\\Debian\\home\\me")
        assertEquals("Debian", parsed?.distribution)
        assertEquals("/home/me", parsed?.linuxPath)
    }

    @Test
    fun `parses distribution root`() {
        val parsed = WslPaths.parseUncPath("//wsl.localhost/Ubuntu/")
        assertEquals("Ubuntu", parsed?.distribution)
        assertEquals("/", parsed?.linuxPath)
    }

    @Test
    fun `non wsl paths are not unc wsl paths`() {
        assertNull(WslPaths.parseUncPath("C:\\src\\project"))
        assertNull(WslPaths.parseUncPath("//fileserver/share/project"))
        assertNull(WslPaths.parseUncPath("/home/me/project"))
        assertNull(WslPaths.parseUncPath("//wsl.localhost"))
        assertNull(WslPaths.parseUncPath("//wsl.localhost//home/me"))
    }

    @Test
    fun `distribution is derived from the project location`() {
        assertEquals(
            "Ubuntu-24.04",
            WslPaths.distributionFromUncPath("//wsl.localhost/Ubuntu-24.04/home/me/project")
        )
        assertNull(WslPaths.distributionFromUncPath("C:\\src\\project"))
        assertNull(WslPaths.distributionFromUncPath(null))
    }

    @Test
    fun `unc path maps to linux path for the same distribution`() {
        assertEquals(
            "/home/me/project/SKILL.md",
            WslPaths.toLinuxPath("\\\\wsl.localhost\\Ubuntu\\home\\me\\project\\SKILL.md", "Ubuntu")
        )
    }

    @Test
    fun `unc distribution match is case insensitive`() {
        assertEquals(
            "/home/me",
            WslPaths.toLinuxPath("//wsl.localhost/ubuntu/home/me", "Ubuntu")
        )
    }

    @Test
    fun `unc path of another distribution is not reachable`() {
        assertNull(WslPaths.toLinuxPath("//wsl.localhost/Debian/home/me", "Ubuntu"))
    }

    @Test
    fun `windows drive path maps to the automount root`() {
        assertEquals("/mnt/c/src/project", WslPaths.toLinuxPath("C:\\src\\project", "Ubuntu"))
        assertEquals("/mnt/d", WslPaths.toLinuxPath("D:\\", "Ubuntu"))
        assertEquals("/mnt/c/src", WslPaths.toLinuxPath("c:/src", "Ubuntu"))
    }

    @Test
    fun `linux paths pass through and junk is rejected`() {
        assertEquals("/home/me/project", WslPaths.toLinuxPath("/home/me/project", "Ubuntu"))
        assertNull(WslPaths.toLinuxPath("   ", "Ubuntu"))
        assertNull(WslPaths.toLinuxPath("relative/path", "Ubuntu"))
    }

    @Test
    fun `linux path maps back to the wsl share`() {
        assertEquals(
            "\\\\wsl.localhost\\Ubuntu\\home\\me\\project",
            WslPaths.toWindowsPath("/home/me/project", "Ubuntu")
        )
        assertEquals("\\\\wsl.localhost\\Ubuntu", WslPaths.toWindowsPath("/", "Ubuntu"))
    }

    @Test
    fun `automount path maps back to its windows drive`() {
        assertEquals("C:\\src\\project", WslPaths.toWindowsPath("/mnt/c/src/project", "Ubuntu"))
        assertEquals("C:\\", WslPaths.toWindowsPath("/mnt/c", "Ubuntu"))
    }

    @Test
    fun `relative linux path has no windows mapping`() {
        assertNull(WslPaths.toWindowsPath("home/me", "Ubuntu"))
        assertNull(WslPaths.toWindowsPath("/home/me", ""))
    }

    @Test
    fun `idea path spelling uses forward slashes`() {
        assertEquals(
            "//wsl.localhost/Ubuntu/home/me",
            WslPaths.toIdeaPath("\\\\wsl.localhost\\Ubuntu\\home\\me")
        )
    }

    @Test
    fun `linux path becomes a canonical file uri`() {
        assertEquals("file:///home/me/project/SKILL.md", WslPaths.toLinuxFileUri("/home/me/project/SKILL.md").toString())
        assertNull(WslPaths.toLinuxFileUri("home/me"))
    }

    @Test
    fun `file uri encodes spaces`() {
        assertEquals("file:///home/me/my%20project/SKILL.md", WslPaths.toLinuxFileUri("/home/me/my project/SKILL.md").toString())
    }

    @Test
    fun `windows path is recovered from unc and drive file uris`() {
        assertEquals(
            "//wsl.localhost/Ubuntu/home/me/SKILL.md",
            WslPaths.windowsPathFromFileUri("file://wsl.localhost/Ubuntu/home/me/SKILL.md")
        )
        assertEquals(
            "//wsl.localhost/Ubuntu/home/me/SKILL.md",
            WslPaths.windowsPathFromFileUri("file:////wsl.localhost/Ubuntu/home/me/SKILL.md")
        )
        assertEquals("C:/src/project", WslPaths.windowsPathFromFileUri("file:///C:/src/project"))
        assertNull(WslPaths.windowsPathFromFileUri("http://example.com/x"))
        assertNull(WslPaths.windowsPathFromFileUri("not a uri"))
    }

    @Test
    fun `linux path is recovered only from plain file uris`() {
        assertEquals("/home/me/SKILL.md", WslPaths.linuxPathFromFileUri("file:///home/me/SKILL.md"))
        assertEquals("/home/me/my project/SKILL.md", WslPaths.linuxPathFromFileUri("file:///home/me/my%20project/SKILL.md"))
        assertNull(WslPaths.linuxPathFromFileUri("file://wsl.localhost/Ubuntu/home/me"))
        assertNull(WslPaths.linuxPathFromFileUri("file:///C:/src"))
    }

    @Test
    fun `windows file uri is rewritten to the linux file uri`() {
        assertEquals(
            "file:///home/me/project/SKILL.md",
            WslPaths.toLinuxFileUri("file://wsl.localhost/Ubuntu/home/me/project/SKILL.md", "Ubuntu").toString()
        )
        assertEquals(
            "file:///mnt/c/src/SKILL.md",
            WslPaths.toLinuxFileUri("file:///C:/src/SKILL.md", "Ubuntu").toString()
        )
        assertNull(WslPaths.toLinuxFileUri("file://wsl.localhost/Debian/home/me", "Ubuntu"))
    }
}
