package io.agnix.jetbrains.lsp

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class AgnixDocumentMatcherTest {

    @Test
    fun `WSL mode excludes files from another distribution`() {
        assertFalse(
            matches(
                path = "//wsl.localhost/Debian/home/me/project/SKILL.md",
                projectBasePath = "//wsl.localhost/Ubuntu/home/me/project"
            )
        )
    }

    @Test
    fun `WSL mode includes mappable files from the active distribution`() {
        assertTrue(
            matches(
                path = "//wsl.localhost/Ubuntu/home/me/project/SKILL.md",
                projectBasePath = "//wsl.localhost/Ubuntu/home/me/project"
            )
        )
    }

    @Test
    fun `synced WSL preference is ignored on non Windows hosts`() {
        assertTrue(
            matches(
                path = "/home/me/project/SKILL.md",
                projectBasePath = "/home/me/project",
                wslHostSupported = false
            )
        )
    }

    @Test
    fun `invalid active WSL configuration does not attach documents`() {
        assertFalse(
            matches(
                path = "//wsl.localhost/Ubuntu/home/me/project/SKILL.md",
                projectBasePath = "//wsl.localhost/Ubuntu/home/me/project",
                lspPath = "relative/agnix-lsp"
            )
        )
    }

    private fun matches(
        path: String,
        projectBasePath: String?,
        wslHostSupported: Boolean = true,
        lspPath: String = "/home/me/.cargo/bin/agnix-lsp"
    ): Boolean = AgnixDocumentMatcher.matchesPath(
        path = path,
        projectBasePath = projectBasePath,
        wslEnabled = true,
        wslHostSupported = wslHostSupported,
        distribution = "Ubuntu",
        lspPath = lspPath
    )
}
