package io.agnix.jetbrains.settings

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class AgnixSettingsConfigurableTest {

    private val native = WslLaunchSettings(
        enabled = false,
        distribution = "",
        lspPath = ""
    )

    @Test
    fun `toggling WSL mode changes the launch identity`() {
        assertTrue(shouldRestartForWslChange(true, native, native.copy(enabled = true)))
    }

    @Test
    fun `switching distribution changes the launch identity`() {
        val ubuntu = WslLaunchSettings(true, "Ubuntu", "/usr/bin/agnix-lsp")
        assertTrue(shouldRestartForWslChange(true, ubuntu, ubuntu.copy(distribution = "Debian")))
    }

    @Test
    fun `switching WSL binary changes the launch identity`() {
        val original = WslLaunchSettings(true, "Ubuntu", "/usr/bin/agnix-lsp")
        assertTrue(
            shouldRestartForWslChange(
                true,
                original,
                original.copy(lspPath = "/opt/agnix/agnix-lsp")
            )
        )
    }

    @Test
    fun `unrelated settings leave the launch identity unchanged`() {
        assertFalse(shouldRestartForWslChange(true, native, native.copy()))
    }

    @Test
    fun `non Windows hosts never restart for synced WSL changes`() {
        assertFalse(shouldRestartForWslChange(false, native, native.copy(enabled = true)))
    }
}
