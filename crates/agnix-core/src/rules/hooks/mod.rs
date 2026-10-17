//! Hooks validation rules (CC-HK-001 to CC-HK-035)

use crate::rules::claude_json_spans::{JsonLocator, Seg};
use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::hooks::{Hook, HooksSchema, SettingsSchema},
};
use rust_i18n::t;
use std::path::Path;

mod extra;
mod helpers;
pub(crate) use extra::{HookFinding, HookOrigin, PathSeg, PluginServers, check_hooks};
use helpers::*;

const RULE_IDS: &[&str] = &[
    "CC-HK-001",
    "CC-HK-002",
    "CC-HK-003",
    "CC-HK-005",
    "CC-HK-006",
    "CC-HK-007",
    "CC-HK-008",
    "CC-HK-009",
    "CC-HK-010",
    "CC-HK-011",
    "CC-HK-012",
    "CC-HK-013",
    "CC-HK-014",
    "CC-HK-015",
    "CC-HK-016",
    // CC-HK-017 is retired: Claude Code appends the input JSON to a prompt or
    // agent hook prompt that has no $ARGUMENTS placeholder.
    "CC-HK-018",
    "CC-HK-020",
    "CC-HK-021",
    "CC-HK-022",
    "CC-HK-023",
    "CC-HK-024",
    "CC-HK-025",
    "CC-HK-026",
    "CC-HK-027",
    "CC-HK-028",
    "CC-HK-029",
    "CC-HK-030",
    "CC-HK-031",
    "CC-HK-032",
    "CC-HK-033",
    "CC-HK-034",
    "CC-HK-035",
];

pub struct HooksValidator;

/// CC-HK-006: Missing command field
fn validate_cc_hk_006_command_field(
    command: &Option<String>,
    hook_location: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if command.is_none() {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CC-HK-006",
                t!("rules.cc_hk_006.message", location = hook_location),
            )
            .with_suggestion(t!("rules.cc_hk_006.suggestion")),
        );
    }
}

/// CC-HK-008: Script file not found
fn validate_cc_hk_008_script_exists(
    command: &str,
    args: Option<&Vec<String>>,
    project_dir: &Path,
    config: &PerFileLintConfig<'_>,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fs = config.fs();
    let script_paths = match args {
        Some(args) => extract_exec_script_paths(command, args),
        None => extract_script_paths(command),
    };
    for script_path in script_paths {
        if !has_unresolved_env_vars(&script_path) {
            let resolved = resolve_script_path(&script_path, project_dir);
            if !fs.exists(&resolved) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-HK-008",
                        t!(
                            "rules.cc_hk_008.message",
                            script = script_path.as_str(),
                            resolved = resolved.display().to_string()
                        ),
                    )
                    .with_suggestion(t!("rules.cc_hk_008.suggestion")),
                );
            }
        }
    }
}

/// CC-HK-009: Dangerous command patterns
fn validate_cc_hk_009_dangerous_patterns(
    command: &str,
    path: &Path,
    (line, col): (usize, usize),
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some((pattern, reason)) = check_dangerous_patterns(command) {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                col,
                "CC-HK-009",
                t!("rules.cc_hk_009.message", reason = reason),
            )
            .with_suggestion(t!("rules.cc_hk_009.suggestion", pattern = pattern)),
        );
    }
}

/// CC-HK-028: `${user_config.*}` interpolation in a shell-form command string.
///
/// Claude Code v2.1.207 rejects `${user_config.<key>}` inside a command
/// hook's shell-form `command` string at load time (shell-injection fix).
/// Values must instead be read inside the script via
/// `$CLAUDE_PLUGIN_OPTION_<KEY>` (plugin hooks) or passed through the
/// environment.
fn validate_cc_hk_028_user_config_interpolation(
    command: &str,
    is_exec_form: bool,
    hook_location: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Exec form is explicitly permitted: "Plugin hooks additionally substitute
    // `${user_config.*}` values, in exec form only", and only "A shell-form
    // plugin hook whose `command` references `${user_config.*}` fails with an
    // error". Presence of `args` is what selects exec form, so firing without
    // checking it flagged the pattern the doc recommends.
    if is_exec_form {
        return;
    }
    if command.contains("${user_config.") {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CC-HK-028",
                t!("rules.cc_hk_028.message", location = hook_location),
            )
            .with_suggestion(t!("rules.cc_hk_028.suggestion")),
        );
    }
}

