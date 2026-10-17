//! CC-SET-034: settings keys and values Claude Code ignores in the file
//! that sets them.
//!
//! Data-driven from the Scope column of the settings reference "Settings
//! index" (code.claude.com/docs/en/settings-reference, page dated
//! 2026-10-14): "Claude Code ignores the key in a repository file. Look for
//! `User, local, or managed`, `User or managed`, `User`, `Managed`, or
//! `Global config` in the Scope column". Keys with scope `Any file` are not
//! listed. Per-key exceptions from the Scope lines are encoded below.

use crate::{
    diagnostics::Diagnostic,
    schemas::kiro_json::{JsonMember, JsonNode, LineIndex},
};
use rust_i18n::t;
use std::collections::HashSet;
use std::path::Path;

/// Which settings file a path is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsScope {
    /// `.claude/settings.json` in a repository.
    Project,
    /// `.claude/settings.local.json`.
    Local,
    /// `~/.claude/settings.json`.
    User,
    /// A managed settings file.
    Managed,
}

/// The files a key is read from, from the Scope column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyScope {
    Managed,
    UserOrManaged,
    UserLocalOrManaged,
    User,
    GlobalConfig,
    /// A global config key current releases no longer read.
    RemovedGlobalConfig,
}

impl KeyScope {
    fn ignored_in(self, file: SettingsScope) -> bool {
        use SettingsScope as S;
        match self {
            Self::Managed => !matches!(file, S::Managed),
            Self::UserOrManaged => matches!(file, S::Project | S::Local),
            Self::UserLocalOrManaged => matches!(file, S::Project),
            Self::User => !matches!(file, S::User),
            Self::GlobalConfig | Self::RemovedGlobalConfig => true,
        }
    }

    /// (message key, suggestion key)
    fn locale_keys(self) -> (&'static str, &'static str) {
        match self {
            Self::Managed => (
                "rules.cc_set_034.managed_message",
                "rules.cc_set_034.managed_suggestion",
            ),
            Self::UserOrManaged => (
                "rules.cc_set_034.user_or_managed_message",
                "rules.cc_set_034.user_or_managed_suggestion",
            ),
            Self::UserLocalOrManaged => (
                "rules.cc_set_034.user_local_or_managed_message",
                "rules.cc_set_034.user_local_or_managed_suggestion",
            ),
            Self::User => (
                "rules.cc_set_034.user_message",
                "rules.cc_set_034.user_suggestion",
            ),
            Self::GlobalConfig => (
                "rules.cc_set_034.global_config_message",
                "rules.cc_set_034.global_config_suggestion",
            ),
            Self::RemovedGlobalConfig => (
                "rules.cc_set_034.removed_global_config_message",
                "rules.cc_set_034.removed_global_config_suggestion",
            ),
        }
    }
}

