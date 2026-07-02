package io.agnix.jetbrains.settings

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class AgnixLspPathValidationTest {

    @Test
    fun `blank path is allowed for auto detection`() {
        assertNull(AgnixLspPathValidation.validate(""))
        assertNull(AgnixLspPathValidation.validate("   "))
    }

    @Test
    fun `bare agnix-lsp exe is rejected with absolute path guidance`() {
        val error = AgnixLspPathValidation.validate("agnix-lsp.exe")

        assertTrue(error!!.contains("full absolute executable path"))
        assertTrue(error.contains("C:\\Program Files\\agnix\\agnix-lsp.exe"))
        assertTrue(error.contains("Do not use just agnix-lsp.exe"))
    }

    @Test
    fun `relative paths are rejected`() {
        assertFalse(AgnixLspPathValidation.isAbsolutePath("agnix-lsp.exe"))
        assertFalse(AgnixLspPathValidation.isAbsolutePath("./agnix-lsp"))
        assertFalse(AgnixLspPathValidation.isAbsolutePath("bin\\agnix-lsp.exe"))
        assertFalse(AgnixLspPathValidation.isAbsolutePath("C:agnix-lsp.exe"))
    }

    @Test
    fun `absolute paths are accepted`() {
        assertTrue(AgnixLspPathValidation.isAbsolutePath("/usr/local/bin/agnix-lsp"))
        assertTrue(AgnixLspPathValidation.isAbsolutePath("C:\\Program Files\\agnix\\agnix-lsp.exe"))
        assertTrue(AgnixLspPathValidation.isAbsolutePath("C:/Program Files/agnix/agnix-lsp.exe"))
        assertTrue(AgnixLspPathValidation.isAbsolutePath("\\\\server\\share\\agnix-lsp.exe"))
    }
}
