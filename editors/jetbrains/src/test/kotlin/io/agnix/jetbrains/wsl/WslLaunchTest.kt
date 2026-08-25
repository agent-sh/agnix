package io.agnix.jetbrains.wsl

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

/**
 * Tests for wsl.exe command construction and WSL settings resolution.
 */
class WslLaunchTest {

    @Test
    fun `command runs the linux binary without a shell`() {
        assertEquals(
            listOf("wsl.exe", "--distribution", "Ubuntu", "--exec", "/home/me/.cargo/bin/agnix-lsp"),
            WslLaunch.buildCommand("Ubuntu", "/home/me/.cargo/bin/agnix-lsp")
        )
    }

    @Test
    fun `distribution and path with spaces stay single arguments`() {
        val command = WslLaunch.buildCommand("Ubuntu 24.04", "/home/me/my tools/agnix-lsp")
        assertEquals("Ubuntu 24.04", command[2])
        assertEquals("/home/me/my tools/agnix-lsp", command[4])
        assertEquals(5, command.size)
    }

    @Test
    fun `surrounding whitespace is trimmed from command arguments`() {
        assertEquals(
            listOf("wsl.exe", "--distribution", "Ubuntu", "--exec", "/usr/local/bin/agnix-lsp"),
            WslLaunch.buildCommand("  Ubuntu  ", "  /usr/local/bin/agnix-lsp  ")
        )
    }

    @Test
    fun `option-looking distribution is rejected instead of injected`() {
        val error = assertThrows(IllegalArgumentException::class.java) {
            WslLaunch.buildCommand("--user root", "/usr/local/bin/agnix-lsp")
        }
        assertEquals(WslLaunch.DISTRIBUTION_INVALID_MESSAGE, error.message)
    }

    @Test
    fun `windows lsp path is rejected in wsl mode`() {
        val error = assertThrows(IllegalArgumentException::class.java) {
            WslLaunch.buildCommand("Ubuntu", "C:\\Program Files\\agnix\\agnix-lsp.exe")
        }
        assertEquals(WslLaunch.LINUX_PATH_REQUIRED_MESSAGE, error.message)
    }

    @Test
    fun `distribution validation rejects empty separators and control characters`() {
        assertEquals(WslLaunch.DISTRIBUTION_REQUIRED_MESSAGE, WslLaunch.validateDistribution("   "))
        assertEquals(WslLaunch.DISTRIBUTION_INVALID_MESSAGE, WslLaunch.validateDistribution("Ubuntu/extra"))
        assertEquals(WslLaunch.DISTRIBUTION_INVALID_MESSAGE, WslLaunch.validateDistribution("Ubuntu\\extra"))
        assertEquals(WslLaunch.DISTRIBUTION_INVALID_MESSAGE, WslLaunch.validateDistribution("Ubu\"ntu"))
        assertEquals(WslLaunch.DISTRIBUTION_INVALID_MESSAGE, WslLaunch.validateDistribution("Ubuntu\u0000"))
        assertNull(WslLaunch.validateDistribution("Ubuntu-24.04"))
    }

    @Test
    fun `lsp path validation requires an absolute linux path`() {
        assertEquals(WslLaunch.LINUX_PATH_REQUIRED_MESSAGE, WslLaunch.validateLspPath(""))
        assertEquals(WslLaunch.LINUX_PATH_REQUIRED_MESSAGE, WslLaunch.validateLspPath("agnix-lsp"))
        assertEquals(WslLaunch.LINUX_PATH_REQUIRED_MESSAGE, WslLaunch.validateLspPath("~/bin/agnix-lsp"))
        assertEquals(WslLaunch.LINUX_PATH_REQUIRED_MESSAGE, WslLaunch.validateLspPath("/usr/bin/agnix\nlsp"))
        assertNull(WslLaunch.validateLspPath("/usr/bin/agnix-lsp"))
    }

    @Test
    fun `disabled mode keeps the native launch path`() {
        val resolution = WslLaunch.resolve(
            enabled = false,
            distribution = "Ubuntu",
            lspPath = "/usr/bin/agnix-lsp",
            projectBasePath = "//wsl.localhost/Ubuntu/home/me/project"
        )
        assertEquals(WslLaunch.Resolution.Disabled, resolution)
    }

    @Test
    fun `configured distribution wins`() {
        val resolution = WslLaunch.resolve(
            enabled = true,
            distribution = "Debian",
            lspPath = "/usr/bin/agnix-lsp",
            projectBasePath = "//wsl.localhost/Ubuntu/home/me/project"
        )
        assertEquals(WslLaunch.Resolution.Ready("Debian", "/usr/bin/agnix-lsp"), resolution)
    }

    @Test
    fun `empty distribution falls back to the project distribution`() {
        val resolution = WslLaunch.resolve(
            enabled = true,
            distribution = "",
            lspPath = "/usr/bin/agnix-lsp",
            projectBasePath = "//wsl.localhost/Ubuntu-24.04/home/me/project"
        )
        assertEquals(WslLaunch.Resolution.Ready("Ubuntu-24.04", "/usr/bin/agnix-lsp"), resolution)
    }

    @Test
    fun `missing distribution for a non-wsl project is reported`() {
        val resolution = WslLaunch.resolve(
            enabled = true,
            distribution = "",
            lspPath = "/usr/bin/agnix-lsp",
            projectBasePath = "C:\\src\\project"
        )
        assertEquals(WslLaunch.Resolution.Invalid(WslLaunch.DISTRIBUTION_REQUIRED_MESSAGE), resolution)
    }

    @Test
    fun `missing linux path is reported`() {
        val resolution = WslLaunch.resolve(
            enabled = true,
            distribution = "Ubuntu",
            lspPath = "",
            projectBasePath = "//wsl.localhost/Ubuntu/home/me/project"
        )
        assertEquals(WslLaunch.Resolution.Invalid(WslLaunch.LINUX_PATH_REQUIRED_MESSAGE), resolution)
    }

    @Test
    fun `validate reports the distribution problem first`() {
        assertEquals(
            WslLaunch.DISTRIBUTION_REQUIRED_MESSAGE,
            WslLaunch.validate("", "not-a-linux-path")
        )
        assertTrue(WslLaunch.validate("Ubuntu", "/usr/bin/agnix-lsp") == null)
    }
}