/// Keys whose Scope is not `Any file`. Dotted names are nested keys; a key
/// covers everything under it.
///
/// Left out on purpose:
/// - `autoContinueAtUsageLimit`: "a project or local settings file that sets
///   it turns the feature off rather than being ignored".
/// - `bashEditDiffEnabled`: only `true` is ignored in repository files (see
///   [`TRUE_IGNORED_IN_REPOSITORY`]).
const SCOPED_KEYS: &[(&str, KeyScope)] = &[
    // Managed
    ("allowAllClaudeAiMcps", KeyScope::Managed),
    ("allowClaudeInChromeWithManagedMcp", KeyScope::Managed),
    ("allowedChannelPlugins", KeyScope::Managed),
    ("allowedProviders", KeyScope::Managed),
    ("allowManagedHooksOnly", KeyScope::Managed),
    ("allowManagedMcpServersOnly", KeyScope::Managed),
    ("allowManagedPermissionRulesOnly", KeyScope::Managed),
    ("availableModelsMatch", KeyScope::Managed),
    ("blockedMarketplaces", KeyScope::Managed),
    ("browserExternalPageTools", KeyScope::Managed),
    ("channelsEnabled", KeyScope::Managed),
    ("claudeMd", KeyScope::Managed),
    ("deniedModels", KeyScope::Managed),
    ("disableBrowserExternalNavigation", KeyScope::Managed),
    ("disableCommandPluginSources", KeyScope::Managed),
    ("disableDesktopLocalSessions", KeyScope::Managed),
    ("disableMobileSimulatorTools", KeyScope::Managed),
    ("disableSideloadFlags", KeyScope::Managed),
    ("forceRemoteSettingsRefresh", KeyScope::Managed),
    ("gatewayInternalNetworks", KeyScope::Managed),
    ("managedMcpServers", KeyScope::Managed),
    ("managedSourcesBehavior", KeyScope::Managed),
    ("modelPricing", KeyScope::Managed),
    ("parentSettingsBehavior", KeyScope::Managed),
    ("pluginSuggestionMarketplaces", KeyScope::Managed),
    ("pluginTrustMessage", KeyScope::Managed),
    ("policyHelper", KeyScope::Managed),
    ("requiredMaximumVersion", KeyScope::Managed),
    ("requiredMinimumVersion", KeyScope::Managed),
    ("sandbox.bwrapPath", KeyScope::Managed),
    (
        "sandbox.filesystem.allowManagedReadPathsOnly",
        KeyScope::Managed,
    ),
    ("sandbox.network.allowManagedDomainsOnly", KeyScope::Managed),
    ("sandbox.socatPath", KeyScope::Managed),
    ("sshHostAllowlist", KeyScope::Managed),
    ("strictKnownMarketplaces", KeyScope::Managed),
    ("strictPluginOnlyCustomization", KeyScope::Managed),
    ("wslInheritsWindowsSettings", KeyScope::Managed),
    // User or managed
    ("appendPlugins", KeyScope::UserOrManaged),
    ("askUserQuestionTimeout", KeyScope::UserOrManaged),
    ("autoMode", KeyScope::UserOrManaged),
    ("desktopSessionCleanupPeriodDays", KeyScope::UserOrManaged),
    ("dialogExpiry", KeyScope::UserOrManaged),
    ("feedbackDrafts", KeyScope::UserOrManaged),
    ("footerLinksRegexes", KeyScope::UserOrManaged),
    ("forceLoginGatewayUrl", KeyScope::UserOrManaged),
    ("modelPicker", KeyScope::UserOrManaged),
    ("pluginConfigs", KeyScope::UserOrManaged),
    ("prependPlugins", KeyScope::UserOrManaged),
    ("processWrapper", KeyScope::UserOrManaged),
    ("sandbox.allowAppleEvents", KeyScope::UserOrManaged),
    (
        "sandbox.credentials.allowPlaintextInject",
        KeyScope::UserOrManaged,
    ),
    ("sandbox.credentials.awsPairs", KeyScope::UserOrManaged),
    ("sandbox.credentials.sigv4", KeyScope::UserOrManaged),
    ("sandbox.filesystem.disabled", KeyScope::UserOrManaged),
    ("sandbox.network.strictAllowlist", KeyScope::UserOrManaged),
    ("sandbox.network.tlsTerminate", KeyScope::UserOrManaged),
    ("sandbox.ripgrep", KeyScope::UserOrManaged),
    ("skipAutoPermissionPrompt", KeyScope::UserOrManaged),
    ("spellcheck", KeyScope::UserOrManaged),
    ("sshConfigs", KeyScope::UserOrManaged),
    ("vimInsertModeRemaps", KeyScope::UserOrManaged),
    // User, local, or managed
    (
        "skipDangerousModePermissionPrompt",
        KeyScope::UserLocalOrManaged,
    ),
    ("syncClaudeAiPlugins", KeyScope::UserLocalOrManaged),
    ("syncClaudeAiSkills", KeyScope::UserLocalOrManaged),
    ("useAutoModeDuringPlan", KeyScope::UserLocalOrManaged),
    // User
    ("worktree.location", KeyScope::User),
    // Global config (~/.claude.json)
    ("autoConnectIde", KeyScope::GlobalConfig),
    ("autoInstallIdeExtension", KeyScope::GlobalConfig),
    ("claudeInChromeDefaultEnabled", KeyScope::GlobalConfig),
    ("copyFullResponse", KeyScope::GlobalConfig),
    ("copyOnSelect", KeyScope::GlobalConfig),
    ("defaultToAgentsView", KeyScope::GlobalConfig),
    ("diffTool", KeyScope::GlobalConfig),
    ("externalEditorContext", KeyScope::GlobalConfig),
    ("leftArrowOpensAgents", KeyScope::GlobalConfig),
    ("prStatusFooterEnabled", KeyScope::GlobalConfig),
    // "On v2.1.256 and earlier" / "On v2.1.233 and earlier"
    ("permissionExplainerEnabled", KeyScope::RemovedGlobalConfig),
    ("teammateDefaultModel", KeyScope::RemovedGlobalConfig),
];

