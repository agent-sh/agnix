package io.agnix.jetbrains.notifications

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.options.ShowSettingsUtil
import com.intellij.openapi.project.Project
import io.agnix.jetbrains.binary.AgnixBinaryDownloader
import io.agnix.jetbrains.binary.PlatformInfo
import io.agnix.jetbrains.settings.AgnixSettingsConfigurable

/**
 * Notification utilities for the agnix plugin.
 *
 * Provides methods for showing various notifications to the user.
 */
object AgnixNotifications {

    private const val NOTIFICATION_GROUP_ID = "agnix.notifications"

    /**
     * Notify that the LSP binary was not found.
     */
    fun notifyBinaryNotFound(project: Project) {
        val notification = NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "agnix-lsp not found",
                "The agnix language server binary was not found. " +
                    "You can download it automatically or install manually.",
                NotificationType.WARNING
            )

        notification.addAction(object : AnAction("Download") {
            override fun actionPerformed(e: AnActionEvent) {
                val downloader = AgnixBinaryDownloader()
                downloader.downloadAsync(project) { path ->
                    if (path != null) {
                        notification.expire()
                    }
                }
            }
        })

        notification.addAction(object : AnAction("Settings") {
            override fun actionPerformed(e: AnActionEvent) {
                ShowSettingsUtil.getInstance().showSettingsDialog(
                    project,
                    AgnixSettingsConfigurable::class.java
                )
                notification.expire()
            }
        })

        notification.notify(project)
    }

    /**
     * Notify that the current platform is not supported.
     */
    fun notifyPlatformNotSupported(project: Project) {
        val platformDesc = PlatformInfo.getPlatformDescription()

        NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "Platform not supported",
                "No pre-built agnix-lsp binary is available for $platformDesc. " +
                    "Please install manually: cargo install agnix-lsp",
                NotificationType.ERROR
            )
            .notify(project)
    }

    /**
     * Notify that the download was successful.
     */
    fun notifyDownloadSuccess(project: Project) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "agnix-lsp installed",
                "The agnix language server was successfully downloaded and installed.",
                NotificationType.INFORMATION
            )
            .notify(project)
    }

    /**
     * Notify that the download failed.
     */
    fun notifyDownloadFailed(project: Project, error: String) {
        val notification = NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "Download failed",
                "Failed to download agnix-lsp: $error",
                NotificationType.ERROR
            )

        notification.addAction(object : AnAction("Retry") {
            override fun actionPerformed(e: AnActionEvent) {
                val downloader = AgnixBinaryDownloader()
                downloader.downloadAsync(project) { path ->
                    if (path != null) {
                        notification.expire()
                    }
                }
            }
        })

        notification.addAction(object : AnAction("Install Manually") {
            override fun actionPerformed(e: AnActionEvent) {
                // Open documentation URL
                com.intellij.ide.BrowserUtil.browse("https://github.com/agent-sh/agnix#installation")
                notification.expire()
            }
        })

        notification.notify(project)
    }

    /**
     * Notify that the LSP binary exists but the OS refused to execute it.
     *
     * This is the locked-down-Windows / corporate-policy case (AppLocker, WDAC,
     * EDR, or AV blocking execution from a user-writable location such as
     * %AppData%), which surfaces as `CreateProcess error=5` (access denied).
     * Upgrading the IDE does not help; the fix is to run agnix-lsp from an
     * allowed location and point the plugin at it, or have IT allowlist the path.
     */
    fun notifyBinaryAccessDenied(project: Project, binaryPath: String) {
        val notification = NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "agnix-lsp could not be started (access denied)",
                "The OS or a security policy (e.g. AppLocker/WDAC, EDR, or antivirus) " +
                    "blocked execution of the agnix language server at:\n$binaryPath\n\n" +
                    "This is common on managed machines where binaries in user-writable " +
                    "locations cannot run. Place agnix-lsp in an allowed location (or ask " +
                    "IT to allowlist it), then set its path in agnix settings. Upgrading the " +
                    "IDE does not resolve this.",
                NotificationType.ERROR
            )

        notification.addAction(object : AnAction("Set Binary Path") {
            override fun actionPerformed(e: AnActionEvent) {
                ShowSettingsUtil.getInstance().showSettingsDialog(
                    project,
                    AgnixSettingsConfigurable::class.java
                )
                notification.expire()
            }
        })

        notification.addAction(object : AnAction("Troubleshooting") {
            override fun actionPerformed(e: AnActionEvent) {
                com.intellij.ide.BrowserUtil.browse(
                    "https://github.com/agent-sh/agnix/blob/main/editors/jetbrains/README.md#troubleshooting"
                )
                notification.expire()
            }
        })

        notification.notify(project)
    }

    /**
     * Notify that WSL execution mode is enabled but cannot be used as configured.
     *
     * Covers a missing distribution name that could not be derived from the project
     * location and a WSL agnix-lsp path that is not an absolute Linux path.
     */
    fun notifyWslLaunchProblem(project: Project, message: String) {
        val notification = NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "agnix-lsp WSL configuration problem",
                message,
                NotificationType.ERROR
            )

        notification.addAction(object : AnAction("Settings") {
            override fun actionPerformed(e: AnActionEvent) {
                ShowSettingsUtil.getInstance().showSettingsDialog(
                    project,
                    AgnixSettingsConfigurable::class.java
                )
                notification.expire()
            }
        })

        notification.notify(project)
    }

    /**
     * Notify that the LSP server encountered an error.
     */
    fun notifyServerError(project: Project, error: String) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "Language server error",
                "agnix-lsp encountered an error: $error",
                NotificationType.ERROR
            )
            .notify(project)
    }

    /**
     * Show an informational notification.
     */
    fun notifyInfo(project: Project, title: String, content: String) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(title, content, NotificationType.INFORMATION)
            .notify(project)
    }
}
