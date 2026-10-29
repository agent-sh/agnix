package io.agnix.jetbrains.filetype

import com.intellij.ide.FileIconProvider
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import io.agnix.jetbrains.AgnixIcons
import javax.swing.Icon

/**
 * Decorates skills without replacing the native Markdown file type.
 * Markdown preview and editor providers use that native type, not just its language.
 */
class AgnixSkillIconProvider : FileIconProvider {
    override fun getIcon(file: VirtualFile, flags: Int, project: Project?): Icon? {
        return if (!file.isDirectory && file.name == "SKILL.md") AgnixIcons.AGNIX else null
    }
}

/**
 * Utility object for checking if a file is an agnix-supported file.
 */
object AgnixFileTypes {

    private val GLOBAL_FILE_NAMES = setOf(
        "SKILL.md",
        "CLAUDE.md",
        "CLAUDE.local.md",
        "AGENTS.md",
        "AGENTS.local.md",
        "AGENTS.override.md",
        "plugin.json",
        "mcp.json"
    )

    private val SUFFIX_PATTERNS = listOf(
        ".mcp.json",
        ".instructions.md",
        ".mdc"
    )

    private val CLAUDE_SETTINGS_FILE_NAMES = setOf(
        "settings.json",
        "settings.local.json"
    )

    private val GITHUB_FILE_NAMES = setOf(
        "copilot-instructions.md"
    )

    /**
     * Check if a file path matches agnix patterns.
     *
     * Takes into account parent directory requirements.
     */
    fun isAgnixFilePath(path: String): Boolean {
        val normalizedPath = path.replace('\\', '/')
        val fileName = normalizedPath.substringAfterLast('/')
        val rooted = "/$normalizedPath"
        if (listOf("/.pi/", "/.grok/", "/.agents/", "/.agent/", "/.gemini/config/", "/.gemini/antigravity-cli/").any { rooted.contains(it) }
            && listOf(".md", ".json", ".toml").any { fileName.endsWith(it) }) {
            return true
        }

        // Check exact file names at any level
        if (fileName in GLOBAL_FILE_NAMES) {
            return true
        }

        // Check extension patterns
        if (SUFFIX_PATTERNS.any { fileName.endsWith(it) }) {
            return true
        }

        // Check directory-specific patterns
        if (normalizedPath.contains("/.claude/") && fileName in CLAUDE_SETTINGS_FILE_NAMES) {
            return true
        }
        if (normalizedPath.contains("/.github/") && fileName in GITHUB_FILE_NAMES) {
            return true
        }
        if (normalizedPath.contains("/.github/instructions/") && fileName.endsWith(".instructions.md")) {
            return true
        }
        if (normalizedPath.contains("/.cursor/rules/") && fileName.endsWith(".mdc")) {
            return true
        }
        if (fileName == ".cursorrules") {
            return true
        }

        return false
    }
}