/// Keys where only `true` is ignored in `.claude/settings.json` and
/// `.claude/settings.local.json`, and `false` still applies.
/// - `remoteControlAtStartup`: "a project or local `true` is ignored"
/// - `bashEditDiffEnabled`: "a `true` in a repository's `.claude/settings.json`
///   or `.claude/settings.local.json` can't turn the recording on"
const TRUE_IGNORED_IN_REPOSITORY: &[&str] = &["remoteControlAtStartup", "bashEditDiffEnabled"];

/// `permissions.defaultMode` values that "don't take effect from project or
/// local settings".
const DEFAULT_MODES_IGNORED_IN_REPOSITORY: &[&str] = &["auto", "bypassPermissions"];

/// `env` variables "Project and local settings can't set" ("Variables Claude
/// Code ignores in `env`"). The page lists them as examples ("They
/// include"), so only these are reported.
const ENV_IGNORED_IN_REPOSITORY: &[&str] = &[
    "CLAUDE_CONFIG_DIR",
    "CLAUDE_CODE_TMPDIR",
    "HOME",
    "TMPDIR",
    "TMP",
    "TEMP",
    "SystemRoot",
    "ComSpec",
    "ProgramData",
    "LOCALAPPDATA",
    "PATHEXT",
    "PSModulePath",
    "OTEL_LOG_RAW_API_BODIES",
    "ENABLE_BETA_TRACING_DETAILED",
    "BETA_TRACING_ENDPOINT",
    "CLAUDE_CODE_ENABLE_TELEMETRY",
    "CLAUDE_CODE_ENHANCED_TELEMETRY_BETA",
    "ENABLE_ENHANCED_TELEMETRY_BETA",
    "OTEL_LOGS_EXPORTER",
    "OTEL_METRICS_EXPORTER",
    "OTEL_TRACES_EXPORTER",
    "OTEL_LOG_USER_PROMPTS",
    "OTEL_LOG_ASSISTANT_RESPONSES",
    "OTEL_LOG_TOOL_CONTENT",
    "OTEL_LOG_TOOL_DETAILS",
    "OTEL_EXPORTER_PROMETHEUS_HOST",
    "OTEL_EXPORTER_PROMETHEUS_PORT",
    "CLAUDE_CODE_PROCESS_WRAPPER",
    "CLAUDE_CODE_SYNC_SKILLS",
    "CLAUDE_CODE_SYNC_PLUGINS",
    "CLAUDE_CODE_PLUGIN_CACHE_DIR",
    "CLAUDE_CODE_PLUGIN_SEED_DIR",
    "CLAUDE_CODE_USER_DIALOG_TIMEOUT_MS",
    "CLAUDE_AFK_TIMEOUT_MS",
    "CLAUDE_AFK_COUNTDOWN_MS",
    "CLAUDE_CODE_DISABLE_ATTACHMENTS",
    "CLAUDE_CODE_DISABLE_FILE_MENTIONS",
];

