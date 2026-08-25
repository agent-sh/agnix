package io.agnix.jetbrains.settings

import com.intellij.openapi.fileChooser.FileChooser
import com.intellij.openapi.fileChooser.FileChooserDescriptor
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import com.intellij.ui.components.JBCheckBox
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBTextField
import com.intellij.util.ui.FormBuilder
import io.agnix.jetbrains.binary.PlatformInfo
import javax.swing.JComboBox
import javax.swing.JComponent
import javax.swing.JPanel

/**
 * UI component for agnix settings.
 *
 * Provides form fields for configuring the plugin.
 */
class AgnixSettingsComponent {

    private val mainPanel: JPanel
    private val enabledCheckBox = JBCheckBox("Enable agnix validation")
    private val lspPathField = TextFieldWithBrowseButton()
    private val autoDownloadCheckBox = JBCheckBox("Automatically download LSP binary if not found")
    private val wslEnabledCheckBox = JBCheckBox("Run agnix-lsp inside WSL (Windows only)")
    private val wslDistributionField = JBTextField()
    private val wslLspPathField = JBTextField()
    private val traceLevelComboBox = JComboBox(AgnixSettings.TraceLevel.entries.toTypedArray())
    private val codeLensCheckBox = JBCheckBox("Show CodeLens annotations")
    private val lspPathChooserDescriptor = FileChooserDescriptor(
        true,
        false,
        false,
        false,
        false,
        false
    ).withTitle("Select agnix-lsp Binary")
        .withDescription("Choose the path to the agnix-lsp executable")

    init {
        configureLspPathFileChooser()
        val wslAvailable = PlatformInfo.supportsWslExecution()
        wslEnabledCheckBox.isEnabled = wslAvailable
        wslDistributionField.isEnabled = wslAvailable
        wslLspPathField.isEnabled = wslAvailable

        // Build the form
        mainPanel = FormBuilder.createFormBuilder()
            .addComponent(enabledCheckBox)
            .addSeparator()
            .addLabeledComponent(JBLabel("LSP binary path:"), lspPathField, 1, false)
            .addTooltip("Leave empty to use auto-detection or downloaded binary")
            .addComponent(autoDownloadCheckBox)
            .addSeparator()
            .addComponent(wslEnabledCheckBox)
            .addTooltip("Starts agnix-lsp with wsl.exe instead of a host-local executable")
            .addLabeledComponent(JBLabel("WSL distribution:"), wslDistributionField, 1, false)
            .addTooltip("Name from 'wsl --list --quiet'; leave empty to use the project's own distribution")
            .addLabeledComponent(JBLabel("WSL agnix-lsp path:"), wslLspPathField, 1, false)
            .addTooltip("Absolute Linux path, for example /home/you/.cargo/bin/agnix-lsp")
            .addSeparator()
            .addLabeledComponent(JBLabel("Trace level:"), traceLevelComboBox, 1, false)
            .addTooltip("Set to 'Messages' or 'Verbose' for debugging LSP communication")
            .addComponent(codeLensCheckBox)
            .addComponentFillVertically(JPanel(), 0)
            .panel
    }

    private fun configureLspPathFileChooser() {
        lspPathField.addActionListener {
            val initialSelection = lspPathField.text
                .takeIf { it.isNotBlank() }
                ?.let { LocalFileSystem.getInstance().findFileByPath(it) }
            val selectedFile = FileChooser.chooseFile(
                lspPathChooserDescriptor,
                null,
                initialSelection
            )
            if (selectedFile != null) {
                lspPathField.text = selectedFile.path
            }
        }
    }

    fun getPanel(): JComponent = mainPanel

    fun getPreferredFocusedComponent(): JComponent = enabledCheckBox

    var enabled: Boolean
        get() = enabledCheckBox.isSelected
        set(value) {
            enabledCheckBox.isSelected = value
        }

    var lspPath: String
        get() = lspPathField.text
        set(value) {
            lspPathField.text = value
        }

    var autoDownload: Boolean
        get() = autoDownloadCheckBox.isSelected
        set(value) {
            autoDownloadCheckBox.isSelected = value
        }

    var wslEnabled: Boolean
        get() = wslEnabledCheckBox.isSelected
        set(value) {
            wslEnabledCheckBox.isSelected = value
        }

    var wslDistribution: String
        get() = wslDistributionField.text
        set(value) {
            wslDistributionField.text = value
        }

    var wslLspPath: String
        get() = wslLspPathField.text
        set(value) {
            wslLspPathField.text = value
        }

    var traceLevel: AgnixSettings.TraceLevel
        get() = traceLevelComboBox.selectedItem as AgnixSettings.TraceLevel
        set(value) {
            traceLevelComboBox.selectedItem = value
        }

    var codeLensEnabled: Boolean
        get() = codeLensCheckBox.isSelected
        set(value) {
            codeLensCheckBox.isSelected = value
        }
}
