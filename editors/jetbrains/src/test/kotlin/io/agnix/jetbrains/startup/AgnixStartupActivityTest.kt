package io.agnix.jetbrains.startup

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class AgnixStartupActivityTest {

    @Test
    fun `valid Windows WSL mode skips the host binary probe`() {
        assertFalse(
            AgnixStartupActivity.shouldCheckHostBinary(
                pluginEnabled = true,
                wslEnabled = true,
                wslHostSupported = true
            )
        )
    }

    @Test
    fun `synced WSL setting off Windows keeps the native probe`() {
        assertTrue(
            AgnixStartupActivity.shouldCheckHostBinary(
                pluginEnabled = true,
                wslEnabled = true,
                wslHostSupported = false
            )
        )
    }

    @Test
    fun `disabled plugin never probes the host binary`() {
        assertFalse(
            AgnixStartupActivity.shouldCheckHostBinary(
                pluginEnabled = false,
                wslEnabled = false,
                wslHostSupported = false
            )
        )
    }
}