/// `env` variables "ignored from every file": identity variables the hosting
/// environment owns and variables read from the launch environment only.
const ENV_IGNORED_EVERYWHERE: &[&str] = &[
    "CLAUDE_CODE_REMOTE",
    "CLAUDE_CODE_ACCOUNT_UUID",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_CODE_PROJECT_DIR_NAME",
    "CLAUDE_CODE_RESTRICTED",
    "CLAUDE_CODE_DISABLE_POWERSHELL_CMD_RM_DENY",
    "CLAUDE_CODE_DISABLE_DANGEROUS_RM_TIMEOUT",
    "CLAUDE_CODE_DISABLE_SUBSTITUTION_RM_PROMPT",
    "CLAUDE_CODE_DISABLE_INLINE_SHELL_RM_PROMPT",
];

fn env_ignored_in_repository(name: &str) -> bool {
    if ENV_IGNORED_IN_REPOSITORY.contains(&name)
        || name.starts_with("XDG_")
        || name.starts_with("ProgramFiles")
    {
        return true;
    }
    // "`OTEL_EXPORTER_OTLP_*` variables whose names end in `_ENDPOINT`,
    // `_HEADERS`, `_PROTOCOL`, `_CERTIFICATE`, `_CLIENT_KEY`, or `_INSECURE`"
    name.starts_with("OTEL_EXPORTER_OTLP_")
        && [
            "_ENDPOINT",
            "_HEADERS",
            "_PROTOCOL",
            "_CERTIFICATE",
            "_CLIENT_KEY",
            "_INSECURE",
        ]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// "Only these values still apply from project and local settings, because
/// they turn something off: `none` for the three exporter selectors, and an
/// off value such as `0` for `OTEL_LOG_USER_PROMPTS`, `OTEL_LOG_TOOL_CONTENT`,
/// and `OTEL_LOG_TOOL_DETAILS`."
fn env_value_turns_telemetry_off(name: &str, value: Option<&str>) -> bool {
    let Some(value) = value.map(str::trim) else {
        return false;
    };
    match name {
        "OTEL_LOGS_EXPORTER" | "OTEL_METRICS_EXPORTER" | "OTEL_TRACES_EXPORTER" => {
            value.eq_ignore_ascii_case("none")
        }
        "OTEL_LOG_USER_PROMPTS" | "OTEL_LOG_TOOL_CONTENT" | "OTEL_LOG_TOOL_DETAILS" => {
            ["0", "false", "off", "no"]
                .iter()
                .any(|off| value.eq_ignore_ascii_case(off))
        }
        _ => false,
    }
}

/// Marker in [`IgnoredKeys`] for `mask` entries of `sandbox.credentials`:
/// "Claude Code honors `mask` entries, `allowPlaintextInject`, `awsPairs`,
/// and `sigv4` only from user settings, managed settings, and the
/// `--settings` flag."
pub(super) const MASK_ENTRIES_KEY: &str = "sandbox.credentials.(mask entries)";

/// `spinnerTipsOverride` fields project and local settings can't set: "from
/// project and local settings it reads plain string tips only".
const SPINNER_TIPS_REPOSITORY_IGNORED: &[&str] = &["tipsFile", "label", "excludeDefault"];

/// What CC-SET-034 found: the dotted key paths Claude Code ignores in this
/// file. Type checks skip them, so the scope finding is reported instead of
/// a type error on a value that has no effect.
#[derive(Debug, Default)]
pub(super) struct IgnoredKeys(HashSet<String>);

impl IgnoredKeys {
    /// True when `key` or any key above it is ignored here.
    pub(super) fn covers(&self, key: &str) -> bool {
        let mut prefix = key;
        loop {
            if self.0.contains(prefix) {
                return true;
            }
            match prefix.rfind('.') {
                Some(dot) => prefix = &prefix[..dot],
                None => return false,
            }
        }
    }
}

/// Find the member at a dotted path.
fn member_at<'a>(root: &'a JsonNode, dotted: &str) -> Option<&'a JsonMember> {
    let mut node = root;
    let mut found = None;
    for segment in dotted.split('.') {
        let member = node.value.get(segment)?;
        node = &member.value;
        found = Some(member);
    }
    found
}

