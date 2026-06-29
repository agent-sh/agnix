package io.agnix.jetbrains.lsp

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Assertions.*
import java.io.IOException

/**
 * Tests for LspProcessErrors access-denied classification.
 */
class LspProcessErrorsTest {

    @Test
    fun `null is not access denied`() {
        assertFalse(LspProcessErrors.isAccessDenied(null))
    }

    @Test
    fun `windows CreateProcess error 5 is access denied`() {
        val e = IOException("CreateProcess error=5, Access is denied")
        assertTrue(LspProcessErrors.isAccessDenied(e))
    }

    @Test
    fun `windows CreateProcess error 5 with localized danish text is access denied`() {
        // Reproduces the reported failure: error code is language-independent,
        // the trailing message is localized (Danish "Adgang naegtet").
        val e = IOException("CreateProcess error=5, Adgang naegtet")
        assertTrue(LspProcessErrors.isAccessDenied(e))
    }

    @Test
    fun `posix errno 13 is access denied`() {
        val e = IOException("Cannot run program \"agnix-lsp\": error=13, Permission denied")
        assertTrue(LspProcessErrors.isAccessDenied(e))
    }

    @Test
    fun `eacces token is access denied`() {
        val e = IOException("spawn EACCES")
        assertTrue(LspProcessErrors.isAccessDenied(e))
    }

    @Test
    fun `permission denied phrase is access denied`() {
        val e = IOException("Permission denied")
        assertTrue(LspProcessErrors.isAccessDenied(e))
    }

    @Test
    fun `uppercase access denied phrase is matched`() {
        // Matching must be case-insensitive and locale-independent (Locale.ROOT),
        // so an uppercased message still classifies as access-denied even on
        // Turkish/Azeri default locales where naive lowercasing of 'I' diverges.
        assertTrue(LspProcessErrors.isAccessDenied(IOException("ACCESS IS DENIED")))
        assertTrue(LspProcessErrors.isAccessDenied(IOException("PERMISSION DENIED")))
        assertTrue(LspProcessErrors.isAccessDenied(IOException("spawn EACCES")))
    }

    @Test
    fun `file not found is not access denied`() {
        // error=2 (ERROR_FILE_NOT_FOUND) is a missing-binary case, not a policy block.
        val e = IOException("CreateProcess error=2, The system cannot find the file specified")
        assertFalse(LspProcessErrors.isAccessDenied(e))
    }

    @Test
    fun `access denied detected through cause chain`() {
        val root = IOException("CreateProcess error=5, Adgang naegtet")
        val mid = RuntimeException("ProcessNotCreatedException", root)
        val top = RuntimeException("CannotStartProcessException", mid)
        assertTrue(LspProcessErrors.isAccessDenied(top))
    }

    @Test
    fun `unrelated error is not access denied`() {
        val e = RuntimeException("connection reset by peer")
        assertFalse(LspProcessErrors.isAccessDenied(e))
    }

    @Test
    fun `self-referential cause does not loop`() {
        val e = object : RuntimeException("boom") {
            override val cause: Throwable get() = this
        }
        // Must terminate and return false without StackOverflow.
        assertFalse(LspProcessErrors.isAccessDenied(e))
    }

    @Test
    fun `blank message is not access denied`() {
        assertFalse(LspProcessErrors.isAccessDenied(IOException("")))
        assertFalse(LspProcessErrors.isAccessDenied(IOException(null as String?)))
    }
}
