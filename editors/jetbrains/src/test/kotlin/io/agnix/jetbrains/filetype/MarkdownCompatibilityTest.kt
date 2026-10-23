package io.agnix.jetbrains.filetype

import com.intellij.ide.FileIconProvider
import com.intellij.testFramework.fixtures.BasePlatformTestCase
import io.agnix.jetbrains.AgnixIcons
import io.agnix.jetbrains.settings.AgnixSettings
import org.intellij.plugins.markdown.lang.MarkdownFileType
import org.intellij.plugins.markdown.lang.MarkdownLanguage

/** Exercises the real plugin registration, rather than only its language field. */
class MarkdownCompatibilityTest : BasePlatformTestCase() {
    private var originallyEnabled = true

    override fun setUp() {
        super.setUp()
        // File-type compatibility needs no server download or network access.
        val settings = AgnixSettings.getInstance()
        originallyEnabled = settings.enabled
        settings.enabled = false
    }

    override fun tearDown() {
        try {
            AgnixSettings.getInstance().enabled = originallyEnabled
        } finally {
            super.tearDown()
        }
    }

    fun testSkillRetainsNativeMarkdownFileType() {
        val file = myFixture.configureByText("SKILL.md", "# Skill\n\n**Preview content**\n")
        assertSame(MarkdownFileType.INSTANCE, file.virtualFile.fileType)
        assertSame(MarkdownLanguage.INSTANCE, file.language)
    }

    fun testOtherMarkdownFilesRetainNativeFileType() {
        myFixture.configureByText("SKILL.md", "# Skill\n")
        val names = listOf(
            "CLAUDE.md", "CLAUDE.local.md", "AGENTS.md",
            "AGENTS.local.md", "AGENTS.override.md", "README.md"
        )
        for (name in names) {
            val file = myFixture.configureByText(name, "# Instructions\n\n**Preview content**\n")
            assertSame(name, MarkdownFileType.INSTANCE, file.virtualFile.fileType)
            assertSame(name, MarkdownLanguage.INSTANCE, file.language)
        }
    }

    fun testMarkdownKeepsItsPrimaryFileType() {
        myFixture.configureByText("SKILL.md", "# Skill\n")
        assertSame(MarkdownFileType.INSTANCE, MarkdownLanguage.INSTANCE.associatedFileType)
    }

    fun testSkillKeepsItsIconWithoutChangingFileType() {
        val file = myFixture.configureByText("SKILL.md", "# Skill\n").virtualFile
        val provider = FileIconProvider.EP_NAME.extensionList.filterIsInstance<AgnixSkillIconProvider>().single()
        assertSame(AgnixIcons.AGNIX, provider.getIcon(file, 0, project))
        assertSame(MarkdownFileType.INSTANCE, file.fileType)
    }

    fun testIconProviderLeavesOtherFilesAndDirectoriesAlone() {
        val provider = FileIconProvider.EP_NAME.extensionList.filterIsInstance<AgnixSkillIconProvider>().single()
        val readme = myFixture.configureByText("README.md", "# Project\n").virtualFile
        assertNull(provider.getIcon(readme, 0, project))
        val directory = myFixture.tempDirFixture.findOrCreateDir("SKILL.md")
        assertNull(provider.getIcon(directory, 0, project))
    }
}