/// CC-HK-015: Model on command hook
fn validate_cc_hk_015_model_on_command(
    hook_location: &str,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut diagnostic = Diagnostic::warning(
        path.to_path_buf(),
        1,
        0,
        "CC-HK-015",
        t!("rules.cc_hk_015.message", location = hook_location),
    )
    .with_suggestion(t!("rules.cc_hk_015.suggestion"));

    // Safe auto-fix: remove the model field line
    if let Some((start, end)) = find_unique_json_field_line_span(content, "model") {
        diagnostic = diagnostic.with_fix(Fix::delete(start, end, t!("rules.cc_hk_015.fix"), true));
    }

    diagnostics.push(diagnostic);
}

/// CC-HK-018: Matcher on events where Claude Code silently ignores it
fn validate_cc_hk_018_matcher_ignored(
    event: &str,
    matcher: &Option<String>,
    matcher_idx: usize,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if HooksSchema::ignores_matcher(event) && matcher.is_some() {
        let hook_location = format!("hooks.{}[{}]", event, matcher_idx);
        let mut diagnostic = Diagnostic::info(
            path.to_path_buf(),
            1,
            0,
            "CC-HK-018",
            t!(
                "rules.cc_hk_018.message",
                location = hook_location.as_str(),
                event = event
            ),
        )
        .with_suggestion(t!("rules.cc_hk_018.suggestion", event = event));

        // Safe auto-fix: remove the matcher field line
        if let Some(matcher_val) = matcher {
            if let Some((start, end)) = find_unique_matcher_line_span(content, matcher_val) {
                diagnostic = diagnostic.with_fix(Fix::delete(
                    start,
                    end,
                    t!("rules.cc_hk_018.fix", event = event),
                    true,
                ));
            }
        }

        diagnostics.push(diagnostic);
    }
}

/// CC-HK-002: Prompt hook on wrong event
fn validate_cc_hk_002_hook_event_type(
    event: &str,
    hook_type: &str,
    supported: bool,
    hook_location: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !supported {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CC-HK-002",
                t!(
                    "rules.cc_hk_002.message",
                    location = hook_location,
                    event = event,
                    hook_type = hook_type
                ),
            )
            .with_suggestion(t!("rules.cc_hk_002.suggestion", hook_type = hook_type)),
        );
    }
}

/// CC-HK-007: Missing prompt field
fn validate_cc_hk_007_prompt_field(
    prompt: &Option<String>,
    hook_location: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if prompt.is_none() {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CC-HK-007",
                t!("rules.cc_hk_007.message", location = hook_location),
            )
            .with_suggestion(t!("rules.cc_hk_007.suggestion")),
        );
    }
}

/// The plugin root of a `hooks/hooks.json` (the directory that contains
/// `hooks/`), when it is a plugin root of any vendor's layout.
fn hooks_file_plugin_root(path: &Path) -> Option<&Path> {
    let hooks_dir = path.parent()?;
    (path.file_name().is_some_and(|n| n == "hooks.json")
        && hooks_dir.file_name().is_some_and(|n| n == "hooks"))
    .then(|| hooks_dir.parent())
    .flatten()
}

/// A `settings.json` at a Claude plugin root (beside `.claude-plugin/`).
fn is_plugin_root_settings(path: &Path, config: &PerFileLintConfig<'_>) -> bool {
    path.file_name().is_some_and(|n| n == "settings.json")
        && path
            .parent()
            .is_some_and(|dir| config.fs().exists(&dir.join(".claude-plugin")))
}

/// Whether `path` is a Gemini CLI extension's or a Codex plugin's
/// `hooks/hooks.json` rather than a Claude Code plugin's.
fn is_other_vendor_hooks_file(path: &Path, config: &PerFileLintConfig<'_>) -> bool {
    let Some(root) = hooks_file_plugin_root(path) else {
        return false;
    };
    let fs = config.fs();
    if fs.exists(&root.join(".claude-plugin")) {
        return false;
    }
    fs.exists(&root.join("gemini-extension.json")) || fs.exists(&root.join(".codex-plugin"))
}

/// For a Claude plugin's `hooks/hooks.json`: the plugin name and the keys of
/// the MCP servers it declares in `.mcp.json` or inline in `plugin.json`.
fn plugin_servers_for_hooks_file(
    path: &Path,
    config: &PerFileLintConfig<'_>,
) -> Option<PluginServers> {
    let root = hooks_file_plugin_root(path)?;
    let fs = config.fs();
    if !fs.exists(&root.join(".claude-plugin")) {
        return None;
    }
    Some(crate::rules::plugin::plugin_mcp_servers(root, fs.as_ref()))
}