fn file_label(scope: SettingsScope, path: &Path) -> String {
    match scope {
        SettingsScope::Project => ".claude/settings.json".to_string(),
        SettingsScope::Local => ".claude/settings.local.json".to_string(),
        SettingsScope::User => "~/.claude/settings.json".to_string(),
        SettingsScope::Managed => path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("managed settings")
            .to_string(),
    }
}

/// CC-SET-034. Returns the ignored keys (for the type checks) and pushes one
/// diagnostic per ignored key or value when `report` is set.
pub(super) fn check_ignored_at_scope(
    path: &Path,
    content: &str,
    root: &JsonNode,
    scope: SettingsScope,
    report: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> IgnoredKeys {
    let mut ignored = IgnoredKeys::default();
    let lines = LineIndex::new(content);
    let file = file_label(scope, path);
    let mut push = |offset: usize, message: String, suggestion: String| {
        if !report {
            return;
        }
        let (line, column) = lines.line_col(offset);
        diagnostics.push(
            Diagnostic::warning(path.to_path_buf(), line, column, "CC-SET-034", message)
                .with_suggestion(suggestion),
        );
    };

    for (key, key_scope) in SCOPED_KEYS {
        if !key_scope.ignored_in(scope) {
            continue;
        }
        let Some(member) = member_at(root, key) else {
            continue;
        };
        if member.value.value.is_null() {
            continue;
        }
        // A parent already reported covers its children.
        if ignored.covers(key) {
            continue;
        }
        ignored.0.insert((*key).to_string());
        let (message_key, suggestion_key) = key_scope.locale_keys();
        push(
            member.key_start,
            t!(message_key, key = *key, file = file.as_str()).to_string(),
            t!(suggestion_key, key = *key).to_string(),
        );
    }

    let repository_file = matches!(scope, SettingsScope::Project | SettingsScope::Local);
    if repository_file {
        for key in TRUE_IGNORED_IN_REPOSITORY {
            let Some(member) = member_at(root, key) else {
                continue;
            };
            if member.value.value.as_bool() != Some(true) {
                continue;
            }
            ignored.0.insert((*key).to_string());
            push(
                member.value.start,
                t!(
                    "rules.cc_set_034.true_ignored_message",
                    key = *key,
                    file = file.as_str()
                )
                .to_string(),
                t!("rules.cc_set_034.true_ignored_suggestion", key = *key).to_string(),
            );
        }

        // `mask` credential entries
        for array in ["files", "envVars"] {
            let Some(member) = member_at(root, &format!("sandbox.credentials.{array}")) else {
                continue;
            };
            let Some(entries) = member.value.value.as_array() else {
                continue;
            };
            for entry in entries {
                let Some(mode) = entry.value.get("mode") else {
                    continue;
                };
                if mode.value.value.as_str() != Some("mask") {
                    continue;
                }
                ignored.0.insert(MASK_ENTRIES_KEY.to_string());
                push(
                    mode.value.start,
                    t!(
                        "rules.cc_set_034.mask_entry_message",
                        array = array,
                        file = file.as_str()
                    )
                    .to_string(),
                    t!("rules.cc_set_034.mask_entry_suggestion").to_string(),
                );
            }
        }

        // `spinnerTipsOverride`: plain string tips only
        if let Some(member) = member_at(root, "spinnerTipsOverride") {
            for field in SPINNER_TIPS_REPOSITORY_IGNORED {
                if let Some(field_member) = member.value.value.get(field) {
                    push(
                        field_member.key_start,
                        t!(
                            "rules.cc_set_034.spinner_tips_message",
                            field = *field,
                            file = file.as_str()
                        )
                        .to_string(),
                        t!("rules.cc_set_034.spinner_tips_suggestion").to_string(),
                    );
                }
            }
            if let Some(tips) = member.value.value.get("tips")
                && let Some(items) = tips.value.value.as_array()
                && let Some(object) = items.iter().find(|item| item.value.as_object().is_some())
            {
                push(
                    object.start,
                    t!(
                        "rules.cc_set_034.spinner_tips_message",
                        field = "tips (object entries)",
                        file = file.as_str()
                    )
                    .to_string(),
                    t!("rules.cc_set_034.spinner_tips_suggestion").to_string(),
                );
            }
        }

        // "For a self-hosted environment ID, user or managed settings, or the
        // `--settings` flag only."
        if let Some(member) = member_at(root, "remote.defaultEnvironmentId")
            && let Some(id) = member.value.value.as_str()
            && id.starts_with("ccpool_")
        {
            push(
                member.value.start,
                t!(
                    "rules.cc_set_034.self_hosted_environment_message",
                    file = file.as_str()
                )
                .to_string(),
                t!("rules.cc_set_034.self_hosted_environment_suggestion").to_string(),
            );
        }

        if let Some(member) = member_at(root, "permissions.defaultMode")
            && let Some(mode) = member.value.value.as_str()
            && DEFAULT_MODES_IGNORED_IN_REPOSITORY.contains(&mode)
        {
            push(
                member.value.start,
                t!(
                    "rules.cc_set_034.default_mode_message",
                    mode = mode,
                    file = file.as_str()
                )
                .to_string(),
                t!("rules.cc_set_034.default_mode_suggestion", mode = mode).to_string(),
            );
        }
    }

    if let Some(env) = root.value.get("env")
        && let Some(variables) = env.value.value.as_object()
    {
        for variable in variables {
            let name = variable.key.as_str();
            let value = variable.value.value.as_str();
            let (message_key, suggestion_key) = if ENV_IGNORED_EVERYWHERE.contains(&name) {
                (
                    "rules.cc_set_034.env_everywhere_message",
                    "rules.cc_set_034.env_everywhere_suggestion",
                )
            } else if repository_file
                && env_ignored_in_repository(name)
                && !env_value_turns_telemetry_off(name, value)
            {
                (
                    "rules.cc_set_034.env_repository_message",
                    "rules.cc_set_034.env_repository_suggestion",
                )
            } else {
                continue;
            };
            push(
                variable.key_start,
                t!(message_key, var = name, file = file.as_str()).to_string(),
                t!(suggestion_key, var = name).to_string(),
            );
        }
    }

    ignored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_keys_are_unique_and_sorted_within_groups() {
        let mut seen = HashSet::new();
        for (key, _) in SCOPED_KEYS {
            assert!(seen.insert(*key), "duplicate scoped key {key}");
        }
    }

    #[test]
    fn ignored_keys_cover_children() {
        let mut ignored = IgnoredKeys::default();
        ignored.0.insert("autoMode".to_string());
        assert!(ignored.covers("autoMode"));
        assert!(ignored.covers("autoMode.classifyAllShell"));
        assert!(!ignored.covers("autoModeX"));
        assert!(!ignored.covers("sandbox.ripgrep"));
    }

    #[test]
    fn telemetry_off_values_still_apply() {
        assert!(env_value_turns_telemetry_off(
            "OTEL_METRICS_EXPORTER",
            Some("none")
        ));
        assert!(env_value_turns_telemetry_off(
            "OTEL_LOG_USER_PROMPTS",
            Some("0")
        ));
        assert!(!env_value_turns_telemetry_off(
            "OTEL_LOG_USER_PROMPTS",
            Some("1")
        ));
        assert!(!env_value_turns_telemetry_off(
            "OTEL_LOG_ASSISTANT_RESPONSES",
            Some("0")
        ));
        assert!(!env_value_turns_telemetry_off(
            "OTEL_METRICS_EXPORTER",
            Some("otlp")
        ));
    }

    #[test]
    fn otlp_family_matches_documented_suffixes() {
        assert!(env_ignored_in_repository("OTEL_EXPORTER_OTLP_ENDPOINT"));
        assert!(env_ignored_in_repository(
            "OTEL_EXPORTER_OTLP_METRICS_HEADERS"
        ));
        assert!(!env_ignored_in_repository("OTEL_EXPORTER_OTLP_TIMEOUT"));
        assert!(env_ignored_in_repository("XDG_CONFIG_HOME"));
        assert!(!env_ignored_in_repository("NODE_ENV"));
    }
}
