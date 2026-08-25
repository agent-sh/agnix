package io.agnix.jetbrains.settings

import com.intellij.openapi.options.Configurable
import com.intellij.openapi.options.ConfigurationException
import io.agnix.jetbrains.binary.AgnixBinaryResolver
import io.agnix.jetbrains.binary.PlatformInfo
import io.agnix.jetbrains.wsl.WslLaunch
import javax.swing.JComponent

/**
 * Configurable for agnix settings in the IDE preferences.
 *
 * Accessible via: Settings/Preferences > Tools > agnix
 */
class AgnixSettingsConfigurable : Configurable {

    private var settingsComponent: AgnixSettingsComponent? = null

    override fun getDisplayName(): String = "agnix"

    override fun getPreferredFocusedComponent(): JComponent? {
        return settingsComponent?.getPreferredFocusedComponent()
    }

    override fun createComponent(): JComponent? {
        settingsComponent = AgnixSettingsComponent()
        return settingsComponent?.getPanel()
    }

    override fun isModified(): Boolean {
        val settings = AgnixSettings.getInstance()
        val component = settingsComponent ?: return false

        return component.enabled != settings.enabled ||
            component.lspPath.trim() != settings.lspPath ||
            component.autoDownload != settings.autoDownload ||
            component.wslEnabled != settings.wslEnabled ||
            component.wslDistribution.trim() != settings.wslDistribution ||
            component.wslLspPath.trim() != settings.wslLspPath ||
            component.traceLevel != settings.traceLevel ||
            component.codeLensEnabled != settings.codeLensEnabled
    }

    override fun apply() {
        val settings = AgnixSettings.getInstance()
        val component = settingsComponent ?: return

        val lspPath = component.lspPath.trim()
        AgnixLspPathValidation.validate(lspPath)?.let { message ->
            throw ConfigurationException(message)
        }

        val wslDistribution = component.wslDistribution.trim()
        val wslLspPath = component.wslLspPath.trim()
        if (component.wslEnabled && PlatformInfo.supportsWslExecution()) {
            // An empty distribution is allowed here: it means "use the distribution the
            // project is opened from", which is only known once a project is open.
            if (wslDistribution.isNotEmpty()) {
                WslLaunch.validateDistribution(wslDistribution)?.let { message ->
                    throw ConfigurationException(message)
                }
            }
            WslLaunch.validateLspPath(wslLspPath)?.let { message ->
                throw ConfigurationException(message)
            }
        }

        val lspPathChanged = settings.lspPath != lspPath

        settings.enabled = component.enabled
        settings.lspPath = lspPath
        settings.autoDownload = component.autoDownload
        settings.wslEnabled = component.wslEnabled
        settings.wslDistribution = wslDistribution
        settings.wslLspPath = wslLspPath
        settings.traceLevel = component.traceLevel
        settings.codeLensEnabled = component.codeLensEnabled

        // Clear binary resolver cache if LSP path changed
        if (lspPathChanged) {
            AgnixBinaryResolver.clearCache()
        }
    }

    override fun reset() {
        val settings = AgnixSettings.getInstance()
        val component = settingsComponent ?: return

        component.enabled = settings.enabled
        component.lspPath = settings.lspPath
        component.autoDownload = settings.autoDownload
        component.wslEnabled = settings.wslEnabled
        component.wslDistribution = settings.wslDistribution
        component.wslLspPath = settings.wslLspPath
        component.traceLevel = settings.traceLevel
        component.codeLensEnabled = settings.codeLensEnabled
    }

    override fun disposeUIResources() {
        settingsComponent = null
    }
}