/// Turn a handler or matcher finding into a diagnostic that points at the
/// offending key or value of a JSON hooks file.
fn finding_to_diagnostic(
    path: &Path,
    content: &str,
    locator: &JsonLocator<'_>,
    finding: HookFinding,
) -> Diagnostic {
    let segs: Vec<Seg<'_>> = std::iter::once(Seg::Key("hooks"))
        .chain(finding.path.iter().map(|seg| match seg {
            PathSeg::Key(key) => Seg::Key(key.as_str()),
            PathSeg::Index(index) => Seg::Index(*index),
        }))
        .collect();
    let (line, column) = if finding.on_key {
        locator.key_pos(&segs)
    } else {
        locator.value_pos(&segs)
    };
    let mut diagnostic = match finding.level {
        crate::diagnostics::DiagnosticLevel::Error => Diagnostic::error,
        crate::diagnostics::DiagnosticLevel::Warning => Diagnostic::warning,
        crate::diagnostics::DiagnosticLevel::Info => Diagnostic::info,
    }(
        path.to_path_buf(),
        line,
        column,
        finding.rule,
        finding.message,
    )
    .with_suggestion(finding.suggestion);
    // Replace the inner text of a JSON string value, only when the source has
    // no escapes, so the edit is exactly the decoded value.
    if let Some(replacement) = finding.replacement
        && let Some((start, end)) = locator.value_span(&segs)
        && end >= start + 2
        && content.get(start..start + 1) == Some("\"")
        && !content[start + 1..end - 1].contains('\\')
    {
        diagnostic = diagnostic.with_fix(Fix::replace(
            start + 1,
            end - 1,
            replacement.as_str(),
            t!("rules.cc_hk_033.fix", fixed = replacement.as_str()),
            false,
        ));
    }
    diagnostic
}

#[cfg(test)]
#[allow(dead_code)]
impl HooksValidator {
    fn check_dangerous_patterns(&self, command: &str) -> Option<(&'static str, &'static str)> {
        check_dangerous_patterns(command)
    }

    fn extract_script_paths(&self, command: &str) -> Vec<String> {
        extract_script_paths(command)
    }

    fn resolve_script_path(&self, script_path: &str, project_dir: &Path) -> std::path::PathBuf {
        resolve_script_path(script_path, project_dir)
    }

    fn has_unresolved_env_vars(&self, path: &str) -> bool {
        has_unresolved_env_vars(path)
    }
}

impl Validator for HooksValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    /// Main validation entry point for hooks configuration.
    ///
    /// ## Validation Phases
    ///
    /// 1. **Category check** - Early return if hooks category disabled
    /// 2. **JSON parsing** - Parse raw JSON, report CC-HK-012 on failure
    /// 3. **Pre-parse validation** - Raw JSON checks (CC-HK-005, CC-HK-011, CC-HK-013..014, CC-HK-016, CC-HK-020..025)
    /// 4. **Handler checks** - CC-HK-010, CC-HK-025 (case) and CC-HK-029..035 (extra.rs)
    /// 5. **Typed parsing** - Parse into SettingsSchema
    /// 6. **Event iteration** - Validate each event and hook (CC-HK-001..003, CC-HK-006..009, CC-HK-015, CC-HK-018, CC-HK-028)
    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !config.rules().hooks {
            return diagnostics;
        }

        // Claude Code never reads settings or hooks files from a Kiro
        // directory (`.kiro/settings/settings.json` holds Kiro IDE editor
        // settings, for example), so Claude hook rules do not apply there.
        if crate::file_types::is_inside_kiro_dir(path) {
            return diagnostics;
        }

        // A `hooks/hooks.json` inside a Gemini CLI extension or a Codex plugin
        // follows that tool's hook schema (`BeforeTool`, `Interrupt`, ...), and
        // Claude Code never reads it. Path detection can't tell these apart
        // from a Claude plugin, so check the plugin root here.
        if is_other_vendor_hooks_file(path, config) {
            return diagnostics;
        }

        // `.vscode/settings.json` is VS Code's editor settings file (JSON with
        // comments and trailing commas); Claude Code never reads it.
        if path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|dir| dir.eq_ignore_ascii_case(".vscode"))
        {
            return diagnostics;
        }

        let raw_value: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(e) => {
                if config.is_rule_enabled("CC-HK-012") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-HK-012",
                            t!("rules.cc_hk_012.message", error = e.to_string()),
                        )
                        .with_suggestion(t!("rules.cc_hk_012.suggestion")),
                    );
                }
                return diagnostics;
            }
        };

        let locator = JsonLocator::new(content);

        // CC-PL-021 (owned by PluginValidator): "Only two keys take effect from
        // a plugin, `agent` and `subagentStatusLine`; every other key is
        // dropped", so hooks in a plugin's root settings.json never run.
        if is_plugin_root_settings(path, config) {
            if config.is_rule_enabled("CC-PL-021")
                && let Some(map) = raw_value.as_object()
            {
                for key in map
                    .keys()
                    .filter(|k| !matches!(k.as_str(), "agent" | "subagentStatusLine" | "$schema"))
                {
                    let (line, col) = locator.key_pos(&[Seg::Key(key.as_str())]);
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            col,
                            "CC-PL-021",
                            t!(
                                "rules.cc_pl_021.message",
                                key = key.as_str(),
                                location = "settings.json"
                            ),
                        )
                        .with_suggestion(t!("rules.cc_pl_021.suggestion")),
                    );
                }
            }
            return diagnostics;
        }

        // Raw JSON validation: CC-HK-005, 011, 013, 014, 016, 020-025.
        // Single consolidated traversal (2 passes max instead of 11).
        if !validate_all_raw_hooks(
            &raw_value,
            path,
            content,
            &locator,
            config,
            &mut diagnostics,
        ) {
            return diagnostics;
        }

        // CC-HK-010 and CC-HK-029..035: per-handler and per-matcher checks
        // shared with skill and agent frontmatter hooks.
        if let Some(events) = raw_value.get("hooks").and_then(|v| v.as_object()) {
            let plugin = plugin_servers_for_hooks_file(path, config);
            let origin = if plugin.is_some() {
                HookOrigin::PluginHooksFile
            } else {
                HookOrigin::Settings
            };
            for finding in check_hooks(events, origin, plugin.as_ref(), config) {
                diagnostics.push(finding_to_diagnostic(path, content, &locator, finding));
            }
        }

        let settings: SettingsSchema = match serde_json::from_value(raw_value) {
            Ok(s) => s,
            Err(e) => {
                if config.is_rule_enabled("CC-HK-012") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-HK-012",
                            t!("rules.cc_hk_012.message", error = e.to_string()),
                        )
                        .with_suggestion(t!("rules.cc_hk_012.suggestion")),
                    );
                }
                return diagnostics;
            }
        };

        let project_dir = path
            .parent()
            .and_then(|p| {
                if p.ends_with(".claude") {
                    p.parent()
                } else {
                    Some(p)
                }
            })
            .unwrap_or_else(|| Path::new("."));

        for (event, matchers) in &settings.hooks {
            // --- Event-level validation ---
            // CC-HK-001: Invalid event name
            if config.is_rule_enabled("CC-HK-001") {
                if !validate_cc_hk_001_event_name(event, path, content, &mut diagnostics) {
                    continue;
                }
            } else if !HooksSchema::VALID_EVENTS.contains(&event.as_str()) {
                continue; // Skip invalid events even if rule disabled
            }

            for (matcher_idx, matcher) in matchers.iter().enumerate() {
                // --- Matcher-level validation ---
                // CC-HK-003: Matcher hint for tool events. A PermissionRequest
                // group without a matcher is reported by CC-HK-035 instead.
                if config.is_rule_enabled("CC-HK-003")
                    && !(event == "PermissionRequest" && config.is_rule_enabled("CC-HK-035"))
                {
                    validate_cc_hk_003_matcher_hint(
                        event,
                        &matcher.matcher,
                        matcher_idx,
                        path,
                        &mut diagnostics,
                    );
                }

                // CC-HK-018: Matcher on UserPromptSubmit/Stop
                if config.is_rule_enabled("CC-HK-018") {
                    validate_cc_hk_018_matcher_ignored(
                        event,
                        &matcher.matcher,
                        matcher_idx,
                        path,
                        content,
                        &mut diagnostics,
                    );
                }

                // --- Hook-level validation ---
                for (hook_idx, hook) in matcher.hooks.iter().enumerate() {
                    let hook_location = format!(
                        "hooks.{}{}.hooks[{}]",
                        event,
                        matcher
                            .matcher
                            .as_ref()
                            .map(|m| format!("[matcher={}]", m))
                            .unwrap_or_else(|| format!("[{}]", matcher_idx)),
                        hook_idx
                    );

                    match hook {
                        Hook::Command {
                            command,
                            args,
                            model,
                            ..
                        } => {
                            // CC-HK-006: Missing command field
                            if config.is_rule_enabled("CC-HK-006") {
                                validate_cc_hk_006_command_field(
                                    command,
                                    &hook_location,
                                    path,
                                    &mut diagnostics,
                                );
                            }

                            // CC-HK-015: Model on command hook
                            if config.is_rule_enabled("CC-HK-015") && model.is_some() {
                                validate_cc_hk_015_model_on_command(
                                    &hook_location,
                                    path,
                                    content,
                                    &mut diagnostics,
                                );
                            }

                            if let Some(cmd) = command {
                                // CC-HK-008: Script file not found
                                if config.is_rule_enabled("CC-HK-008") {
                                    validate_cc_hk_008_script_exists(
                                        cmd,
                                        args.as_ref(),
                                        project_dir,
                                        config,
                                        path,
                                        &mut diagnostics,
                                    );
                                }

                                // CC-HK-009: Dangerous command patterns
                                if config.is_rule_enabled("CC-HK-009") {
                                    let pos = locator.value_pos(&[
                                        Seg::Key("hooks"),
                                        Seg::Key(event.as_str()),
                                        Seg::Index(matcher_idx),
                                        Seg::Key("hooks"),
                                        Seg::Index(hook_idx),
                                        Seg::Key("command"),
                                    ]);
                                    validate_cc_hk_009_dangerous_patterns(
                                        cmd,
                                        path,
                                        pos,
                                        &mut diagnostics,
                                    );
                                }

                                // CC-HK-028: ${user_config.*} interpolation rejected at load time
                                if config.is_rule_enabled("CC-HK-028") {
                                    validate_cc_hk_028_user_config_interpolation(
                                        cmd,
                                        args.is_some(),
                                        &hook_location,
                                        path,
                                        &mut diagnostics,
                                    );
                                }
                            }
                        }
                        Hook::Prompt { prompt, .. } => {
                            // CC-HK-002: Prompt on wrong event
                            if config.is_rule_enabled("CC-HK-002") {
                                validate_cc_hk_002_hook_event_type(
                                    event,
                                    "prompt",
                                    HooksSchema::is_prompt_event(event),
                                    &hook_location,
                                    path,
                                    &mut diagnostics,
                                );
                            }

                            // CC-HK-007: Missing prompt field
                            if config.is_rule_enabled("CC-HK-007") {
                                validate_cc_hk_007_prompt_field(
                                    prompt,
                                    &hook_location,
                                    path,
                                    &mut diagnostics,
                                );
                            }
                        }
                        Hook::Agent { prompt, .. } => {
                            // CC-HK-002: Agent hook on wrong event.
                            if config.is_rule_enabled("CC-HK-002") {
                                validate_cc_hk_002_hook_event_type(
                                    event,
                                    "agent",
                                    HooksSchema::is_agent_event(event),
                                    &hook_location,
                                    path,
                                    &mut diagnostics,
                                );
                            }

                            // CC-HK-007: Missing prompt field
                            if config.is_rule_enabled("CC-HK-007") {
                                validate_cc_hk_007_prompt_field(
                                    prompt,
                                    &hook_location,
                                    path,
                                    &mut diagnostics,
                                );
                            }
                        }
                        Hook::Http { .. } => {
                            // Other HTTP checks run on the raw JSON (CC-HK-005,
                            // CC-HK-011, CC-HK-016, CC-HK-020, CC-HK-024).
                            // CC-HK-002: "`SessionStart` and `Setup` support
                            // `command` and `mcp_tool` hooks ... They don't
                            // support `http`, `prompt`, or `agent` hooks."
                            if config.is_rule_enabled("CC-HK-002") {
                                validate_cc_hk_002_hook_event_type(
                                    event,
                                    "http",
                                    !matches!(event.as_str(), "SessionStart" | "Setup"),
                                    &hook_location,
                                    path,
                                    &mut diagnostics,
                                );
                            }
                        }
                        Hook::McpTool { .. } => {
                            // MCP tool hooks (added Claude Code v2.1.118) are
                            // validated at the raw JSON level via CC-HK-016
                            // (string allow-list). Schema details for required
                            // fields are not yet documented at
                            // code.claude.com/docs/en/hooks (as of 2026-04-23);
                            // re-tighten when upstream docs land.
                        }
                    }
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_extra;
