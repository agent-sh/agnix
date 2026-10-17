//! Agent file validation (CC-AG-001 to CC-AG-020)
//!
//! Validates Claude Code subagent definitions in `.claude/agents/*.md`.
//! Includes structural validation of hooks, tool names, memory, permissions,
//! effort, isolation, maxTurns, background, and unknown frontmatter fields.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    fs::FileSystem,
    parsers::frontmatter::{FrontmatterParts, split_frontmatter},
    rules::{Validator, ValidatorMetadata},
    schemas::agent::AgentSchema,
    schemas::hooks::HooksSchema,
    schemas::skill::VALID_EFFORT_LEVELS,
    validation::is_valid_mcp_tool_format,
};
use rust_i18n::t;
use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

/// Convert raw serde YAML errors into user-friendly messages.
fn humanize_yaml_error(raw: &str) -> String {
    let mut msg = raw.to_string();

    // "tools: invalid type: string "X", expected a sequence"
    // -> "tools: expected a YAML list (use '- item' syntax), got a comma-separated string"
    if msg.contains("expected a sequence") && msg.contains("invalid type: string") {
        if let Some(field) = msg.split(':').next() {
            return format!(
                "{}: expected a YAML list (use '- item' syntax on separate lines), not a comma-separated string",
                field.trim()
            );
        }
    }

    // "expected a string" for fields that should be strings
    if msg.contains("expected a string") && msg.contains("invalid type: sequence") {
        if let Some(field) = msg.split(':').next() {
            return format!(
                "{}: expected a single string value, not a YAML list",
                field.trim()
            );
        }
    }

    // Strip " at line X column Y" suffix for cleaner messages since we already report location
    if let Some(pos) = msg.find(" at line ") {
        msg.truncate(pos);
    }

    msg
}

/// Valid model values per CC-AG-003 (short aliases).
///
/// Shared with the skill validator (CC-SK-001) so the two lists cannot drift:
/// the sub-agents reference says `model` "Accepts the same values as the
/// `--model` flag", which is the model-config alias table.
use crate::schemas::skill::VALID_MODEL_ALIASES as VALID_MODELS;

/// Check if a model value is valid: either a known short alias or a full model ID
/// matching the `claude-*` pattern.
fn is_valid_model(model: &str) -> bool {
    VALID_MODELS.contains(&model) || model.starts_with("claude-")
}

/// Valid permission modes per CC-AG-004.
///
/// The six documented modes (permission-modes reference, "Available modes")
/// plus `manual`, which the CLI accepts as an alias for `default` wherever the
/// value is typed - "The Manual label and the `manual` alias require Claude
/// Code v2.1.200 or later."
///
/// `delegate` was previously listed here but appears in no current reference
/// (neither sub-agents nor permission-modes) and was added without a cited
/// source, so it is no longer accepted.
const VALID_PERMISSION_MODES: &[&str] = &[
    "default",
    "manual",
    "acceptEdits",
    "plan",
    "auto",
    "dontAsk",
    "bypassPermissions",
];

/// Valid memory scopes per CC-AG-008
const VALID_MEMORY_SCOPES: &[&str] = &["user", "project", "local"];

// Valid effort values per CC-AG-014: shared with CC-SK-018 via
// schemas::skill::VALID_EFFORT_LEVELS so agent and skill effort lists
// stay in lock-step (Claude Code uses the same set for both).

/// Valid isolation values per CC-AG-015
const VALID_ISOLATION_VALUES: &[&str] = &["worktree"];

/// Known agent frontmatter fields per CC-AG-019
const KNOWN_AGENT_FIELDS: &[&str] = &[
    "name",
    "description",
    "model",
    "tools",
    "disallowedTools",
    "permissionMode",
    "maxTurns",
    "effort",
    "background",
    "omitClaudeMd",
    "autoCompactWindow",
    "isolation",
    "initialPrompt",
    "experimental",
    "mcpServers",
    "memory",
    "skills",
    "hooks",
    "mode",
    "color",
];

/// Known Claude Code tools for CC-AG-009 and CC-AG-010.
///
/// Mirrors the built-in tools reference (code.claude.com/docs/en/tools,
/// verified 2026-08-27) plus legacy/internal names kept for version
/// tolerance. Alphabetized.
///
/// Tools that subagents never receive (`AskUserQuestion`, `Workflow`,
/// `ScheduleWakeup`, …) stay listed here: per the sub-agents reference those
/// are silently filtered from the resolved pool rather than rejected as
/// unknown names, so flagging them under CC-AG-009/010 would be wrong.
const KNOWN_AGENT_TOOLS: &[&str] = &[
    "Agent",
    "Artifact",
    "AskUserQuestion",
    "Bash",
    "CronCreate",
    "CronDelete",
    "CronList",
    "Edit",
    "EndConversation",
    "EnterPlanMode",
    "EnterWorktree",
    "ExitPlanMode",
    "ExitWorktree",
    "Glob",
    "Grep",
    "LSP",
    "ListAgents",
    "ListMcpResourcesTool",
    "Monitor",
    "MultiTool",
    "NotebookEdit",
    "PowerShell",
    "PushNotification",
    "Read",
    "ReadMcpResourceTool",
    "RemoteTrigger",
    "ReportFindings",
    "ScheduleWakeup",
    "SendFeedback",
    "SendMessage",
    "SendUserFile",
    "ShareOnboardingGuide",
    "Skill",
    "StatusBarMessageTool",
    "SubagentHandback",
    "Task",
    "TaskCreate",
    "TaskGet",
    "TaskList",
    "TaskOutput",
    "TaskStop",
    "TaskUpdate",
    "TodoRead",
    "TodoWrite",
    "ToolSearch",
    "WaitForMcpServers",
    "WebFetch",
    "WebSearch",
    "Workflow",
    "Write",
];

const RULE_IDS: &[&str] = &[
    "CC-AG-001",
    "CC-AG-002",
    "CC-AG-003",
    "CC-AG-004",
    "CC-AG-005",
    "CC-AG-006",
    "CC-AG-007",
    "CC-AG-008",
    "CC-AG-009",
    "CC-AG-010",
    "CC-AG-011",
    "CC-AG-012",
    // CC-AG-013 is retired: the sub-agents reference states no naming format
    // for the `skills` list, and CC-AG-005 already reports names that resolve
    // to no skill.
    "CC-AG-014",
    "CC-AG-015",
    // CC-AG-016 (invalid background type) is intentionally not listed here.
    // It is caught by serde at parse time (CC-AG-007) since background is
    // Option<bool>. No separate runtime diagnostic is needed.
    "CC-AG-017",
    "CC-AG-019",
    "CC-AG-020",
    "CC-AG-021",
    "CC-AG-022",
    "CC-AG-023",
    "CC-AG-024",
    // Plugin agent fields Claude Code ignores; checked on the agent Markdown
    // file, which is where the plugins reference documents them.
    "CC-PL-014",
];

pub struct AgentValidator;

/// Maximum directory traversal depth to prevent unbounded filesystem walking
const MAX_TRAVERSAL_DEPTH: usize = 10;

/// Find the byte range of a scalar frontmatter value for a key, using
/// pre-computed [`FrontmatterParts`]. Returns the value-only range (without
/// quotes) in full-content byte offsets.
fn frontmatter_value_byte_range_from_parts(
    parts: &FrontmatterParts,
    key: &str,
) -> Option<(usize, usize)> {
    if !parts.has_frontmatter || !parts.has_closing {
        return None;
    }

    let frontmatter = &parts.frontmatter;
    let mut offset = 0usize;
    let bytes = frontmatter.as_bytes();

    for line in frontmatter.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            let line_end = offset + line.len();
            if line_end < bytes.len() && bytes[line_end] == b'\n' {
                offset = line_end + 1;
            } else {
                offset = line_end;
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix(key)
            && let Some(after_colon) = rest.trim_start().strip_prefix(':')
        {
            let leading_ws = line.len() - trimmed.len();
            let ws_after_key = rest.len() - rest.trim_start().len();
            let key_end = leading_ws + key.len() + ws_after_key + 1; // ':'

            let value_str = after_colon.trim_start();
            if value_str.is_empty() {
                return None;
            }

            let value_offset_in_line = key_end + (after_colon.len() - value_str.len());
            let (value_start, value_len) = if let Some(inner) = value_str.strip_prefix('"') {
                let end_quote = inner.find('"')?;
                (value_offset_in_line + 1, end_quote)
            } else if let Some(inner) = value_str.strip_prefix('\'') {
                let end_quote = inner.find('\'')?;
                (value_offset_in_line + 1, end_quote)
            } else {
                let value_end = value_str
                    .find(" #")
                    .or_else(|| value_str.find("\t#"))
                    .unwrap_or(value_str.len());
                (value_offset_in_line, value_end)
            };

            let abs_start = parts.frontmatter_start + offset + value_start;
            let abs_end = abs_start + value_len;
            return Some((abs_start, abs_end));
        }

        let line_end = offset + line.len();
        if line_end < bytes.len() && bytes[line_end] == b'\n' {
            offset = line_end + 1;
        } else {
            offset = line_end;
        }
    }

    None
}

impl AgentValidator {
    /// Find the project root by looking for .claude directory.
    /// Limited to MAX_TRAVERSAL_DEPTH levels to prevent unbounded traversal.
    fn find_project_root<'a>(path: &'a Path, fs: &dyn FileSystem) -> Option<&'a Path> {
        let mut current = path.parent();
        let mut depth = 0;
        while let Some(dir) = current {
            if depth >= MAX_TRAVERSAL_DEPTH {
                break;
            }
            if fs.exists(&dir.join(".claude")) {
                return Some(dir);
            }
            // Also check if we're inside .claude
            if dir.file_name().map(|n| n == ".claude").unwrap_or(false) {
                return dir.parent();
            }
            current = dir.parent();
            depth += 1;
        }
        // No fallback - return None if .claude directory not found
        None
    }

    /// Validate skill name to prevent path traversal attacks.
    /// Returns true if the name is safe (alphanumeric, hyphens, underscores only).
    fn is_safe_skill_name(name: &str) -> bool {
        !name.is_empty()
            && !name.contains('/')
            && !name.contains('\\')
            && !name.contains("..")
            && !name.starts_with('.')
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }

    /// Check if a skill exists at the expected location.
    /// Returns false for invalid skill names (path traversal attempts).
    fn skill_exists(project_root: &Path, skill_name: &str, fs: &dyn FileSystem) -> bool {
        if !Self::is_safe_skill_name(skill_name) {
            return false;
        }
        let skill_path = project_root
            .join(".claude")
            .join("skills")
            .join(skill_name)
            .join("SKILL.md");
        fs.exists(&skill_path)
    }

    /// Helper to check if a tool name is valid (either known or properly formatted MCP tool).
    fn is_valid_tool_name(tool: &str) -> bool {
        is_valid_mcp_tool_format(tool, KNOWN_AGENT_TOOLS)
    }
}

impl Validator for AgentValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // pi subagent definitions use pi's own fields, tool names and model ids,
        // and a Claude-rule autofix would break them (CC-AG-003 rewrote a pi
        // `model` to `sonnet`).
        if is_pi_agent_root(path, config.fs().as_ref()) {
            return diagnostics;
        }

        // Other tools' agent directories that path detection hands to this
        // validator because the parent is named `agents`: OpenCode
        // (`.opencode/agents/`, `~/.config/opencode/agents/`) and GitHub
        // Copilot (`.github/agents/`). Claude Code reads neither.
        if is_other_vendor_agents_dir(path) {
            return diagnostics;
        }

        let local_agent = path
            .ancestors()
            .skip(1)
            .find_map(|ancestor| {
                if ancestor.file_name().is_some_and(|name| name == "agents") {
                    Some(
                        ancestor
                            .parent()
                            .and_then(Path::file_name)
                            .is_some_and(|name| name == ".claude" || name == "claude-code"),
                    )
                } else if config.fs().exists(&ancestor.join(".claude-plugin")) {
                    Some(false)
                } else {
                    None
                }
            })
            .unwrap_or(true);
        // Check if content has frontmatter
        if !(if local_agent {
            content.starts_with("---\n") || content.starts_with("---\r\n")
        } else {
            content.trim_start().starts_with("---")
        }) {
            // "No `name`: Claude Code treats the file as documentation kept
            // beside your agents" and "An opening `---` that isn't the file's
            // first line: Claude Code reads the file as having no frontmatter
            // and treats it as documentation." A README is a normal thing to
            // keep there, so this is information, not an error.
            if local_agent && config.is_rule_enabled("CC-AG-007") {
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-AG-007",
                        t!("rules.cc_ag_007.message"),
                    )
                    .with_suggestion(t!("rules.cc_ag_007.suggestion")),
                );
            }
            return diagnostics;
        }

        // Parse frontmatter directly to preserve serde_yaml error location
        let parts = split_frontmatter(content);
        let schema: AgentSchema = match serde_yaml::from_str(&parts.frontmatter) {
            Ok(s) => s,
            Err(e) => {
                if local_agent && config.is_rule_enabled("CC-AG-007") {
                    // serde_yaml lines are relative to the frontmatter string;
                    // add 1 to account for the `---` delimiter line.
                    let (line, column) = e
                        .location()
                        .map(|loc| (loc.line() + 1, loc.column()))
                        .unwrap_or((1, 0));
                    let raw_error = e.to_string();
                    let friendly_error = humanize_yaml_error(&raw_error);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            column,
                            "CC-AG-007",
                            t!("rules.cc_ag_007.parse_error", error = friendly_error),
                        )
                        .with_suggestion(t!("rules.cc_ag_007.parse_error_suggestion")),
                    );
                }
                return diagnostics;
            }
        };

        // CC-AG-001: Missing name field
        if local_agent
            && config.is_rule_enabled("CC-AG-001")
            && schema.name.as_deref().unwrap_or("").trim().is_empty()
        {
            let mut diagnostic = Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CC-AG-001",
                t!("rules.cc_ag_001.message"),
            )
            .with_suggestion(t!("rules.cc_ag_001.suggestion"));

            // Derive name from filename (e.g., "reviewer.md" -> "reviewer")
            // Sanitize via kebab-case conversion to prevent YAML injection from special chars
            let derived_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(crate::rules::skill::convert_to_kebab_case)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "agent".to_string());
            let parts_fm = split_frontmatter(content);
            if parts_fm.has_frontmatter && parts_fm.has_closing {
                // Insert after opening --- and its line ending
                let insert_pos =
                    crate::rules::frontmatter_content_offset(content, parts_fm.frontmatter_start);
                diagnostic = diagnostic.with_fix(Fix::insert(
                    insert_pos,
                    format!("name: {}\n", derived_name),
                    format!("Insert name: {}", derived_name),
                    false,
                ));
            }

            diagnostics.push(diagnostic);
        }

        // CC-AG-020: Claude Code reserves ':' in agent names for plugin
        // namespace qualification, so local agent definitions cannot use it.
        if config.is_rule_enabled("CC-AG-020")
            && let Some(name) = schema.name.as_deref()
            && local_agent
            && (name.contains(':') || name.starts_with('-') || name.chars().count() > 256)
        {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CC-AG-020",
                    t!("rules.cc_ag_020.message", name = name),
                )
                .with_suggestion(t!("rules.cc_ag_020.suggestion")),
            );
        }

        // CC-AG-002: Missing description field
        if config.is_rule_enabled("CC-AG-002")
            && schema
                .description
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            let mut diagnostic = Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CC-AG-002",
                t!("rules.cc_ag_002.message"),
            )
            .with_suggestion(t!("rules.cc_ag_002.suggestion"));

            let parts_fm = split_frontmatter(content);
            if parts_fm.has_frontmatter && parts_fm.has_closing {
                let insert_pos =
                    crate::rules::frontmatter_content_offset(content, parts_fm.frontmatter_start);
                diagnostic = diagnostic.with_fix(Fix::insert(
                    insert_pos,
                    "description: TODO - add agent description\n".to_string(),
                    "Insert description placeholder",
                    false,
                ));
            }

            diagnostics.push(diagnostic);
        }

        // CC-AG-003: Invalid model value
        if config.is_rule_enabled("CC-AG-003") {
            if let Some(model) = &schema.model {
                if !is_valid_model(model.as_str()) {
                    let valid_display = format!("{}, or claude-*", VALID_MODELS.join(", "));
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-AG-003",
                        t!(
                            "rules.cc_ag_003.message",
                            model = model.as_str(),
                            valid = valid_display
                        ),
                    )
                    .with_suggestion(t!("rules.cc_ag_003.suggestion", valid = valid_display));

                    // Unsafe auto-fix: default invalid model to sonnet.
                    if let Some((start, end)) =
                        frontmatter_value_byte_range_from_parts(&parts, "model")
                    {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            "sonnet",
                            "Replace invalid model with 'sonnet'",
                            false,
                        ));
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }

        // CC-AG-004: Invalid permission mode
        if config.is_rule_enabled("CC-AG-004") {
            if let Some(mode) = &schema.permission_mode {
                if !VALID_PERMISSION_MODES.contains(&mode.as_str()) {
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-AG-004",
                        t!(
                            "rules.cc_ag_004.message",
                            mode = mode.as_str(),
                            valid = VALID_PERMISSION_MODES.join(", ")
                        ),
                    )
                    .with_suggestion(t!(
                        "rules.cc_ag_004.suggestion",
                        valid = VALID_PERMISSION_MODES.join(", ")
                    ));

                    // Unsafe auto-fix: normalize invalid permission mode to default.
                    if let Some((start, end)) =
                        frontmatter_value_byte_range_from_parts(&parts, "permissionMode")
                    {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            "default",
                            "Replace invalid permissionMode with 'default'",
                            false,
                        ));
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }

        // CC-AG-005: Referenced skill not found, or one Claude Code doesn't
        // preload: "up to the first 32 distinct names in the list" and "You
        // can't preload skills that set `disable-model-invocation: true`".
        if config.is_rule_enabled("CC-AG-005") {
            if let Some(skills) = &schema.skills {
                let fs = config.fs();
                let (skills_line, skills_col) = frontmatter_key_pos(content, &parts, "skills");
                let mut distinct: Vec<&str> = Vec::new();
                for skill in skills {
                    if !distinct.contains(&skill.as_str()) {
                        distinct.push(skill.as_str());
                    }
                }
                if distinct.len() > 32 {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            skills_line,
                            skills_col,
                            "CC-AG-005",
                            t!(
                                "rules.cc_ag_005.too_many",
                                count = distinct.len(),
                                skipped = distinct[32..].join(", ")
                            ),
                        )
                        .with_suggestion(t!("rules.cc_ag_005.too_many_suggestion")),
                    );
                }
                if let Some(project_root) = Self::find_project_root(path, fs.as_ref()) {
                    for skill_name in skills {
                        if !Self::is_safe_skill_name(skill_name) {
                            continue;
                        }
                        let skill_md = project_root
                            .join(".claude")
                            .join("skills")
                            .join(skill_name)
                            .join("SKILL.md");
                        let not_preloadable = fs.read_to_string(&skill_md).is_ok_and(|text| {
                            let fm = split_frontmatter(&text);
                            fm.has_frontmatter
                                && quick_frontmatter_scalar(
                                    &fm.frontmatter,
                                    "disable-model-invocation",
                                )
                                .is_some_and(|v| {
                                    matches!(
                                        v.to_ascii_lowercase().as_str(),
                                        "true" | "yes" | "on" | "1"
                                    )
                                })
                        });
                        if not_preloadable {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    skills_line,
                                    skills_col,
                                    "CC-AG-005",
                                    t!(
                                        "rules.cc_ag_005.not_preloadable",
                                        skill = skill_name.as_str()
                                    ),
                                )
                                .with_suggestion(t!("rules.cc_ag_005.not_preloadable_suggestion")),
                            );
                        }
                    }
                }
                if let Some(project_root) = Self::find_project_root(path, fs.as_ref()) {
                    for skill_name in skills {
                        #[cfg(feature = "filesystem")]
                        let user_skill = dirs::home_dir().map(|home| {
                            home.join(".claude/skills")
                                .join(skill_name)
                                .join("SKILL.md")
                        });
                        #[cfg(not(feature = "filesystem"))]
                        let user_skill: Option<std::path::PathBuf> = None;
                        if !skill_name.contains(':')
                            && !Self::skill_exists(project_root, skill_name, fs.as_ref())
                            && !user_skill.as_ref().is_some_and(|path| {
                                Self::is_safe_skill_name(skill_name) && fs.exists(path)
                            })
                        {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-AG-005",
                                    t!("rules.cc_ag_005.message", skill = skill_name.as_str()),
                                )
                                .with_suggestion(t!(
                                    "rules.cc_ag_005.suggestion",
                                    skill = skill_name.as_str()
                                )),
                            );
                        }
                    }
                }
            }
        }

        // CC-AG-006: Tool/disallowed conflict
        if config.is_rule_enabled("CC-AG-006") {
            if let (Some(tools), Some(disallowed)) = (&schema.tools, &schema.disallowed_tools) {
                let tools_set: HashSet<&str> = tools.iter().map(|s| s.as_str()).collect();
                let disallowed_set: HashSet<&str> = disallowed.iter().map(|s| s.as_str()).collect();

                let conflicts: Vec<&str> =
                    tools_set.intersection(&disallowed_set).copied().collect();

                if !conflicts.is_empty() {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-AG-006",
                            t!("rules.cc_ag_006.message", conflicts = conflicts.join(", ")),
                        )
                        .with_suggestion(t!("rules.cc_ag_006.suggestion")),
                    );
                }
            }
        }

        // CC-AG-008: Invalid memory scope
        if config.is_rule_enabled("CC-AG-008") {
            if let Some(memory) = &schema.memory {
                if !VALID_MEMORY_SCOPES.contains(&memory.as_str()) {
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-AG-008",
                        t!("rules.cc_ag_008.message", scope = memory.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_ag_008.suggestion"));

                    // Unsafe auto-fix: replace with closest valid memory scope
                    if let Some(closest) =
                        super::find_closest_value(memory.as_str(), VALID_MEMORY_SCOPES)
                    {
                        if let Some((start, end)) =
                            frontmatter_value_byte_range_from_parts(&parts, "memory")
                        {
                            diagnostic = diagnostic.with_fix(Fix::replace(
                                start,
                                end,
                                closest,
                                t!("rules.cc_ag_008.fix", fixed = closest),
                                false,
                            ));
                        }
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }

        // CC-AG-009: Invalid tool name in tools list
        // CC-AG-010: Invalid tool name in disallowedTools list
        // Compute known tools list once via OnceLock (shared across both rules)
        static KNOWN_TOOLS_LIST: OnceLock<String> = OnceLock::new();
        let known_tools_str = KNOWN_TOOLS_LIST.get_or_init(|| KNOWN_AGENT_TOOLS.join(", "));

        if config.is_rule_enabled("CC-AG-009") {
            if let Some(tools) = &schema.tools {
                for tool in tools {
                    if !Self::is_valid_tool_name(tool) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-AG-009",
                                t!(
                                    "rules.cc_ag_009.message",
                                    tool = tool.as_str(),
                                    known = known_tools_str
                                ),
                            )
                            .with_suggestion(t!("rules.cc_ag_009.suggestion")),
                        );
                    }
                }
            }
        }

        if config.is_rule_enabled("CC-AG-010") {
            if let Some(disallowed) = &schema.disallowed_tools {
                for tool in disallowed {
                    // The deny list additionally accepts this exact pattern
                    // to remove every MCP tool. Allow lists still require a
                    // specific server, as do all other server patterns.
                    if tool != "mcp__*" && !Self::is_valid_tool_name(tool) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-AG-010",
                                t!(
                                    "rules.cc_ag_010.message",
                                    tool = tool.as_str(),
                                    known = known_tools_str
                                ),
                            )
                            .with_suggestion(t!("rules.cc_ag_010.suggestion")),
                        );
                    }
                }
            }
        }

        // CC-HK-014: `once` is honored only in skill hooks. Agent hooks
        // otherwise remain valid; warn without changing their content.
        // Borrow the hooks check without owning its metadata ID; each rule
        // has one registry owner, as with hooks checked by SkillValidator.
        if config.is_rule_enabled("CC-HK-014") {
            if let Some(hooks) = schema.hooks.as_ref().and_then(|v| v.as_object()) {
                for (event, matchers) in hooks {
                    for (matcher_idx, matcher) in
                        matchers.as_array().into_iter().flatten().enumerate()
                    {
                        for (hook_idx, hook) in matcher
                            .get("hooks")
                            .and_then(|v| v.as_array())
                            .into_iter()
                            .flatten()
                            .enumerate()
                        {
                            if hook.get("once").is_some() {
                                let location =
                                    format!("hooks.{event}[{matcher_idx}].hooks[{hook_idx}]");
                                diagnostics.push(
                                    Diagnostic::warning(
                                        path.to_path_buf(),
                                        1,
                                        0,
                                        "CC-HK-014",
                                        t!("rules.cc_hk_014.message", location = location.as_str()),
                                    )
                                    .with_suggestion(t!("rules.cc_hk_014.suggestion")),
                                );
                            }
                        }
                    }
                }
            }
        }

        // CC-AG-011: Hooks in agent frontmatter validation
        if config.is_rule_enabled("CC-AG-011") {
            if let Some(hooks_value) = &schema.hooks {
                // Compute valid events list once
                static VALID_EVENTS_LIST: OnceLock<String> = OnceLock::new();
                let valid_events_str =
                    VALID_EVENTS_LIST.get_or_init(|| HooksSchema::VALID_EVENTS.join(", "));

                // The hooks field should be an object mapping event names to arrays
                if let Some(hooks_obj) = hooks_value.as_object() {
                    for (event_name, event_value) in hooks_obj {
                        // Validate event name
                        if !HooksSchema::VALID_EVENTS.contains(&event_name.as_str()) {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-AG-011",
                                    t!(
                                        "rules.cc_ag_011.message",
                                        error = format!(
                                            "unknown event '{}', valid events: {}",
                                            event_name, valid_events_str
                                        )
                                    ),
                                )
                                .with_suggestion(t!("rules.cc_ag_011.suggestion")),
                            );
                            continue;
                        }

                        // Validate event value is an array of hook matchers
                        if let Some(matchers) = event_value.as_array() {
                            for (i, matcher) in matchers.iter().enumerate() {
                                if let Some(matcher_obj) = matcher.as_object() {
                                    // Each matcher must have a 'hooks' array
                                    match matcher_obj.get("hooks") {
                                        Some(hooks_arr) => {
                                            if let Some(hooks) = hooks_arr.as_array() {
                                                for (j, hook) in hooks.iter().enumerate() {
                                                    if let Some(hook_obj) = hook.as_object() {
                                                        // Each hook must have a valid 'type'
                                                        match hook_obj
                                                            .get("type")
                                                            .and_then(|t| t.as_str())
                                                        {
                                                            Some(
                                                                "command" | "http" | "mcp_tool"
                                                                | "prompt" | "agent",
                                                            ) => {}
                                                            Some(invalid_type) => {
                                                                diagnostics.push(
                                                                    Diagnostic::error(
                                                                        path.to_path_buf(),
                                                                        1,
                                                                        0,
                                                                        "CC-AG-011",
                                                                        t!(
                                                                            "rules.cc_ag_011.message",
                                                                            error = format!(
                                                                                "hook type '{}' in hooks.{}[{}].hooks[{}] is invalid, must be 'command', 'http', 'mcp_tool', 'prompt', or 'agent'",
                                                                                invalid_type, event_name, i, j
                                                                            )
                                                                        ),
                                                                    )
                                                                    .with_suggestion(t!("rules.cc_ag_011.suggestion")),
                                                                );
                                                            }
                                                            None => {
                                                                diagnostics.push(
                                                                    Diagnostic::error(
                                                                        path.to_path_buf(),
                                                                        1,
                                                                        0,
                                                                        "CC-AG-011",
                                                                        t!(
                                                                            "rules.cc_ag_011.message",
                                                                            error = format!(
                                                                                "hook in hooks.{}[{}].hooks[{}] is missing required 'type' field",
                                                                                event_name, i, j
                                                                            )
                                                                        ),
                                                                    )
                                                                    .with_suggestion(t!("rules.cc_ag_011.suggestion")),
                                                                );
                                                            }
                                                        }
                                                    } else {
                                                        diagnostics.push(
                                                            Diagnostic::error(
                                                                path.to_path_buf(),
                                                                1,
                                                                0,
                                                                "CC-AG-011",
                                                                t!(
                                                                    "rules.cc_ag_011.message",
                                                                    error = format!(
                                                                        "hook in hooks.{}[{}].hooks[{}] must be an object",
                                                                        event_name, i, j
                                                                    )
                                                                ),
                                                            )
                                                            .with_suggestion(t!("rules.cc_ag_011.suggestion")),
                                                        );
                                                    }
                                                }
                                            } else {
                                                diagnostics.push(
                                                    Diagnostic::error(
                                                        path.to_path_buf(),
                                                        1,
                                                        0,
                                                        "CC-AG-011",
                                                        t!(
                                                            "rules.cc_ag_011.message",
                                                            error = format!(
                                                                "'hooks' field in hooks.{}[{}] must be an array",
                                                                event_name, i
                                                            )
                                                        ),
                                                    )
                                                    .with_suggestion(t!("rules.cc_ag_011.suggestion")),
                                                );
                                            }
                                        }
                                        None => {
                                            diagnostics.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    1,
                                                    0,
                                                    "CC-AG-011",
                                                    t!(
                                                        "rules.cc_ag_011.message",
                                                        error = format!(
                                                            "matcher in hooks.{}[{}] is missing required 'hooks' array",
                                                            event_name, i
                                                        )
                                                    ),
                                                )
                                                .with_suggestion(t!("rules.cc_ag_011.suggestion")),
                                            );
                                        }
                                    }
                                } else {
                                    diagnostics.push(
                                        Diagnostic::error(
                                            path.to_path_buf(),
                                            1,
                                            0,
                                            "CC-AG-011",
                                            t!(
                                                "rules.cc_ag_011.message",
                                                error = format!(
                                                    "matcher in hooks.{}[{}] must be an object",
                                                    event_name, i
                                                )
                                            ),
                                        )
                                        .with_suggestion(t!("rules.cc_ag_011.suggestion")),
                                    );
                                }
                            }
                        } else {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-AG-011",
                                    t!(
                                        "rules.cc_ag_011.message",
                                        error = format!(
                                            "event '{}' must map to an array of hook matchers",
                                            event_name
                                        )
                                    ),
                                )
                                .with_suggestion(t!("rules.cc_ag_011.suggestion")),
                            );
                        }
                    }
                } else {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-AG-011",
                            t!(
                                "rules.cc_ag_011.message",
                                error =
                                    "hooks must be an object mapping event names to hook arrays"
                            ),
                        )
                        .with_suggestion(t!("rules.cc_ag_011.suggestion")),
                    );
                }
            }
        }

        // CC-AG-012: bypassPermissions warning
        if config.is_rule_enabled("CC-AG-012") {
            if let Some(mode) = &schema.permission_mode {
                if mode == "bypassPermissions" {
                    let diagnostic = Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-AG-012",
                        t!("rules.cc_ag_012.message"),
                    )
                    .with_suggestion(t!("rules.cc_ag_012.suggestion"));

                    diagnostics.push(diagnostic);
                }
            }
        }

        // CC-AG-014: Invalid effort value
        if config.is_rule_enabled("CC-AG-014") {
            if let Some(effort) = &schema.effort {
                if !VALID_EFFORT_LEVELS.contains(&effort.as_str()) {
                    let valid_display = VALID_EFFORT_LEVELS.join(", ");
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-AG-014",
                        format!(
                            "Invalid effort value '{}', must be one of: {}",
                            effort, valid_display
                        ),
                    )
                    .with_suggestion(format!("Use a valid effort value: {}", valid_display));

                    // Unsafe auto-fix: replace with closest valid effort value
                    if let Some(closest) =
                        super::find_closest_value(effort.as_str(), VALID_EFFORT_LEVELS)
                    {
                        if let Some((start, end)) =
                            frontmatter_value_byte_range_from_parts(&parts, "effort")
                        {
                            diagnostic = diagnostic.with_fix(Fix::replace(
                                start,
                                end,
                                closest,
                                format!("Replace invalid effort with '{}'", closest),
                                false,
                            ));
                        }
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }

        // CC-AG-015: Invalid isolation value
        if config.is_rule_enabled("CC-AG-015") {
            if let Some(isolation) = &schema.isolation {
                if !VALID_ISOLATION_VALUES.contains(&isolation.as_str()) {
                    let valid_display = VALID_ISOLATION_VALUES.join(", ");
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-AG-015",
                        format!(
                            "Invalid isolation value '{}', must be one of: {}",
                            isolation, valid_display
                        ),
                    )
                    .with_suggestion(format!("Use a valid isolation value: {}", valid_display));

                    // Unsafe auto-fix: replace with 'worktree' (the only valid value)
                    if let Some((start, end)) =
                        frontmatter_value_byte_range_from_parts(&parts, "isolation")
                    {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            "worktree",
                            "Replace invalid isolation with 'worktree'",
                            false,
                        ));
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }

        // CC-AG-016: Invalid background type
        // Since background is Option<bool> in the schema, serde_yaml will reject
        // non-boolean values at parse time (CC-AG-007). No additional runtime
        // validation is needed here - the type system enforces correctness.
        // See tests: test_cc_ag_016_serde_catches_non_bool

        // CC-AG-017: Invalid maxTurns value (zero)
        // serde_yaml handles non-integer values at parse time (CC-AG-007).
        // We only need to check for zero since u32 already prevents negatives.
        if config.is_rule_enabled("CC-AG-017") {
            if let Some(max_turns) = schema.max_turns {
                if max_turns == 0 {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-AG-017",
                            "Invalid maxTurns value '0', must be a positive integer (> 0)",
                        )
                        .with_suggestion(
                            "Set maxTurns to a positive integer, e.g. maxTurns: 10".to_string(),
                        ),
                    );
                }
            }
        }

        // CC-AG-019: Unknown agent frontmatter field
        // Unknown keys are captured by serde(flatten) into schema.extra during
        // the initial parse, so no re-parsing is needed.
        if config.is_rule_enabled("CC-AG-019") {
            let mut keys: Vec<&String> = schema.extra.keys().collect();
            keys.sort();
            for key_str in keys {
                let (line, col) = frontmatter_key_pos(content, &parts, key_str);
                let suggestion = if key_str == "cacheTtl" {
                    t!("rules.cc_ag_019.cache_ttl_suggestion").to_string()
                } else {
                    t!(
                        "rules.cc_ag_019.unknown_suggestion",
                        field = key_str.as_str(),
                        known = KNOWN_AGENT_FIELDS.join(", ")
                    )
                    .to_string()
                };
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        col,
                        "CC-AG-019",
                        t!("rules.cc_ag_019.unknown", field = key_str.as_str()),
                    )
                    .with_suggestion(suggestion),
                );
            }
            // "set `autoCompactWindow` in its frontmatter to a token count
            // from `100000` to `1000000`" and "in frontmatter Claude Code
            // reads `200k` as 200, which is outside the allowed range. The
            // subagent still loads and compacts as if the field were absent."
            if let Some(value) = &schema.auto_compact_window
                && let Some(problem) = auto_compact_window_problem(value)
            {
                let (line, col) = frontmatter_key_pos(content, &parts, "autoCompactWindow");
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        col,
                        "CC-AG-019",
                        t!(
                            "rules.cc_ag_019.auto_compact_window",
                            value = problem.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.cc_ag_019.auto_compact_window_suggestion")),
                );
            }
        }

        let plugin_root = plugin_agent_root(path, config.fs().as_ref());

        // CC-PL-014: "Ignored fields: `permissionMode`, `hooks`,
        // `mcpServers`, and `initialPrompt`" in plugin agents.
        if let Some(root) = plugin_root
            && config.is_rule_enabled("CC-PL-014")
        {
            let present = [
                ("permissionMode", schema.permission_mode.is_some()),
                ("hooks", schema.hooks.is_some()),
                ("mcpServers", schema.mcp_servers.is_some()),
                ("initialPrompt", schema.initial_prompt.is_some()),
            ];
            let plugin = root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            for (field, _) in present.iter().filter(|(_, set)| *set) {
                let (line, col) = frontmatter_key_pos(content, &parts, field);
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        col,
                        "CC-PL-014",
                        t!("rules.cc_pl_014.message", field = *field, plugin = plugin),
                    )
                    .with_suggestion(
                        if *field == "permissionMode" || *field == "initialPrompt" {
                            t!("rules.cc_pl_014.suggestion_remove", field = *field)
                        } else {
                            t!("rules.cc_pl_014.suggestion_plugin_level", field = *field)
                        },
                    ),
                );
            }
        }

        // CC-PL-020 (owned by SkillValidator): a plugin agent body that writes
        // `$CLAUDE_PLUGIN_ROOT` without braces gets no substitution, and the
        // variable isn't in the Bash tool's environment.
        if plugin_root.is_some()
            && config.is_rule_enabled("CC-PL-020")
            && let Some((pos, spelling)) = crate::rules::skill::unbraced_plugin_var(
                &content[parts.body_start.min(content.len())..],
            )
        {
            let offset = parts.body_start.min(content.len()) + pos;
            let line = content[..offset].bytes().filter(|&b| b == b'\n').count() + 1;
            let col = offset - content[..offset].rfind('\n').map_or(0, |i| i + 1) + 1;
            let braced = format!("${{{}}}", &spelling[1..]);
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "CC-PL-020",
                    t!("rules.cc_pl_020.message", spelling = spelling),
                )
                .with_suggestion(t!("rules.cc_pl_020.suggestion", braced = braced.as_str())),
            );
        }

        // CC-HK-0xx handler checks on agent frontmatter hooks. Plugin agents
        // ignore `hooks` (CC-PL-014 above).
        if plugin_root.is_none()
            && let Some(events) = schema.hooks.as_ref().and_then(|v| v.as_object())
        {
            let (line, col) = frontmatter_key_pos(content, &parts, "hooks");
            for finding in crate::rules::hooks::check_hooks(
                events,
                crate::rules::hooks::HookOrigin::Frontmatter,
                None,
                config,
            ) {
                diagnostics.push(frontmatter_hook_diagnostic(path, line, col, finding));
            }
        }

        // CC-AG-022: "Limit tool access: grant only necessary permissions" and
        // `tools` "Inherits every tool available to subagents if omitted."
        if config.is_rule_enabled("CC-AG-022")
            && schema.tools.is_none()
            && schema.disallowed_tools.is_none()
            && schema.name.as_deref().is_some_and(|n| !n.trim().is_empty())
        {
            let (line, col) = frontmatter_key_pos(content, &parts, "name");
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    line,
                    col,
                    "CC-AG-022",
                    t!("rules.cc_ag_022.message"),
                )
                .with_suggestion(t!("rules.cc_ag_022.suggestion")),
            );
        }

        // CC-AG-024: a `disallowedTools` entry with a specifier "still removes
        // the whole tool from the subagent, not only the matching commands",
        // and an `Agent(type, ...)` list in a subagent's `tools` is ignored
        // unless the agent runs as the main thread with `claude --agent`.
        if config.is_rule_enabled("CC-AG-024") {
            for entry in schema.disallowed_tools.iter().flatten() {
                if let Some((tool, _)) = entry.split_once('(')
                    && entry.ends_with(')')
                    && !tool.is_empty()
                    && tool.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    let (line, col) = frontmatter_key_pos(content, &parts, "disallowedTools");
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            col,
                            "CC-AG-024",
                            t!(
                                "rules.cc_ag_024.disallowed_specifier",
                                entry = entry.as_str(),
                                tool = tool
                            ),
                        )
                        .with_suggestion(t!(
                            "rules.cc_ag_024.disallowed_specifier_suggestion",
                            entry = entry.as_str()
                        )),
                    );
                }
            }
            for entry in schema.tools.iter().flatten() {
                if let Some(inner) = entry
                    .strip_prefix("Agent(")
                    .or_else(|| entry.strip_prefix("Task("))
                    && inner.ends_with(')')
                    && inner.len() > 1
                {
                    let (line, col) = frontmatter_key_pos(content, &parts, "tools");
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            line,
                            col,
                            "CC-AG-024",
                            t!("rules.cc_ag_024.agent_type_list", entry = entry.as_str()),
                        )
                        .with_suggestion(t!("rules.cc_ag_024.agent_type_list_suggestion")),
                    );
                }
            }
        }

        // CC-AG-021 and CC-AG-023 look at the whole `.claude/agents/` tree.
        if local_agent
            && (config.is_rule_enabled("CC-AG-021") || config.is_rule_enabled("CC-AG-023"))
            && let Some(tree) = local_agents_tree(path)
        {
            check_agents_tree(
                path,
                content,
                &parts,
                &schema,
                tree,
                config,
                &mut diagnostics,
            );
        }

        diagnostics
    }
}

/// Most files read from one agents tree for CC-AG-021 and CC-AG-023.
const MAX_TREE_AGENTS: usize = 2000;

/// The combined-description warning threshold Claude Code shows at startup:
/// "When the combined descriptions of your subagents, except the built-in
/// ones, exceed 15,000 tokens".
const AGENT_DESCRIPTION_TOKEN_BUDGET: usize = 15_000;

/// Directories named `agents` that belong to other tools.
fn is_other_vendor_agents_dir(path: &Path) -> bool {
    let Some(agents_dir) = nearest_agents_dir(path) else {
        return false;
    };
    let parent = agents_dir.parent();
    let parent_name = parent.and_then(Path::file_name).and_then(|n| n.to_str());
    match parent_name {
        Some(".opencode") | Some(".github") => true,
        Some("opencode") => parent
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .is_some_and(|n| n == ".config"),
        _ => false,
    }
}

fn nearest_agents_dir(path: &Path) -> Option<&Path> {
    path.ancestors()
        .skip(1)
        .find(|dir| dir.file_name().is_some_and(|name| name == "agents"))
}

/// The plugin root of a plugin agent (`<root>/agents/**.md` with
/// `<root>/.claude-plugin/`).
fn plugin_agent_root<'a>(path: &'a Path, fs: &dyn FileSystem) -> Option<&'a Path> {
    let root = nearest_agents_dir(path)?.parent()?;
    fs.exists(&root.join(".claude-plugin")).then_some(root)
}

/// The `.claude/agents` directory of a local agent.
fn local_agents_tree(path: &Path) -> Option<&Path> {
    let agents = nearest_agents_dir(path)?;
    agents
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|n| n == ".claude")
        .then_some(agents)
}

/// (line, column) of a top-level frontmatter key, or of the opening `---`.
fn frontmatter_key_pos(content: &str, parts: &FrontmatterParts, key: &str) -> (usize, usize) {
    let mut offset = parts.frontmatter_start;
    for line in parts.frontmatter.split_inclusive('\n') {
        if let Some(rest) = line.strip_prefix(key)
            && rest.trim_start().starts_with(':')
        {
            let line_no = content[..offset.min(content.len())]
                .bytes()
                .filter(|&b| b == b'\n')
                .count()
                + 1;
            return (line_no, 1);
        }
        offset += line.len();
    }
    (1, 1)
}

/// Why an `autoCompactWindow` value is ignored, if it is.
fn auto_compact_window_problem(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::Number(n) => match n.as_u64() {
            Some(v) if (100_000..=1_000_000).contains(&v) => None,
            _ => Some(n.to_string()),
        },
        serde_yaml::Value::String(s) => Some(format!("'{s}'")),
        serde_yaml::Value::Null => None,
        other => Some(format!("{other:?}")),
    }
}

/// A CC-HK finding on a frontmatter hook, reported at the `hooks` key.
fn frontmatter_hook_diagnostic(
    path: &Path,
    line: usize,
    col: usize,
    finding: crate::rules::hooks::HookFinding,
) -> Diagnostic {
    let make = match finding.level {
        crate::diagnostics::DiagnosticLevel::Error => Diagnostic::error,
        crate::diagnostics::DiagnosticLevel::Warning => Diagnostic::warning,
        crate::diagnostics::DiagnosticLevel::Info => Diagnostic::info,
    };
    make(path.to_path_buf(), line, col, finding.rule, finding.message)
        .with_suggestion(finding.suggestion)
}

/// The value of a top-level `key:` line in a frontmatter block, unquoted.
fn quick_frontmatter_scalar(frontmatter: &str, key: &str) -> Option<String> {
    frontmatter.lines().find_map(|line| {
        let rest = line.strip_prefix(key)?.trim_start().strip_prefix(':')?;
        let value = rest.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value);
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// Every `.md` file under an agents tree, sorted.
fn agent_tree_files(tree: &Path, fs: &dyn FileSystem) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![(tree.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = fs.read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            if entry.metadata.is_symlink {
                continue;
            }
            if entry.metadata.is_dir && depth < MAX_TRAVERSAL_DEPTH {
                stack.push((entry.path.clone(), depth + 1));
            } else if entry.metadata.is_file
                && entry.path.extension().is_some_and(|e| e == "md")
                && files.len() < MAX_TREE_AGENTS
            {
                files.push(entry.path.clone());
            }
        }
    }
    files.sort();
    files
}

/// CC-AG-021 (combined description budget) and CC-AG-023 (duplicate names)
/// over the `.claude/agents` tree that holds `path`.
fn check_agents_tree(
    path: &Path,
    content: &str,
    parts: &FrontmatterParts,
    schema: &AgentSchema,
    tree: &Path,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fs = config.fs();
    let files = agent_tree_files(tree, fs.as_ref());
    // Read each other agent's frontmatter once. Files without frontmatter on
    // the first line are documentation, not agents.
    let agents: Vec<(std::path::PathBuf, String)> = files
        .iter()
        .filter_map(|file| {
            let text = if file == path {
                content.to_string()
            } else {
                fs.read_to_string(file).ok()?
            };
            if !(text.starts_with("---\n") || text.starts_with("---\r\n")) {
                return None;
            }
            Some((file.clone(), text))
        })
        .collect();
    let rel = |p: &Path| {
        p.strip_prefix(tree)
            .unwrap_or(p)
            .display()
            .to_string()
            .replace('\\', "/")
    };

    // CC-AG-023: "if two files under the same `.claude/agents/` directory,
    // including its subfolders, declare the same name, Claude Code loads
    // only one of them, chosen by filesystem read order".
    if config.is_rule_enabled("CC-AG-023")
        && let Some(name) = schema
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
    {
        let others: Vec<String> = agents
            .iter()
            .filter(|(file, _)| file != path)
            .filter(|(_, text)| {
                quick_frontmatter_scalar(&split_frontmatter(text).frontmatter, "name").as_deref()
                    == Some(name)
            })
            .map(|(file, _)| rel(file))
            .collect();
        if !others.is_empty() {
            let (line, col) = frontmatter_key_pos(content, parts, "name");
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "CC-AG-023",
                    t!(
                        "rules.cc_ag_023.message",
                        name = name,
                        others = others.join(", ")
                    ),
                )
                .with_suggestion(t!("rules.cc_ag_023.suggestion")),
            );
        }
    }

    // CC-AG-021: the first agent file in the tree reports the total, so one
    // project gets one diagnostic. "Each agent counts its name plus its
    // `description` frontmatter." Tokens are estimated as characters / 4,
    // agnix's usual estimate; Claude Code's own count can differ.
    if config.is_rule_enabled("CC-AG-021") && agents.first().is_some_and(|(f, _)| f == path) {
        let mut chars = 0usize;
        for (_, text) in &agents {
            let fm = split_frontmatter(text);
            let Ok(agent) = serde_yaml::from_str::<AgentSchema>(&fm.frontmatter) else {
                continue;
            };
            let Some(name) = agent.name.as_deref().filter(|n| !n.trim().is_empty()) else {
                continue;
            };
            chars +=
                name.chars().count() + agent.description.as_deref().unwrap_or("").chars().count();
        }
        let tokens = chars / 4;
        if tokens > AGENT_DESCRIPTION_TOKEN_BUDGET {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    1,
                    "CC-AG-021",
                    t!(
                        "rules.cc_ag_021.message",
                        tokens = tokens,
                        count = agents.len(),
                        budget = AGENT_DESCRIPTION_TOKEN_BUDGET
                    ),
                )
                .with_suggestion(t!("rules.cc_ag_021.suggestion")),
            );
        }
    }
}

/// pi settings keys that Claude Code settings never use (pi `docs/settings.md`).
const PI_SETTINGS_KEYS: &[&str] = &[
    "defaultProvider",
    "defaultModel",
    "defaultThinkingLevel",
    "modelThinkingLevels",
    "enabledModels",
    "packages",
];

/// Package names whose import marks an extension directory as pi's.
const PI_PACKAGES: &[&str] = &[
    "@earendil-works/pi-coding-agent",
    "@mariozechner/pi-coding-agent",
];

/// Returns true if `path` is an agent in a pi agent directory: the directory
/// holding `agents/` is not a Claude Code root (`.claude`, `claude-code`, or one
/// with `.claude-plugin/`), and it has a pi package manifest, pi settings, or an
/// extension that imports pi. pi has no marker file of its own; it finds agents
/// by location (`~/.pi/agent/agents`, `.pi/agents`, package and settings paths),
/// and these are the files that define those locations.
fn is_pi_agent_root(path: &Path, fs: &dyn crate::fs::FileSystem) -> bool {
    let Some(agents_dir) = path
        .ancestors()
        .skip(1)
        .find(|dir| dir.file_name().is_some_and(|name| name == "agents"))
    else {
        return false;
    };
    let Some(root) = agents_dir.parent() else {
        return false;
    };
    let root_name = root.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if root_name == ".claude"
        || root_name == "claude-code"
        || fs.exists(&root.join(".claude-plugin"))
    {
        return false;
    }

    let json_object = |name: &str| {
        fs.read_to_string(&root.join(name))
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|value| value.as_object().cloned())
    };
    if json_object("package.json").is_some_and(|manifest| {
        manifest.get("pi").is_some_and(serde_json::Value::is_object)
            || manifest
                .get("pi-subagents")
                .is_some_and(serde_json::Value::is_object)
    }) {
        return true;
    }
    if json_object("settings.json").is_some_and(|settings| {
        PI_SETTINGS_KEYS
            .iter()
            .any(|key| settings.contains_key(*key))
    }) {
        return true;
    }
    imports_pi_package(fs, root, 0) || imports_pi_package(fs, &root.join("extensions"), 1)
}

/// Returns true if a `.ts`, `.js` or `.mjs` file in `dir` (and, at depth 1, its
/// subdirectories) imports pi. Only the first 64 KiB of each file is searched.
fn imports_pi_package(fs: &dyn crate::fs::FileSystem, dir: &Path, depth: usize) -> bool {
    const MAX_BYTES: usize = 64 * 1024;
    let Ok(entries) = fs.read_dir(dir) else {
        return false;
    };
    entries.iter().any(|entry| {
        if entry.metadata.is_symlink {
            return false;
        }
        if entry.metadata.is_dir {
            return depth == 1 && imports_pi_package(fs, &entry.path, depth + 1);
        }
        let script = entry
            .path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e, "ts" | "js" | "mjs"));
        script
            && fs.read_to_string(&entry.path).is_ok_and(|text| {
                let head = &text[..text.floor_char_boundary(MAX_BYTES.min(text.len()))];
                PI_PACKAGES.iter().any(|package| head.contains(package))
            })
    })
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;
    use tempfile::TempDir;

    /// A pi agent with pi-only fields, lowercase tools and a provider model id.
    const PI_AGENT: &str = "---\nname: explore\ndescription: Explore the codebase\ntools: read, bash\nmodel: amazon-bedrock/claude\nthinking: high\nmax_turns: 4\n---\nExplore.\n";

    /// Write `files` under a temp root and return the CC-AG rules reported for
    /// `agent`.
    fn cc_ag_rules(files: &[(&str, &str)], agent: &str) -> Vec<String> {
        let temp = TempDir::new().unwrap();
        for (name, content) in files {
            let path = temp.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, content).unwrap();
        }
        let path = temp.path().join(agent);
        let content = std::fs::read_to_string(&path).unwrap();
        AgentValidator
            .validate(&path, &content, &LintConfig::default())
            .into_iter()
            .filter(|d| d.rule.starts_with("CC-AG-"))
            .map(|d| d.rule)
            .collect()
    }

    #[test]
    fn test_pi_agent_roots_skip_claude_rules() {
        let agent = ("agent/agents/explore.md", PI_AGENT);
        for marker in [
            (
                "agent/settings.json",
                r#"{"defaultProvider": "amazon-bedrock"}"#,
            ),
            (
                "agent/package.json",
                r#"{"name": "x", "pi": {"extensions": ["./index.ts"]}}"#,
            ),
            (
                "agent/package.json",
                r#"{"pi-subagents": {"agents": ["./agents"]}}"#,
            ),
            (
                "agent/index.ts",
                "import type { ExtensionAPI } from \"@earendil-works/pi-coding-agent\";\n",
            ),
            (
                "agent/extensions/dash/index.ts",
                "import { Text } from \"@mariozechner/pi-coding-agent\";\n",
            ),
        ] {
            let rules = cc_ag_rules(&[agent, marker], agent.0);
            assert!(rules.is_empty(), "{marker:?}: {rules:?}");
        }
    }

    #[test]
    fn test_pi_markers_do_not_hide_claude_agents() {
        let pi_settings = r#"{"defaultProvider": "amazon-bedrock"}"#;
        // A Claude plugin root wins over pi markers.
        let rules = cc_ag_rules(
            &[
                ("plugin/.claude-plugin/plugin.json", r#"{"name": "plugin"}"#),
                ("plugin/settings.json", pi_settings),
                ("plugin/agents/explore.md", PI_AGENT),
            ],
            "plugin/agents/explore.md",
        );
        assert!(rules.contains(&"CC-AG-009".to_string()), "{rules:?}");

        // `.claude/agents` is always Claude's, even in a pi package repo.
        let rules = cc_ag_rules(
            &[
                (
                    "repo/package.json",
                    r#"{"pi": {"extensions": ["./index.ts"]}}"#,
                ),
                ("repo/.claude/agents/explore.md", PI_AGENT),
            ],
            "repo/.claude/agents/explore.md",
        );
        assert!(rules.contains(&"CC-AG-009".to_string()), "{rules:?}");

        // Claude-only settings, malformed settings, and a package.json without
        // a `pi` object are not pi markers.
        for marker in [
            (
                "plugin/settings.json",
                r#"{"model": "sonnet", "permissions": {}}"#,
            ),
            ("plugin/settings.json", "{not json"),
            ("plugin/settings.json", r#"["defaultProvider"]"#),
            (
                "plugin/package.json",
                r#"{"name": "can-i-help", "keywords": ["pi-package"]}"#,
            ),
        ] {
            let rules = cc_ag_rules(
                &[marker, ("plugin/agents/explore.md", PI_AGENT)],
                "plugin/agents/explore.md",
            );
            assert!(
                rules.contains(&"CC-AG-009".to_string()),
                "{marker:?}: {rules:?}"
            );
        }
    }

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = AgentValidator;
        validator.validate(
            Path::new(".claude/agents/test-agent.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_with_path(path: &Path, content: &str) -> Vec<Diagnostic> {
        let validator = AgentValidator;
        validator.validate(path, content, &LintConfig::default())
    }

    #[test]
    fn test_cc_ag_019_experimental_cache_ttl_is_recognized() {
        for ttl in ["5m", "1h"] {
            let content = format!(
                "---\nname: repo-auditor\ndescription: Audits repositories\nexperimental:\n  cacheTtl: {ttl}\n---\nReview the repository."
            );
            let diagnostics = validate(&content);
            assert!(
                diagnostics
                    .iter()
                    .all(|d| d.rule != "CC-AG-007" && d.rule != "CC-AG-019"),
                "documented experimental map should parse and be recognized: {diagnostics:?}"
            );
        }
        let diagnostics = validate(
            "---\nname: repo-auditor\ndescription: Audits repositories\ncacheTtl: 1h\n---\nReview the repository.",
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CC-AG-019"));
    }

    #[test]
    fn test_cc_ag_007_experimental_requires_map() {
        for value in ["true", "5m", "[5m, 1h]"] {
            let content = format!(
                "---\nname: repo-auditor\ndescription: Audits repositories\nexperimental: {value}\n---\nReview the repository."
            );
            assert!(
                validate(&content).iter().any(|d| d.rule == "CC-AG-007"),
                "experimental must be a map, not {value}"
            );
        }
    }

    #[test]
    fn test_cc_ag_010_all_mcp_deny_pattern_only() {
        for entry in ["mcp__*", "[mcp__*]"] {
            let content = format!(
                "---\nname: local-only\ndescription: Works without MCP tools\ndisallowedTools: {entry}\n---\nReview local files."
            );
            let diagnostics = validate(&content);
            assert!(
                diagnostics
                    .iter()
                    .all(|d| d.rule != "CC-AG-007" && d.rule != "CC-AG-010"),
                "exact all-server deny pattern should be accepted: {diagnostics:?}"
            );
        }
        for (field, rule, entry) in [
            ("tools", "CC-AG-009", "mcp__*"),
            ("disallowedTools", "CC-AG-010", "mcp__github-*"),
            ("disallowedTools", "CC-AG-010", "mcp__*__read"),
            ("disallowedTools", "CC-AG-010", "MCP__*"),
            ("disallowedTools", "CC-AG-010", "mcp__"),
        ] {
            let content = format!(
                "---\nname: local-only\ndescription: Works without MCP tools\n{field}: [{entry}]\n---\nReview local files."
            );
            let diagnostics = validate(&content);
            assert!(
                diagnostics.iter().any(|d| d.rule == rule),
                "{field}: {entry} must remain invalid: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_documented_handback_tool_is_recognized_in_both_lists() {
        for field in ["tools", "disallowedTools"] {
            let content = format!(
                "---\nname: reporter\ndescription: Reports work\n{field}: [SubagentHandback, UnknownHandback]\n---\nReport findings."
            );
            let diagnostics = validate(&content);
            let hits: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-AG-009" || d.rule == "CC-AG-010")
                .collect();
            assert_eq!(hits.len(), 1, "{field}: {diagnostics:?}");
            assert!(hits[0].message.contains("UnknownHandback"));
        }
    }

    #[test]
    fn test_cc_ag_011_accepts_documented_handler_types() {
        for handler in [
            "type: command\n          command: echo checked",
            "type: http\n          url: https://example.com/hooks",
            "type: mcp_tool\n          server: audit\n          tool: check",
            "type: prompt\n          prompt: Check the result",
            "type: agent\n          prompt: Check the result",
        ] {
            let content = format!(
                "---\nname: reviewer\ndescription: Reviews changes\nhooks:\n  Stop:\n    - hooks:\n        - {handler}\n---\nReview the changes."
            );
            let diagnostics = validate(&content);
            assert!(
                diagnostics
                    .iter()
                    .all(|d| d.rule != "CC-AG-007" && d.rule != "CC-AG-011"),
                "documented handler should be accepted: {diagnostics:?}"
            );
        }
        let diagnostics = validate(
            "---\nname: reviewer\ndescription: Reviews changes\nhooks:\n  Stop:\n    - hooks:\n        - type: remote_agent\n---\nReview the changes.",
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CC-AG-011"));
    }

    // ===== CC-AG-001 Tests: Missing Name Field =====

    #[test]
    fn test_cc_ag_001_missing_name() {
        let content = r#"---
description: A test agent
---
Agent instructions here"#;

        let diagnostics = validate(content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();

        assert_eq!(cc_ag_001.len(), 1);
        assert_eq!(cc_ag_001[0].level, DiagnosticLevel::Error);
        // Message now states the effect: the file is treated as documentation.
        assert!(cc_ag_001[0].message.contains("no 'name' field"));
        assert!(cc_ag_001[0].message.contains("documentation"));
    }

    #[test]
    fn test_cc_ag_001_empty_name() {
        let content = r#"---
name: ""
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();

        assert_eq!(cc_ag_001.len(), 1);
    }

    #[test]
    fn test_cc_ag_001_whitespace_name() {
        let content = r#"---
name: "   "
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();

        assert_eq!(cc_ag_001.len(), 1);
    }

    #[test]
    fn test_cc_ag_001_valid_name() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();

        assert_eq!(cc_ag_001.len(), 0);
    }

    // ===== CC-AG-002 Tests: Missing Description Field =====

    #[test]
    fn test_cc_ag_002_missing_description() {
        let content = r#"---
name: my-agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-002")
            .collect();

        assert_eq!(cc_ag_002.len(), 1);
        assert_eq!(cc_ag_002[0].level, DiagnosticLevel::Error);
        assert!(
            cc_ag_002[0]
                .message
                .contains("missing required 'description'")
        );
    }

    #[test]
    fn test_cc_ag_002_empty_description() {
        let content = r#"---
name: my-agent
description: ""
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-002")
            .collect();

        assert_eq!(cc_ag_002.len(), 1);
    }

    #[test]
    fn test_cc_ag_002_valid_description() {
        let content = r#"---
name: my-agent
description: This agent helps with testing
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-002")
            .collect();

        assert_eq!(cc_ag_002.len(), 0);
    }

    // ===== CC-AG-001 auto-fix tests =====

    #[test]
    fn test_cc_ag_001_has_fix() {
        let content = "---\ndescription: A test agent\n---\nAgent instructions";
        let diagnostics = validate_with_path(Path::new(".claude/agents/reviewer.md"), content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();
        assert_eq!(cc_ag_001.len(), 1);
        assert!(cc_ag_001[0].has_fixes(), "CC-AG-001 should have auto-fix");
        let fix = &cc_ag_001[0].fixes[0];
        assert!(!fix.safe, "CC-AG-001 fix should be unsafe");
        assert!(
            fix.replacement.contains("name: reviewer"),
            "Fix should insert name derived from filename, got: {}",
            fix.replacement
        );
    }

    // ===== CC-AG-002 auto-fix tests =====

    #[test]
    fn test_cc_ag_002_has_fix() {
        let content = "---\nname: my-agent\n---\nAgent instructions";
        let diagnostics = validate(content);
        let cc_ag_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-002")
            .collect();
        assert_eq!(cc_ag_002.len(), 1);
        assert!(cc_ag_002[0].has_fixes(), "CC-AG-002 should have auto-fix");
        let fix = &cc_ag_002[0].fixes[0];
        assert!(!fix.safe, "CC-AG-002 fix should be unsafe");
        assert!(
            fix.replacement.contains("description:"),
            "Fix should insert description placeholder"
        );
    }

    // ===== CC-AG-013 auto-fix tests =====

    // ===== CC-AG-001 YAML injection prevention =====

    #[test]
    fn test_cc_ag_001_sanitizes_special_filename() {
        let content = "---\ndescription: A test agent\n---\nAgent instructions";
        let diagnostics =
            validate_with_path(Path::new(".claude/agents/my: agent\"file.md"), content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();
        assert_eq!(cc_ag_001.len(), 1);
        assert!(cc_ag_001[0].has_fixes(), "CC-AG-001 should have auto-fix");
        let fix = &cc_ag_001[0].fixes[0];
        // The name should be sanitized - no colons or quotes
        assert!(
            !fix.replacement.contains(':') || fix.replacement.starts_with("name:"),
            "Derived name should be sanitized to prevent YAML injection, got: {}",
            fix.replacement
        );
    }

    // ===== CC-AG-003 Tests: Invalid Model Value =====

    #[test]
    fn test_cc_ag_003_invalid_model() {
        let content = r#"---
name: my-agent
description: A test agent
model: gpt-4
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 1);
        assert_eq!(cc_ag_003[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_003[0].message.contains("Invalid model"));
        assert!(cc_ag_003[0].message.contains("gpt-4"));
    }

    #[test]
    fn test_cc_ag_003_has_unsafe_fix() {
        let content = r#"---
name: my-agent
description: A test agent
model: gpt-4
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003 = diagnostics
            .iter()
            .find(|d| d.rule == "CC-AG-003")
            .expect("CC-AG-003 should be reported");

        assert!(cc_ag_003.has_fixes());
        let fix = &cc_ag_003.fixes[0];
        assert_eq!(fix.replacement, "sonnet");
        assert!(!fix.safe);
    }

    #[test]
    fn test_cc_ag_003_valid_model_sonnet() {
        let content = r#"---
name: my-agent
description: A test agent
model: sonnet
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 0);
    }

    #[test]
    fn test_cc_ag_003_valid_model_opus() {
        let content = r#"---
name: my-agent
description: A test agent
model: opus
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 0);
    }

    #[test]
    fn test_cc_ag_003_valid_model_haiku() {
        let content = r#"---
name: my-agent
description: A test agent
model: haiku
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 0);
    }

    #[test]
    fn test_cc_ag_003_valid_model_inherit() {
        let content = r#"---
name: my-agent
description: A test agent
model: inherit
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 0);
    }

    #[test]
    fn test_cc_ag_003_no_model_ok() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 0);
    }

    // ===== CC-AG-004 Tests: Invalid Permission Mode =====

    #[test]
    fn test_cc_ag_004_invalid_permission_mode() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: admin
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 1);
        assert_eq!(cc_ag_004[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_004[0].message.contains("Invalid permissionMode"));
        assert!(cc_ag_004[0].message.contains("admin"));
    }

    #[test]
    fn test_cc_ag_004_has_unsafe_fix() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: admin
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004 = diagnostics
            .iter()
            .find(|d| d.rule == "CC-AG-004")
            .expect("CC-AG-004 should be reported");

        assert!(cc_ag_004.has_fixes());
        let fix = &cc_ag_004.fixes[0];
        assert_eq!(fix.replacement, "default");
        assert!(!fix.safe);
    }

    #[test]
    fn test_cc_ag_004_valid_permission_mode_default() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: default
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    #[test]
    fn test_cc_ag_004_valid_permission_mode_accept_edits() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: acceptEdits
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    #[test]
    fn test_cc_ag_004_valid_permission_mode_dont_ask() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: dontAsk
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    #[test]
    fn test_cc_ag_004_valid_permission_mode_bypass() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: bypassPermissions
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    #[test]
    fn test_cc_ag_004_valid_permission_mode_plan() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: plan
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    /// `delegate` is not a documented permission mode. It appears in neither
    /// the sub-agents nor the permission-modes reference, and was added without
    /// a cited source, so it is now rejected like any other unknown value.
    #[test]
    fn test_cc_ag_004_delegate_is_not_a_documented_mode() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: delegate
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(
            cc_ag_004.len(),
            1,
            "`delegate` is undocumented and must be flagged, got: {diagnostics:?}"
        );
    }

    /// The six documented modes plus the `manual` alias for `default`
    /// (v2.1.200+). `auto` and `manual` were previously rejected.
    #[test]
    fn test_cc_ag_004_documented_modes_accepted() {
        for mode in [
            "default",
            "manual",
            "acceptEdits",
            "plan",
            "auto",
            "dontAsk",
            "bypassPermissions",
        ] {
            let content = format!(
                r#"---
name: my-agent
description: A test agent
permissionMode: {mode}
---
Agent instructions"#
            );

            let diagnostics = validate(&content);
            let cc_ag_004: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-AG-004")
                .collect();

            assert!(
                cc_ag_004.is_empty(),
                "documented permission mode '{mode}' must be accepted, got: {cc_ag_004:?}"
            );
        }
    }

    /// `model` "Accepts the same values as the `--model` flag", which is the
    /// model-config alias table. `fable` and the rest were rejected before.
    #[test]
    fn test_cc_ag_003_documented_model_aliases_accepted() {
        for model in [
            "default",
            "best",
            "fable",
            "sonnet",
            "opus",
            "haiku",
            "opusplan",
            "sonnet[1m]",
            "opus[1m]",
            "inherit",
            "claude-opus-5",
        ] {
            let content = format!(
                r#"---
name: my-agent
description: A test agent
model: {model}
---
Agent instructions"#
            );

            let diagnostics = validate(&content);
            let cc_ag_003: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-AG-003")
                .collect();

            assert!(
                cc_ag_003.is_empty(),
                "documented model value '{model}' must be accepted, got: {cc_ag_003:?}"
            );
        }
    }

    /// The sub-agents reference's own verbatim example carries parenthesized
    /// arguments containing commas and spaces. Splitting the string form on
    /// `,`/space before stripping at `(` shattered these into fragments like
    /// `researcher)` and `run`, which then tripped CC-AG-009.
    #[test]
    fn test_cc_ag_009_parenthesized_args_not_split() {
        let content = r#"---
name: my-agent
description: A test agent
tools: Agent(worker, researcher), Read, Bash(npm run test:*)
---
Agent instructions"#;

        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();

        assert!(
            hits.is_empty(),
            "parenthesized tool arguments must survive tokenization, got: {hits:?}"
        );
    }

    #[test]
    fn test_cc_ag_004_no_permission_mode_ok() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    // ===== CC-AG-005 Tests: Referenced Skill Not Found =====

    #[test]
    fn test_cc_ag_005_missing_skill() {
        let temp = TempDir::new().unwrap();
        let claude_dir = temp.path().join(".claude");
        let agents_dir = claude_dir.join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();

        let agent_path = agents_dir.join("test-agent.md");

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - nonexistent-skill
---
Agent instructions"#;

        let diagnostics = validate_with_path(&agent_path, content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        assert_eq!(cc_ag_005.len(), 1);
        assert_eq!(cc_ag_005[0].level, DiagnosticLevel::Warning);
        assert!(cc_ag_005[0].message.contains("nonexistent-skill"));
        assert!(cc_ag_005[0].message.contains("not found"));
    }

    #[test]
    fn test_cc_ag_005_existing_skill() {
        let temp = TempDir::new().unwrap();
        let claude_dir = temp.path().join(".claude");
        let agents_dir = claude_dir.join("agents");
        let skills_dir = claude_dir.join("skills").join("my-skill");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: A skill\n---\nBody",
        )
        .unwrap();

        let agent_path = agents_dir.join("test-agent.md");

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - my-skill
---
Agent instructions"#;

        let diagnostics = validate_with_path(&agent_path, content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        assert_eq!(cc_ag_005.len(), 0);
    }

    #[test]
    fn test_cc_ag_005_multiple_missing_skills() {
        let temp = TempDir::new().unwrap();
        let claude_dir = temp.path().join(".claude");
        let agents_dir = claude_dir.join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();

        let agent_path = agents_dir.join("test-agent.md");

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - missing-one
  - missing-two
  - missing-three
---
Agent instructions"#;

        let diagnostics = validate_with_path(&agent_path, content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        assert_eq!(cc_ag_005.len(), 3);
    }

    #[test]
    fn test_cc_ag_005_no_skills_ok() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        assert_eq!(cc_ag_005.len(), 0);
    }

    // ===== CC-AG-006 Tests: Tool/Disallowed Conflict =====

    #[test]
    fn test_cc_ag_006_tool_conflict() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
  - Read
  - Write
disallowedTools:
  - Bash
  - Edit
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();

        assert_eq!(cc_ag_006.len(), 1);
        assert_eq!(cc_ag_006[0].level, DiagnosticLevel::Warning);
        assert!(cc_ag_006[0].message.contains("Bash"));
        assert!(cc_ag_006[0].message.contains("both"));
    }

    #[test]
    fn test_cc_ag_006_multiple_conflicts() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
  - Read
  - Write
disallowedTools:
  - Bash
  - Read
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();

        assert_eq!(cc_ag_006.len(), 1);
        // Should mention both conflicting tools
        assert!(cc_ag_006[0].message.contains("Bash") && cc_ag_006[0].message.contains("Read"));
    }

    #[test]
    fn test_cc_ag_006_no_conflict() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
  - Read
disallowedTools:
  - Write
  - Edit
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();

        assert_eq!(cc_ag_006.len(), 0);
    }

    #[test]
    fn test_cc_ag_006_only_tools_ok() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
  - Read
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();

        assert_eq!(cc_ag_006.len(), 0);
    }

    #[test]
    fn test_cc_ag_006_only_disallowed_ok() {
        let content = r#"---
name: my-agent
description: A test agent
disallowedTools:
  - Bash
  - Read
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();

        assert_eq!(cc_ag_006.len(), 0);
    }

    // ===== Parse Error Tests =====

    #[test]
    fn test_no_frontmatter() {
        let content = "Just agent instructions without frontmatter";

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert_eq!(parse_errors.len(), 1);
        // Changed expectation: a file without frontmatter is documentation to
        // Claude Code ("treats the file as documentation kept beside your
        // agents"), so this is now an info note, not an error.
        assert_eq!(parse_errors[0].level, DiagnosticLevel::Info);
        assert!(parse_errors[0].message.contains("documentation"));
    }

    #[test]
    fn test_invalid_yaml() {
        let content = r#"---
name: [invalid yaml
description: test
---
Body"#;

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert_eq!(parse_errors.len(), 1);
        assert!(parse_errors[0].message.contains("Failed to parse"));
    }

    // ===== Valid Agent Tests =====

    #[test]
    fn test_valid_agent_minimal() {
        let content = r#"---
name: my-agent
description: A helpful agent for testing
---
Agent instructions here"#;

        let diagnostics = validate(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();

        assert!(errors.is_empty());
    }

    #[test]
    fn test_valid_agent_full() {
        let content = r#"---
name: full-agent
description: A fully configured agent
model: opus
permissionMode: acceptEdits
tools:
  - Bash
  - Read
  - Write
disallowedTools:
  - Edit
---
Agent instructions with full configuration"#;

        let diagnostics = validate(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();

        assert!(errors.is_empty());
    }

    // ===== Fixture Tests =====

    #[test]
    fn test_fixture_missing_name() {
        let content = include_str!("../../../../tests/fixtures/invalid/agents/missing-name.md");
        let diagnostics = validate(content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();
        assert!(!cc_ag_001.is_empty());
    }

    #[test]
    fn test_fixture_missing_description() {
        let content =
            include_str!("../../../../tests/fixtures/invalid/agents/missing-description.md");
        let diagnostics = validate(content);
        let cc_ag_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-002")
            .collect();
        assert!(!cc_ag_002.is_empty());
    }

    #[test]
    fn test_fixture_invalid_model() {
        let content = include_str!("../../../../tests/fixtures/invalid/agents/invalid-model.md");
        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();
        assert!(!cc_ag_003.is_empty());
    }

    #[test]
    fn test_fixture_invalid_permission() {
        let content =
            include_str!("../../../../tests/fixtures/invalid/agents/invalid-permission.md");
        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();
        assert!(!cc_ag_004.is_empty());
    }

    #[test]
    fn test_fixture_tool_conflict() {
        let content = include_str!("../../../../tests/fixtures/invalid/agents/tool-conflict.md");
        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();
        assert!(!cc_ag_006.is_empty());
    }

    #[test]
    fn test_fixture_valid_agent() {
        let content = include_str!("../../../../tests/fixtures/valid/agents/valid-agent.md");
        let diagnostics = validate(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty());
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_cc_ag_005_empty_skills_array() {
        let content = r#"---
name: my-agent
description: A test agent
skills: []
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();
        assert_eq!(cc_ag_005.len(), 0);
    }

    #[test]
    fn test_cc_ag_006_empty_tools_array() {
        let content = r#"---
name: my-agent
description: A test agent
tools: []
disallowedTools:
  - Bash
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();
        assert_eq!(cc_ag_006.len(), 0);
    }

    #[test]
    fn test_cc_ag_006_empty_disallowed_array() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
disallowedTools: []
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();
        assert_eq!(cc_ag_006.len(), 0);
    }

    #[test]
    fn test_skill_name_path_traversal_rejected() {
        let temp = TempDir::new().unwrap();
        let claude_dir = temp.path().join(".claude");
        let agents_dir = claude_dir.join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();

        let agent_path = agents_dir.join("test-agent.md");

        // Try path traversal attack
        let content = r#"---
name: my-agent
description: A test agent
skills:
  - ../../../etc/passwd
---
Agent instructions"#;

        let diagnostics = validate_with_path(&agent_path, content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();
        // Should report as not found (rejected), not as a security breach
        assert_eq!(cc_ag_005.len(), 1);
    }

    #[test]
    fn test_is_safe_skill_name() {
        assert!(AgentValidator::is_safe_skill_name("my-skill"));
        assert!(AgentValidator::is_safe_skill_name("skill_name"));
        assert!(AgentValidator::is_safe_skill_name("skill123"));
        assert!(!AgentValidator::is_safe_skill_name("../parent"));
        assert!(!AgentValidator::is_safe_skill_name("path/to/skill"));
        assert!(!AgentValidator::is_safe_skill_name(".hidden"));
        assert!(!AgentValidator::is_safe_skill_name(""));
    }

    // ===== Config Wiring Tests =====

    #[test]
    fn test_config_disabled_agents_category_returns_empty() {
        let mut config = LintConfig::default();
        config.rules_mut().agents = false;

        let content = r#"---
description: A test agent without name
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(Path::new("test-agent.md"), content, &config);

        // CC-AG-001 should not fire when agents category is disabled
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();
        assert_eq!(cc_ag_001.len(), 0);
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-AG-001".to_string()];

        // Agent missing both name and description
        let content = r#"---
model: sonnet
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(Path::new("test-agent.md"), content, &config);

        // CC-AG-001 should not fire when specifically disabled
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();
        assert_eq!(cc_ag_001.len(), 0);

        // But CC-AG-002 should still fire (description is missing)
        let cc_ag_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-002")
            .collect();
        assert_eq!(cc_ag_002.len(), 1);
    }

    #[test]
    fn test_config_cursor_target_disables_agent_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.set_target(TargetTool::Cursor);

        let content = r#"---
description: Agent without name
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(Path::new("test-agent.md"), content, &config);

        // CC-AG-* rules should not fire for Cursor target
        let agent_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CC-AG-"))
            .collect();
        assert_eq!(agent_rules.len(), 0);
    }

    #[test]
    fn test_config_claude_code_target_enables_agent_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.set_target(TargetTool::ClaudeCode);

        let content = r#"---
description: Agent without name
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(Path::new("test-agent.md"), content, &config);

        // CC-AG-001 should fire for ClaudeCode target
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();
        assert_eq!(cc_ag_001.len(), 1);
    }

    // ===== MockFileSystem Integration Tests for CC-AG-005 =====

    #[test]
    fn test_cc_ag_005_with_mock_fs_missing_skill() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        // Set up directory structure: .claude/agents exists but skill doesn't
        mock_fs.add_dir("/project/.claude");
        mock_fs.add_dir("/project/.claude/agents");
        // No skill directory created

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - nonexistent-skill
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(
            Path::new("/project/.claude/agents/test-agent.md"),
            content,
            &config,
        );

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        assert_eq!(cc_ag_005.len(), 1);
        assert!(cc_ag_005[0].message.contains("nonexistent-skill"));
        assert!(cc_ag_005[0].message.contains("not found"));
    }

    #[test]
    fn test_cc_ag_005_with_mock_fs_existing_skill() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        // Set up complete directory structure with skill
        mock_fs.add_dir("/project/.claude");
        mock_fs.add_dir("/project/.claude/agents");
        mock_fs.add_dir("/project/.claude/skills");
        mock_fs.add_dir("/project/.claude/skills/my-skill");
        mock_fs.add_file(
            "/project/.claude/skills/my-skill/SKILL.md",
            "---\nname: my-skill\ndescription: A skill\n---\nBody",
        );

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - my-skill
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(
            Path::new("/project/.claude/agents/test-agent.md"),
            content,
            &config,
        );

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        // No errors - skill exists
        assert_eq!(cc_ag_005.len(), 0);
    }

    #[test]
    fn test_cc_ag_005_with_mock_fs_multiple_skills_mixed() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        // Set up structure with one skill present, two missing
        mock_fs.add_dir("/project/.claude");
        mock_fs.add_dir("/project/.claude/agents");
        mock_fs.add_dir("/project/.claude/skills");
        mock_fs.add_dir("/project/.claude/skills/existing-skill");
        mock_fs.add_file(
            "/project/.claude/skills/existing-skill/SKILL.md",
            "---\nname: existing-skill\ndescription: Exists\n---\nBody",
        );
        // missing-skill-1 and missing-skill-2 are not created

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - existing-skill
  - missing-skill-1
  - missing-skill-2
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(
            Path::new("/project/.claude/agents/test-agent.md"),
            content,
            &config,
        );

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        // Should report 2 missing skills
        assert_eq!(cc_ag_005.len(), 2);
        let messages: Vec<&str> = cc_ag_005.iter().map(|d| d.message.as_str()).collect();
        assert!(messages.iter().any(|m| m.contains("missing-skill-1")));
        assert!(messages.iter().any(|m| m.contains("missing-skill-2")));
    }

    #[test]
    fn test_cc_ag_005_with_mock_fs_path_traversal_rejected() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_dir("/project/.claude");
        mock_fs.add_dir("/project/.claude/agents");
        // Even if we create a file at the traversal target, it should be rejected
        mock_fs.add_file("/etc/passwd", "root:x:0:0:root:/root:/bin/bash");

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - ../../../etc/passwd
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(
            Path::new("/project/.claude/agents/test-agent.md"),
            content,
            &config,
        );

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        // Path traversal attempts should be rejected as "not found"
        assert_eq!(cc_ag_005.len(), 1);
        assert!(cc_ag_005[0].message.contains("not found"));
    }

    #[test]
    fn test_cc_ag_005_with_mock_fs_no_claude_directory() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        // No .claude directory at all
        mock_fs.add_dir("/project");

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - some-skill
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics =
            validator.validate(Path::new("/project/random/test-agent.md"), content, &config);

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        // Without .claude directory, no project root found, so no CC-AG-005 errors
        // (can't validate skill references without knowing where to look)
        assert_eq!(cc_ag_005.len(), 0);
    }

    #[test]
    fn test_cc_ag_005_with_mock_fs_empty_skills_array() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_dir("/project/.claude");
        mock_fs.add_dir("/project/.claude/agents");

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills: []
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(
            Path::new("/project/.claude/agents/test-agent.md"),
            content,
            &config,
        );

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        // Empty skills array = no errors
        assert_eq!(cc_ag_005.len(), 0);
    }

    // ===== Additional CC-AG-007 Parse Error Tests =====

    #[test]
    fn test_cc_ag_007_empty_file() {
        let content = "";

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert_eq!(parse_errors.len(), 1);
    }

    #[test]
    fn test_cc_ag_007_invalid_yaml_syntax() {
        // Test with actually invalid YAML that will fail parsing
        let content = "---\nname: test\n  bad indent: value\n---\nBody";

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert_eq!(parse_errors.len(), 1);
    }

    #[test]
    fn test_cc_ag_007_valid_yaml_no_error() {
        let content = r#"---
name: valid-agent
description: A properly formatted agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert!(parse_errors.is_empty());
    }

    #[test]
    fn test_cc_ag_007_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-AG-007".to_string()];

        let content = r#"---
name: [invalid yaml
---
Body"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(
            Path::new("/project/.claude/agents/test.md"),
            content,
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-AG-007"));
    }

    // ===== Additional CC-AG rule edge cases =====

    #[test]
    fn test_cc_ag_003_all_valid_models() {
        // Must match VALID_MODELS constant in agent.rs
        let valid_models = VALID_MODELS;

        for model in valid_models {
            let content = format!(
                "---\nname: test\ndescription: Test agent\nmodel: {}\n---\nBody",
                model
            );

            let diagnostics = validate(&content);
            let cc_ag_003: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-AG-003")
                .collect();
            assert!(cc_ag_003.is_empty(), "Model '{}' should be valid", model);
        }
    }

    #[test]
    fn test_cc_ag_004_all_valid_permission_modes() {
        // Must match VALID_PERMISSION_MODES constant in agent.rs
        let valid_modes = VALID_PERMISSION_MODES;

        for mode in valid_modes {
            let content = format!(
                "---\nname: test\ndescription: Test agent\npermissionMode: {}\n---\nBody",
                mode
            );

            let diagnostics = validate(&content);
            let cc_ag_004: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-AG-004")
                .collect();
            assert!(
                cc_ag_004.is_empty(),
                "Permission mode '{}' should be valid",
                mode
            );
        }
    }

    // ===== CC-AG-007 line/column accuracy tests =====

    #[test]
    fn test_cc_ag_007_type_error_still_reported() {
        // tools accepts a string or a list, but not a bare scalar like an
        // integer - that type error still surfaces as CC-AG-007. (Because the
        // field uses a serde untagged enum, the error attributes to the
        // frontmatter start rather than the exact field line; precise line
        // reporting for genuine YAML syntax errors is covered by
        // test_cc_ag_007_invalid_yaml_reports_correct_line.)
        let content = "---\nname: test\ndescription: test\ntools: 123\n---\nBody";

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert_eq!(parse_errors.len(), 1);
        // The untagged enum attributes the type error to the frontmatter start
        // (line 2 here) rather than the exact field line - guard that it lands
        // within the frontmatter, not at the missing-frontmatter fallback (1).
        assert_eq!(parse_errors[0].line, 2);
    }

    #[test]
    fn test_cc_ag_007_comma_separated_tools_accepted() {
        // Reproduces #957: the canonical sub-agent form `tools: Read, Glob, Grep`
        // (a comma-separated string) must parse without a CC-AG-007 error.
        let content = "---\nname: code-reviewer\ndescription: Reviews code\ntools: Read, Glob, Grep\nmodel: sonnet\n---\nBody";
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().all(|d| d.rule != "CC-AG-007"),
            "comma-separated tools string must not trip CC-AG-007, got: {:?}",
            diagnostics
                .iter()
                .filter(|d| d.rule == "CC-AG-007")
                .collect::<Vec<_>>()
        );
        // The split tools (Read/Glob/Grep) are all valid -> no CC-AG-009 either.
        assert!(
            diagnostics.iter().all(|d| d.rule != "CC-AG-009"),
            "valid tools in string form must not trip CC-AG-009"
        );
    }

    #[test]
    fn test_cc_ag_007_invalid_yaml_reports_correct_line() {
        // Invalid YAML on line 2
        let content = "---\nname: [invalid yaml\n---\nBody";

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert_eq!(parse_errors.len(), 1);
        assert_eq!(
            parse_errors[0].line, 2,
            "Expected error on line 2 (invalid YAML key), got {}",
            parse_errors[0].line
        );
    }

    #[test]
    fn test_cc_ag_007_reports_column() {
        // tools as a bare integer is a genuine type error (string/list accepted)
        let content = "---\nname: test\ndescription: test\ntools: 123\n---\nBody";

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert_eq!(parse_errors.len(), 1);
        assert!(
            parse_errors[0].column > 0,
            "Expected column > 0 when location is available, got {}",
            parse_errors[0].column
        );
    }

    #[test]
    fn test_cc_ag_007_missing_frontmatter_still_reports_line_1() {
        let content = "Just agent instructions without frontmatter";

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert_eq!(parse_errors.len(), 1);
        assert_eq!(parse_errors[0].line, 1);
        assert_eq!(parse_errors[0].column, 0);
    }

    #[test]
    fn test_cc_ag_006_both_empty_arrays_ok() {
        let content = r#"---
name: test
description: Test agent
tools: []
disallowedTools: []
---
Body"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();
        assert!(cc_ag_006.is_empty());
    }

    // ===== CC-AG-008 Tests: Invalid Memory Scope =====

    #[test]
    fn test_cc_ag_008_invalid_memory_scope() {
        let content = r#"---
name: my-agent
description: A test agent
memory: global
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-008")
            .collect();

        assert_eq!(cc_ag_008.len(), 1);
        assert_eq!(cc_ag_008[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_008[0].message.contains("global"));
    }

    #[test]
    fn test_cc_ag_008_valid_memory_user() {
        let content = r#"---
name: my-agent
description: A test agent
memory: user
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-008")
            .collect();
        assert_eq!(cc_ag_008.len(), 0);
    }

    #[test]
    fn test_cc_ag_008_valid_memory_project() {
        let content = r#"---
name: my-agent
description: A test agent
memory: project
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-008")
            .collect();
        assert_eq!(cc_ag_008.len(), 0);
    }

    #[test]
    fn test_cc_ag_008_valid_memory_local() {
        let content = r#"---
name: my-agent
description: A test agent
memory: local
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-008")
            .collect();
        assert_eq!(cc_ag_008.len(), 0);
    }

    #[test]
    fn test_cc_ag_008_no_memory_ok() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-008")
            .collect();
        assert_eq!(cc_ag_008.len(), 0);
    }

    #[test]
    fn test_cc_ag_008_autofix_case_insensitive() {
        let content =
            "---\nname: my-agent\ndescription: A test agent\nmemory: User\n---\nAgent instructions";
        let diagnostics = validate(content);
        let cc_ag_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-008")
            .collect();
        assert_eq!(cc_ag_008.len(), 1);
        assert!(
            cc_ag_008[0].has_fixes(),
            "CC-AG-008 should have auto-fix for case mismatch"
        );
        let fix = &cc_ag_008[0].fixes[0];
        assert!(!fix.safe, "CC-AG-008 fix should be unsafe");
        assert_eq!(fix.replacement, "user", "Fix should suggest 'user'");
    }

    #[test]
    fn test_cc_ag_008_no_autofix_nonsense() {
        let content = "---\nname: my-agent\ndescription: A test agent\nmemory: global\n---\nAgent instructions";
        let diagnostics = validate(content);
        let cc_ag_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-008")
            .collect();
        assert_eq!(cc_ag_008.len(), 1);
        // "global" has no close match to user/project/local - no fix
        assert!(
            !cc_ag_008[0].has_fixes(),
            "CC-AG-008 should not auto-fix nonsense values"
        );
    }

    #[test]
    fn test_cc_ag_009_accepts_tools_added_in_recent_releases() {
        // `SendFeedback` (Claude Code v2.1.238) and `ListAgents` (v2.1.224) were
        // missing from KNOWN_AGENT_TOOLS, so agents that list them had a valid
        // configuration reported as an unknown tool name.
        let content = r#"---
name: my-agent
description: A test agent
tools: Read, ListAgents, SendFeedback, SendMessage
---
Agent instructions"#;

        let hits: Vec<_> = validate(content)
            .into_iter()
            .filter(|d| d.rule == "CC-AG-009" || d.rule == "CC-AG-010")
            .collect();

        assert!(
            hits.is_empty(),
            "documented built-in tools must not be flagged, got: {hits:?}"
        );
    }

    // ===== CC-AG-009 Tests: Invalid Tool Name in Tools List =====

    #[test]
    fn test_cc_ag_009_invalid_tool() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
  - UnknownTool
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();

        assert_eq!(cc_ag_009.len(), 1);
        assert_eq!(cc_ag_009[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_009[0].message.contains("UnknownTool"));
    }

    #[test]
    fn test_cc_ag_009_all_valid_tools() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
  - Read
  - Write
  - Edit
  - Grep
  - Glob
  - Task
  - WebFetch
  - WebSearch
  - NotebookEdit
  - Skill
  - StatusBarMessageTool
  - TaskOutput
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();
        assert_eq!(cc_ag_009.len(), 0);
    }

    #[test]
    fn test_cc_ag_009_scoped_tool_valid() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash(git:*)
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();
        assert_eq!(cc_ag_009.len(), 0);
    }

    #[test]
    fn test_cc_ag_009_multiple_invalid() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - FakeTool
  - AnotherFake
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();
        assert_eq!(cc_ag_009.len(), 2);
    }

    #[test]
    fn test_cc_ag_009_mcp_tool_valid() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Read
  - mcp__memory__create_entities
  - mcp__filesystem__read_file
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();
        assert_eq!(
            cc_ag_009.len(),
            0,
            "MCP tools with mcp__ prefix should be accepted"
        );
    }

    #[test]
    fn test_cc_ag_009_mcp_tool_invalid_formats() {
        // Test various invalid MCP formats. `mcp__server` is NOT here: the
        // sub-agents reference documents the server-only form as valid, so it
        // is covered by test_cc_ag_009_mcp_server_only_form_accepted below.
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Read
  - mcp__
  - mcp__bad name
  - mcp__supabase-*
  - MCP__server__tool
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();
        // Should flag: mcp__, mcp__bad name, mcp__supabase-* (glob in the
        // server segment), MCP__server__tool (uppercase prefix)
        assert_eq!(cc_ag_009.len(), 4, "Invalid MCP formats should be rejected");
    }

    /// The sub-agents reference documents `mcp__<server>` and
    /// `mcp__<server>__*` as equivalent server-level patterns for both
    /// `tools` and `disallowedTools`, so neither may be flagged. Regression
    /// for the CC-AG-009/010 half of upstream issue #1277.
    #[test]
    fn test_cc_ag_009_mcp_server_only_form_accepted() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Read
  - mcp__supabase-ssh
  - mcp__playwright__*
disallowedTools:
  - mcp__github
---
Agent instructions"#;

        let diagnostics = validate(content);
        let tool_name_hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009" || d.rule == "CC-AG-010")
            .collect();
        assert!(
            tool_name_hits.is_empty(),
            "server-only MCP form must be accepted in tools and disallowedTools, got: {tool_name_hits:?}"
        );
    }

    /// `Workflow` and the other tools added to the built-in tools reference
    /// must not be flagged as unknown. Subagents never receive `Workflow`,
    /// but Claude Code filters it from the resolved pool rather than
    /// rejecting the name, so CC-AG-009 must stay silent.
    #[test]
    fn test_cc_ag_009_recent_builtin_tools_accepted() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Workflow
  - ToolSearch
  - Artifact
  - ReportFindings
  - SendUserFile
  - EndConversation
  - PowerShell
  - LSP
  - TaskCreate
  - EnterWorktree
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();
        assert!(
            cc_ag_009.is_empty(),
            "documented built-in tools must be accepted, got: {cc_ag_009:?}"
        );
    }

    #[test]
    fn test_cc_ag_009_skill_tool_valid() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Skill
  - StatusBarMessageTool
  - TaskOutput
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();
        assert_eq!(
            cc_ag_009.len(),
            0,
            "Skill, StatusBarMessageTool, and TaskOutput should be recognized"
        );
    }

    // ===== CC-AG-010 Tests: Invalid Tool Name in DisallowedTools =====

    #[test]
    fn test_cc_ag_010_invalid_disallowed_tool() {
        let content = r#"---
name: my-agent
description: A test agent
disallowedTools:
  - Bash
  - FakeTool
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-010")
            .collect();

        assert_eq!(cc_ag_010.len(), 1);
        assert_eq!(cc_ag_010[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_010[0].message.contains("FakeTool"));
    }

    #[test]
    fn test_cc_ag_010_valid_disallowed_tools() {
        let content = r#"---
name: my-agent
description: A test agent
disallowedTools:
  - Bash
  - Write
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-010")
            .collect();
        assert_eq!(cc_ag_010.len(), 0);
    }

    #[test]
    fn test_cc_ag_010_mcp_tool_valid() {
        let content = r#"---
name: my-agent
description: A test agent
disallowedTools:
  - mcp__memory__create_entities
  - mcp__filesystem__read_file
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-010")
            .collect();
        assert_eq!(
            cc_ag_010.len(),
            0,
            "MCP tools with mcp__ prefix should be accepted in disallowedTools"
        );
    }

    #[test]
    fn test_cc_ag_010_mcp_case_sensitive() {
        let content = r#"---
name: my-agent
description: A test agent
disallowedTools:
  - MCP__memory__create_entities
  - Mcp__test__tool
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-010")
            .collect();
        assert_eq!(
            cc_ag_010.len(),
            2,
            "MCP prefix is case-sensitive: MCP__ and Mcp__ should be rejected in disallowedTools"
        );
    }

    #[test]
    fn test_cc_ag_010_scoped_mcp_tool_valid() {
        let content = r#"---
name: my-agent
description: A test agent
disallowedTools:
  - mcp__memory__create_entities(scope:*)
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-010")
            .collect();
        assert_eq!(
            cc_ag_010.len(),
            0,
            "Scoped MCP tools should be accepted in disallowedTools"
        );
    }

    #[test]
    fn test_cc_ag_010_parameter_scoped_agent_tool_valid() {
        // Claude Code v2.1.178 added Tool(param:value) permission syntax;
        // the base tool still needs to be recognized after stripping the
        // parameter clause.
        let content = r#"---
name: my-agent
description: A test agent
disallowedTools:
  - Agent(model:opus)
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-010")
            .collect();
        assert_eq!(
            cc_ag_010.len(),
            0,
            "Agent(model:opus) should be accepted in disallowedTools"
        );
    }

    #[test]
    fn test_cc_ag_009_mcp_case_sensitive() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - MCP__memory__create_entities
  - Mcp__test__tool
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();
        assert_eq!(
            cc_ag_009.len(),
            2,
            "MCP prefix is case-sensitive: MCP__ and Mcp__ should be rejected"
        );
    }

    #[test]
    fn test_cc_ag_009_parameter_scoped_agent_tool_valid() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Agent(model:opus)
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();
        assert_eq!(
            cc_ag_009.len(),
            0,
            "Agent(model:opus) should be accepted in tools"
        );
    }

    #[test]
    fn test_cc_ag_009_scoped_mcp_tool_valid() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - mcp__github__search_repositories(scope:*)
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();
        assert_eq!(cc_ag_009.len(), 0, "Scoped MCP tools should be accepted");
    }

    // ===== CC-AG-011 Tests: Hooks in Agent Frontmatter =====

    #[test]
    fn test_cc_ag_011_invalid_hook_event() {
        let content = r#"---
name: my-agent
description: A test agent
hooks:
  InvalidEvent:
    - matcher: "*"
      hooks:
        - type: command
          command: echo hello
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-011")
            .collect();

        assert_eq!(cc_ag_011.len(), 1);
        assert_eq!(cc_ag_011[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_011[0].message.contains("InvalidEvent"));
    }

    #[test]
    fn test_cc_ag_011_valid_hook_events() {
        let content = r#"---
name: my-agent
description: A test agent
hooks:
  PreToolUse:
    - matcher: "*"
      hooks:
        - type: command
          command: echo hello
  Stop:
    - hooks:
        - type: prompt
          prompt: Summarize
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-011")
            .collect();
        assert_eq!(cc_ag_011.len(), 0);
    }

    #[test]
    fn test_cc_ag_011_hooks_not_object() {
        let content = r#"---
name: my-agent
description: A test agent
hooks: "invalid"
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-011")
            .collect();

        assert_eq!(cc_ag_011.len(), 1);
        assert!(cc_ag_011[0].message.contains("must be an object"));
    }

    #[test]
    fn test_cc_ag_011_no_hooks_ok() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-011")
            .collect();
        assert_eq!(cc_ag_011.len(), 0);
    }

    #[test]
    fn test_cc_ag_011_event_value_not_array() {
        let content = r#"---
name: my-agent
description: A test agent
hooks:
  PreToolUse: "invalid-string"
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-011")
            .collect();
        assert_eq!(cc_ag_011.len(), 1);
        assert!(cc_ag_011[0].message.contains("must map to an array"));
    }

    #[test]
    fn test_cc_ag_011_matcher_missing_hooks_array() {
        let content = r#"---
name: my-agent
description: A test agent
hooks:
  PreToolUse:
    - matcher: "*"
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-011")
            .collect();
        assert_eq!(cc_ag_011.len(), 1);
        assert!(
            cc_ag_011[0]
                .message
                .contains("missing required 'hooks' array")
        );
    }

    #[test]
    fn test_cc_ag_011_hook_missing_type() {
        let content = r#"---
name: my-agent
description: A test agent
hooks:
  PreToolUse:
    - matcher: "*"
      hooks:
        - command: echo hello
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-011")
            .collect();
        assert_eq!(cc_ag_011.len(), 1);
        assert!(
            cc_ag_011[0]
                .message
                .contains("missing required 'type' field")
        );
    }

    #[test]
    fn test_cc_ag_011_hook_invalid_type() {
        let content = r#"---
name: my-agent
description: A test agent
hooks:
  PreToolUse:
    - matcher: "*"
      hooks:
        - type: invalid
          command: echo hello
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-011")
            .collect();
        assert_eq!(cc_ag_011.len(), 1);
        assert!(cc_ag_011[0].message.contains("hook type 'invalid'"));
        assert!(
            cc_ag_011[0]
                .message
                .contains("must be 'command', 'http', 'mcp_tool', 'prompt', or 'agent'")
        );
    }

    #[test]
    fn test_cc_ag_011_matcher_not_object() {
        let content = r#"---
name: my-agent
description: A test agent
hooks:
  PreToolUse:
    - "just a string"
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-011")
            .collect();
        assert_eq!(cc_ag_011.len(), 1);
        assert!(cc_ag_011[0].message.contains("matcher"));
        assert!(cc_ag_011[0].message.contains("must be an object"));
    }

    #[test]
    fn test_cc_ag_011_hooks_field_not_array() {
        let content = r#"---
name: my-agent
description: A test agent
hooks:
  PreToolUse:
    - matcher: "*"
      hooks: "not-an-array"
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-011")
            .collect();
        assert_eq!(cc_ag_011.len(), 1);
        assert!(cc_ag_011[0].message.contains("'hooks' field"));
        assert!(cc_ag_011[0].message.contains("must be an array"));
    }

    // ===== CC-AG-012 Tests: bypassPermissions Warning =====

    #[test]
    fn test_cc_ag_012_bypass_permissions_warning() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: bypassPermissions
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-012")
            .collect();

        assert_eq!(cc_ag_012.len(), 1);
        assert_eq!(cc_ag_012[0].level, DiagnosticLevel::Warning);
        assert!(cc_ag_012[0].message.contains("bypassPermissions"));
    }

    #[test]
    fn test_cc_ag_012_other_modes_no_warning() {
        for mode in &["default", "acceptEdits", "dontAsk", "plan", "delegate"] {
            let content = format!(
                "---\nname: test\ndescription: Test agent\npermissionMode: {}\n---\nBody",
                mode
            );

            let diagnostics = validate(&content);
            let cc_ag_012: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-AG-012")
                .collect();
            assert_eq!(
                cc_ag_012.len(),
                0,
                "Mode '{}' should not trigger CC-AG-012",
                mode
            );
        }
    }

    #[test]
    fn test_cc_ag_012_no_permission_mode_no_warning() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-012")
            .collect();
        assert_eq!(cc_ag_012.len(), 0);
    }

    // ===== CC-AG-013 Tests: Skill Name Format =====

    // ===== CC-AG-012 auto-fix tests =====

    #[test]
    fn test_cc_ag_012_has_no_autofix() {
        let content = "---\nname: my-agent\ndescription: A test agent\npermissionMode: bypassPermissions\n---\nAgent instructions";
        let diagnostics = validate(content);
        let cc_ag_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-012")
            .collect();
        assert_eq!(cc_ag_012.len(), 1);
        assert!(
            !cc_ag_012[0].has_fixes(),
            "Safety advice must not rewrite permissionMode"
        );
        assert_eq!(cc_ag_012[0].level, DiagnosticLevel::Warning);
    }

    // ===== Fixture Tests for New Rules =====

    #[test]
    fn test_fixture_invalid_memory() {
        let content = include_str!("../../../../tests/fixtures/invalid/agents/invalid-memory.md");
        let diagnostics = validate(content);
        let cc_ag_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-008")
            .collect();
        assert!(!cc_ag_008.is_empty());
    }

    #[test]
    fn test_fixture_invalid_tool_name() {
        let content =
            include_str!("../../../../tests/fixtures/invalid/agents/invalid-tool-name.md");
        let diagnostics = validate(content);
        let cc_ag_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-009")
            .collect();
        assert!(!cc_ag_009.is_empty());
    }

    #[test]
    fn test_fixture_invalid_disallowed_tool() {
        let content =
            include_str!("../../../../tests/fixtures/invalid/agents/invalid-disallowed-tool.md");
        let diagnostics = validate(content);
        let cc_ag_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-010")
            .collect();
        assert!(!cc_ag_010.is_empty());
    }

    #[test]
    fn test_fixture_invalid_hooks() {
        let content = include_str!("../../../../tests/fixtures/invalid/agents/invalid-hooks.md");
        let diagnostics = validate(content);
        let cc_ag_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-011")
            .collect();
        assert!(!cc_ag_011.is_empty());
    }

    #[test]
    fn test_fixture_bypass_permissions() {
        let content =
            include_str!("../../../../tests/fixtures/invalid/agents/bypass-permissions.md");
        let diagnostics = validate(content);
        let cc_ag_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-012")
            .collect();
        assert!(!cc_ag_012.is_empty());
        assert_eq!(cc_ag_012[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_fixture_invalid_skill_format() {
        let content =
            include_str!("../../../../tests/fixtures/invalid/agents/invalid-skill-format.md");
        let diagnostics = validate(content);
        // CC-AG-013 is retired: the sub-agents reference states no naming
        // format for the `skills` list.
        assert!(diagnostics.iter().all(|d| d.rule != "CC-AG-013"));
    }

    #[test]
    fn test_fixture_valid_agent_with_new_fields() {
        let content =
            include_str!("../../../../tests/fixtures/valid/agents/agent-with-new-fields.md");
        let diagnostics = validate(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "Valid agent fixture should have no errors, got: {:?}",
            errors
        );
    }

    // ===== is_valid_skill_name_format unit tests =====

    // ===== CC-AG-007 parse error suggestion test =====

    #[test]
    fn test_cc_ag_007_parse_error_has_suggestion() {
        let content = "---\n invalid: [yaml\n---\ncontent";

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert_eq!(parse_errors.len(), 1);
        assert!(
            parse_errors[0].suggestion.is_some(),
            "CC-AG-007 parse error should have a suggestion"
        );
        assert!(
            parse_errors[0]
                .suggestion
                .as_ref()
                .unwrap()
                .contains("YAML frontmatter syntax"),
            "Suggestion should mention YAML frontmatter syntax"
        );
    }

    // ===== Integration Test: Metadata Auto-Population =====

    #[test]
    fn test_validator_produces_diagnostics_with_metadata() {
        // Integration test to verify that AgentValidator produces diagnostics
        // with metadata fields auto-populated from agnix-rules.
        let content = r#"---
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);

        // Should trigger CC-AG-001 (missing name)
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();

        assert_eq!(
            cc_ag_001.len(),
            1,
            "Should produce exactly one CC-AG-001 diagnostic"
        );

        let diag = &cc_ag_001[0];

        // Verify metadata is auto-populated
        assert!(
            diag.metadata.is_some(),
            "Diagnostic should have metadata auto-populated"
        );

        let meta = diag.metadata.as_ref().unwrap();
        assert_eq!(
            meta.category, "claude-agents",
            "CC-AG-001 should have category 'claude-agents'"
        );
        assert_eq!(
            meta.severity, "HIGH",
            "CC-AG-001 should have severity 'HIGH'"
        );
        assert_eq!(
            meta.applies_to_tool,
            Some("claude-code".to_string()),
            "CC-AG-001 should apply to 'claude-code'"
        );
    }

    // ===== CC-AG-003: Full model IDs (claude-* pattern) =====

    #[test]
    fn test_cc_ag_020_rejects_reserved_colon_in_name() {
        let content = "---\nname: plugin:reviewer\ndescription: Reviews code\n---\nReview changes.";
        let diagnostics = validate(content);
        assert_eq!(
            diagnostics.iter().filter(|d| d.rule == "CC-AG-020").count(),
            1
        );
    }

    #[test]
    fn test_cc_ag_020_allows_local_agent_name() {
        let content = "---\nname: reviewer\ndescription: Reviews code\n---\nReview changes.";
        assert!(validate(content).iter().all(|d| d.rule != "CC-AG-020"));
    }

    #[test]
    fn test_cc_ag_003_valid_full_model_id_opus() {
        let content = "---
name: a
description: b
model: claude-opus-4-6
---
Body";
        let d = validate(content);
        assert_eq!(d.iter().filter(|x| x.rule == "CC-AG-003").count(), 0);
    }

    #[test]
    fn test_cc_ag_003_valid_full_model_id_sonnet() {
        let content = "---
name: a
description: b
model: claude-sonnet-4-6
---
Body";
        let d = validate(content);
        assert_eq!(d.iter().filter(|x| x.rule == "CC-AG-003").count(), 0);
    }

    #[test]
    fn test_cc_ag_003_valid_full_model_id_with_date() {
        let content = "---
name: a
description: b
model: claude-haiku-4-5-20251001
---
Body";
        let d = validate(content);
        assert_eq!(d.iter().filter(|x| x.rule == "CC-AG-003").count(), 0);
    }

    #[test]
    fn test_cc_ag_003_rejects_non_claude_prefix() {
        let content = "---
name: a
description: b
model: gpt-4o
---
Body";
        let d = validate(content);
        assert_eq!(d.iter().filter(|x| x.rule == "CC-AG-003").count(), 1);
    }

    #[test]
    fn test_is_valid_model_short_aliases() {
        assert!(is_valid_model("sonnet"));
        assert!(is_valid_model("opus"));
        assert!(is_valid_model("haiku"));
        assert!(is_valid_model("inherit"));
    }

    #[test]
    fn test_is_valid_model_full_ids() {
        assert!(is_valid_model("claude-opus-4-6"));
        assert!(is_valid_model("claude-sonnet-4-6"));
        assert!(is_valid_model("claude-haiku-4-5-20251001"));
    }

    #[test]
    fn test_is_valid_model_invalid() {
        assert!(!is_valid_model("gpt-4"));
        assert!(!is_valid_model("gemini-pro"));
        assert!(!is_valid_model(""));
    }

    // ===== New fields: no false positives =====

    #[test]
    fn test_new_fields_no_parse_error() {
        let c = "---
name: a
description: b
maxTurns: 5
effort: high
background: true
omitClaudeMd: true
autoCompactWindow: 200000
isolation: worktree
initialPrompt: hi
mcpServers:
  m:
    command: x
---
Body";
        let d = validate(c);
        let parse_errors: Vec<_> = d.iter().filter(|x| x.rule == "CC-AG-007").collect();
        assert!(
            parse_errors.is_empty(),
            "New fields should not trigger CC-AG-007 parse errors: {:?}",
            parse_errors
        );
    }

    #[test]
    fn test_max_turns_accepts_positive_integer() {
        let c = "---
name: a
description: b
maxTurns: 10
---
Body";
        let d = validate(c);
        assert_eq!(
            d.iter().filter(|x| x.rule == "CC-AG-017").count(),
            0,
            "Valid maxTurns should not trigger CC-AG-017"
        );
    }

    #[test]
    fn test_max_turns_rejects_string() {
        let c = "---
name: a
description: b
maxTurns: bad
---
Body";
        let d = validate(c);
        assert_eq!(d.iter().filter(|x| x.rule == "CC-AG-007").count(), 1);
    }

    #[test]
    fn test_monitor_tool_accepted() {
        // tools: list (CC-AG-009)
        let c = "---
name: a
description: b
tools: [Monitor]
---
Body";
        let d = validate(c);
        assert_eq!(
            d.iter().filter(|x| x.rule == "CC-AG-009").count(),
            0,
            "Monitor tool should not trigger CC-AG-009 (added in Claude Code v2.1.98)"
        );

        // disallowedTools list (CC-AG-010) - same KNOWN_AGENT_TOOLS list
        let c2 = "---
name: a
description: b
disallowedTools: [Monitor]
---
Body";
        let d2 = validate(c2);
        assert_eq!(
            d2.iter().filter(|x| x.rule == "CC-AG-010").count(),
            0,
            "Monitor tool should not trigger CC-AG-010 in disallowedTools"
        );
    }

    #[test]
    fn test_color_field_accepted() {
        let c = "---
name: a
description: b
color: blue
---
Body";
        let d = validate(c);
        assert_eq!(
            d.iter().filter(|x| x.rule == "CC-AG-019").count(),
            0,
            "color field should not trigger CC-AG-019 unknown-field"
        );
    }

    #[test]
    fn test_background_accepts_bool() {
        let c = "---
name: a
description: b
background: false
---
Body";
        let d = validate(c);
        assert_eq!(
            d.iter().filter(|x| x.rule == "CC-AG-007").count(),
            0,
            "Valid background bool should not trigger CC-AG-007 parse error"
        );
    }

    #[test]
    fn test_background_rejects_string() {
        let c = "---
name: a
description: b
background: yes-please
---
Body";
        let d = validate(c);
        assert_eq!(d.iter().filter(|x| x.rule == "CC-AG-007").count(), 1);
    }

    #[test]
    fn test_isolation_accepts_worktree() {
        let c = "---
name: a
description: b
isolation: worktree
---
Body";
        let d = validate(c);
        assert_eq!(
            d.iter().filter(|x| x.rule == "CC-AG-015").count(),
            0,
            "Valid isolation 'worktree' should not trigger CC-AG-015"
        );
    }

    #[test]
    fn test_initial_prompt_accepts_string() {
        let c = "---
name: a
description: b
initialPrompt: Start here
---
Body";
        let d = validate(c);
        assert_eq!(
            d.iter().filter(|x| x.rule == "CC-AG-007").count(),
            0,
            "Valid initialPrompt string should not trigger CC-AG-007 parse error"
        );
    }

    #[test]
    fn test_mcp_servers_accepts_object() {
        let c = "---
name: a
description: b
mcpServers:
  m:
    command: x
---
Body";
        let d = validate(c);
        assert_eq!(
            d.iter().filter(|x| x.rule == "CC-AG-007").count(),
            0,
            "Valid mcpServers object should not trigger CC-AG-007 parse error"
        );
    }

    // ===== CC-AG-014 Tests: Invalid Effort Value =====

    #[test]
    fn test_cc_ag_014_invalid_effort() {
        let content = r#"---
name: my-agent
description: A test agent
effort: turbo
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_014: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-014")
            .collect();

        assert_eq!(cc_ag_014.len(), 1);
        assert_eq!(cc_ag_014[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_014[0].message.contains("turbo"));
        assert!(cc_ag_014[0].message.contains("Invalid effort"));
    }

    #[test]
    fn test_cc_ag_014_all_valid_effort_values() {
        for effort in VALID_EFFORT_LEVELS {
            let content = format!(
                "---\nname: test\ndescription: Test agent\neffort: {}\n---\nBody",
                effort
            );

            let diagnostics = validate(&content);
            let cc_ag_014: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-AG-014")
                .collect();
            assert!(
                cc_ag_014.is_empty(),
                "Effort value '{}' should be valid",
                effort
            );
        }
    }

    #[test]
    fn test_cc_ag_014_no_effort_ok() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_014: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-014")
            .collect();
        assert_eq!(cc_ag_014.len(), 0);
    }

    #[test]
    fn test_cc_ag_014_autofix_close_match() {
        let content =
            "---\nname: my-agent\ndescription: A test agent\neffort: hig\n---\nAgent instructions";
        let diagnostics = validate(content);
        let cc_ag_014: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-014")
            .collect();
        assert_eq!(cc_ag_014.len(), 1);
        assert!(
            cc_ag_014[0].has_fixes(),
            "CC-AG-014 should have auto-fix for close match"
        );
        let fix = &cc_ag_014[0].fixes[0];
        assert_eq!(fix.replacement, "high", "Fix should suggest 'high'");
    }

    // ===== CC-AG-015 Tests: Invalid Isolation Value =====

    #[test]
    fn test_cc_ag_015_invalid_isolation() {
        let content = r#"---
name: my-agent
description: A test agent
isolation: sandbox
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_015: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-015")
            .collect();

        assert_eq!(cc_ag_015.len(), 1);
        assert_eq!(cc_ag_015[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_015[0].message.contains("sandbox"));
        assert!(cc_ag_015[0].message.contains("Invalid isolation"));
    }

    #[test]
    fn test_cc_ag_015_valid_worktree() {
        let content = r#"---
name: my-agent
description: A test agent
isolation: worktree
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_015: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-015")
            .collect();
        assert_eq!(cc_ag_015.len(), 0);
    }

    #[test]
    fn test_cc_ag_015_no_isolation_ok() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_015: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-015")
            .collect();
        assert_eq!(cc_ag_015.len(), 0);
    }

    #[test]
    fn test_cc_ag_015_has_autofix() {
        let content = "---\nname: my-agent\ndescription: A test agent\nisolation: docker\n---\nAgent instructions";
        let diagnostics = validate(content);
        let cc_ag_015: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-015")
            .collect();
        assert_eq!(cc_ag_015.len(), 1);
        assert!(cc_ag_015[0].has_fixes(), "CC-AG-015 should have auto-fix");
        let fix = &cc_ag_015[0].fixes[0];
        assert_eq!(fix.replacement, "worktree");
        assert!(!fix.safe, "CC-AG-015 fix should be unsafe");
    }

    // ===== CC-AG-016 Tests: Invalid Background Type =====

    #[test]
    fn test_cc_ag_016_serde_catches_non_bool() {
        // background is Option<bool> in AgentSchema, so serde rejects non-boolean
        // values at parse time, resulting in CC-AG-007 (parse error).
        let content = "---\nname: a\ndescription: b\nbackground: yes-please\n---\nBody";
        let diagnostics = validate(content);
        let cc_ag_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();
        assert_eq!(
            cc_ag_007.len(),
            1,
            "Non-boolean background should trigger CC-AG-007 parse error"
        );
    }

    #[test]
    fn test_cc_ag_016_valid_booleans() {
        for val in &["true", "false"] {
            let content = format!(
                "---\nname: a\ndescription: b\nbackground: {}\n---\nBody",
                val
            );
            let diagnostics = validate(&content);
            let errors: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.level == DiagnosticLevel::Error)
                .collect();
            assert!(
                errors.is_empty(),
                "background: {} should not trigger errors",
                val
            );
        }
    }

    // ===== CC-AG-017 Tests: Invalid maxTurns Value =====

    #[test]
    fn test_cc_ag_017_zero_max_turns() {
        let content = r#"---
name: my-agent
description: A test agent
maxTurns: 0
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_017: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-017")
            .collect();

        assert_eq!(cc_ag_017.len(), 1);
        assert_eq!(cc_ag_017[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_017[0].message.contains("maxTurns"));
        assert!(cc_ag_017[0].message.contains("positive integer"));
    }

    #[test]
    fn test_cc_ag_017_valid_positive_max_turns() {
        let content = r#"---
name: my-agent
description: A test agent
maxTurns: 5
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_017: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-017")
            .collect();
        assert_eq!(cc_ag_017.len(), 0);
    }

    #[test]
    fn test_cc_ag_017_no_max_turns_ok() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_017: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-017")
            .collect();
        assert_eq!(cc_ag_017.len(), 0);
    }

    #[test]
    fn test_cc_ag_017_serde_catches_non_integer() {
        // maxTurns is Option<u32>, so serde rejects non-integer values
        let content = "---\nname: a\ndescription: b\nmaxTurns: bad\n---\nBody";
        let diagnostics = validate(content);
        let cc_ag_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();
        assert_eq!(
            cc_ag_007.len(),
            1,
            "Non-integer maxTurns should trigger CC-AG-007 parse error"
        );
    }

    // ===== CC-AG-019 Tests: Unknown Agent Frontmatter Field =====

    #[test]
    fn test_cc_ag_019_unknown_field() {
        let content = r#"---
name: my-agent
description: A test agent
unknownField: some value
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_019: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-019")
            .collect();

        assert_eq!(cc_ag_019.len(), 1);
        assert_eq!(cc_ag_019[0].level, DiagnosticLevel::Warning);
        assert!(cc_ag_019[0].message.contains("unknownField"));
        assert!(cc_ag_019[0].message.contains("Unknown"));
    }

    #[test]
    fn test_cc_ag_019_multiple_unknown_fields() {
        let content = r#"---
name: my-agent
description: A test agent
foo: bar
baz: qux
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_019: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-019")
            .collect();

        assert_eq!(cc_ag_019.len(), 2);
        let messages: Vec<&str> = cc_ag_019.iter().map(|d| d.message.as_str()).collect();
        assert!(messages.iter().any(|m| m.contains("foo")));
        assert!(messages.iter().any(|m| m.contains("baz")));
    }

    #[test]
    fn test_cc_ag_019_all_known_fields_no_warning() {
        let content = r#"---
name: my-agent
description: A test agent
model: sonnet
permissionMode: default
effort: high
maxTurns: 10
background: true
isolation: worktree
initialPrompt: hello
memory: user
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_019: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-019")
            .collect();
        assert_eq!(cc_ag_019.len(), 0);
    }

    #[test]
    fn test_cc_ag_019_no_warning_for_mode_field() {
        // 'mode' is a known field per the spec
        let content = r#"---
name: my-agent
description: A test agent
mode: plan
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_019: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-019")
            .collect();
        assert_eq!(cc_ag_019.len(), 0);
    }

    #[test]
    fn test_omit_claude_md_is_known_and_boolean() {
        let valid = "---\nname: my-agent\ndescription: A test agent\nomitClaudeMd: true\n---\nAgent instructions";
        let diagnostics = validate(valid);
        assert!(
            diagnostics
                .iter()
                .all(|d| d.rule != "CC-AG-019" && d.rule != "CC-AG-007")
        );

        let invalid = "---\nname: my-agent\ndescription: A test agent\nomitClaudeMd: perhaps\n---\nAgent instructions";
        let diagnostics = validate(invalid);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-AG-007"));
    }

    #[test]
    fn test_auto_compact_window_is_known_number() {
        let valid = r#"---
name: my-agent
description: A test agent
autoCompactWindow: 200000
---
Agent instructions"#;
        let diagnostics = validate(valid);
        assert!(
            diagnostics
                .iter()
                .all(|d| d.rule != "CC-AG-019" && d.rule != "CC-AG-007"),
            "numeric autoCompactWindow must be accepted, got {diagnostics:?}"
        );

        let invalid = r#"---
name: my-agent
description: A test agent
autoCompactWindow: earlier
---
Agent instructions"#;
        // Changed expectation: a non-number no longer fails the frontmatter
        // parse. The sub-agents reference says Claude Code reads `200k` as 200
        // and "The subagent still loads and compacts as if the field were
        // absent", so the value is reported as ignored (CC-AG-019).
        let diagnostics = validate(invalid);
        assert!(
            diagnostics.iter().all(|d| d.rule != "CC-AG-007"),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "CC-AG-019" && d.message.contains("autoCompactWindow")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn test_cc_ag_019_auto_compact_window_range_and_strings() {
        for (value, ignored) in [
            ("200000", false),
            ("100000", false),
            ("1000000", false),
            ("99999", true),
            ("1000001", true),
            ("200k", true),
            ("\"200000\"", true),
        ] {
            let content = format!(
                "---\nname: my-agent\ndescription: A test agent\ntools: Read\nautoCompactWindow: {value}\n---\nBody\n"
            );
            let diagnostics = validate(&content);
            let hits: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-AG-019")
                .collect();
            assert_eq!(!hits.is_empty(), ignored, "{value}: {diagnostics:?}");
            if ignored {
                assert_eq!(hits[0].line, 5);
                assert!(diagnostics.iter().all(|d| d.rule != "CC-AG-007"));
            }
        }
    }
}

#[cfg(test)]
mod tests_claude_ext {
    //! CC-AG-005 (preload limits), CC-AG-021..024, CC-PL-014 on plugin agent
    //! files, CC-HK checks on agent frontmatter hooks, and other tools'
    //! `agents` directories.
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;
    use std::fs;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, content: &str) -> std::path::PathBuf {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    fn check(path: &Path) -> Vec<Diagnostic> {
        let content = fs::read_to_string(path).unwrap();
        AgentValidator.validate(path, &content, &LintConfig::default())
    }

    fn rules(diagnostics: &[Diagnostic], id: &str) -> usize {
        diagnostics.iter().filter(|d| d.rule == id).count()
    }

    fn agent(name: &str, description: &str, extra: &str) -> String {
        format!("---\nname: {name}\ndescription: {description}\n{extra}---\nBody\n")
    }

    #[test]
    fn cc_ag_021_combined_description_budget() {
        let temp = TempDir::new().unwrap();
        let long = "d".repeat(31_000);
        let first = write(
            temp.path(),
            ".claude/agents/a.md",
            &agent("a", &long, "tools: Read\n"),
        );
        let second = write(
            temp.path(),
            ".claude/agents/sub/b.md",
            &agent("b", &long, "tools: Read\n"),
        );
        let diagnostics = check(&first);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-021")
            .collect();
        assert_eq!(hits.len(), 1, "{diagnostics:?}");
        assert_eq!(hits[0].level, DiagnosticLevel::Warning);
        assert!(hits[0].message.contains("15000"));
        // Only the first file in the tree reports the project total.
        assert_eq!(rules(&check(&second), "CC-AG-021"), 0);

        let small = TempDir::new().unwrap();
        let only = write(
            small.path(),
            ".claude/agents/a.md",
            &agent("a", "Reviews code", "tools: Read\n"),
        );
        assert_eq!(rules(&check(&only), "CC-AG-021"), 0);
    }

    #[test]
    fn cc_ag_023_duplicate_names_in_tree() {
        let temp = TempDir::new().unwrap();
        let a = write(
            temp.path(),
            ".claude/agents/reviewer.md",
            &agent("reviewer", "Reviews code", "tools: Read\n"),
        );
        let b = write(
            temp.path(),
            ".claude/agents/team/reviewer-v2.md",
            &agent("reviewer", "Reviews code too", "tools: Read\n"),
        );
        let c = write(
            temp.path(),
            ".claude/agents/other.md",
            &agent("other", "Other", "tools: Read\n"),
        );
        // A README without frontmatter is documentation, not an agent.
        write(
            temp.path(),
            ".claude/agents/README.md",
            "# Agents\nname: reviewer\n",
        );
        for path in [&a, &b] {
            let diagnostics = check(path);
            assert_eq!(rules(&diagnostics, "CC-AG-023"), 1, "{diagnostics:?}");
        }
        assert_eq!(rules(&check(&c), "CC-AG-023"), 0);
    }

    #[test]
    fn cc_ag_022_inherits_all_tools() {
        let temp = TempDir::new().unwrap();
        let open = write(temp.path(), ".claude/agents/a.md", &agent("a", "A", ""));
        let diagnostics = check(&open);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-022")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, DiagnosticLevel::Info);
        for extra in ["tools: Read, Grep\n", "disallowedTools: Write\n"] {
            let path = write(temp.path(), ".claude/agents/b.md", &agent("b", "B", extra));
            assert_eq!(rules(&check(&path), "CC-AG-022"), 0, "{extra}");
        }
    }

    #[test]
    fn cc_ag_024_specifiers_and_agent_type_lists() {
        let temp = TempDir::new().unwrap();
        let path = write(
            temp.path(),
            ".claude/agents/a.md",
            &agent(
                "a",
                "A",
                "tools: Agent(worker, researcher), Read, Bash\ndisallowedTools: Bash(git push *), mcp__github\n",
            ),
        );
        let diagnostics = check(&path);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-024")
            .collect();
        assert_eq!(hits.len(), 2, "{diagnostics:?}");
        assert!(
            hits.iter()
                .any(|d| d.level == DiagnosticLevel::Warning
                    && d.message.contains("Bash(git push *)"))
        );
        assert!(
            hits.iter().any(|d| d.level == DiagnosticLevel::Info
                && d.message.contains("Agent(worker, researcher)"))
        );
    }

    #[test]
    fn cc_ag_005_preload_limits() {
        let temp = TempDir::new().unwrap();
        write(
            temp.path(),
            ".claude/skills/deploy/SKILL.md",
            "---\nname: deploy\ndescription: Deploy\ndisable-model-invocation: true\n---\nDeploy.\n",
        );
        write(
            temp.path(),
            ".claude/skills/review/SKILL.md",
            "---\nname: review\ndescription: Review\n---\nReview.\n",
        );
        let path = write(
            temp.path(),
            ".claude/agents/a.md",
            &agent("a", "A", "tools: Read\nskills:\n  - deploy\n  - review\n"),
        );
        let diagnostics = check(&path);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();
        assert_eq!(hits.len(), 1, "{diagnostics:?}");
        assert!(hits[0].message.contains("deploy"));

        let many: String = (0..34).map(|i| format!("  - s{i}\n")).collect();
        let path = write(
            temp.path(),
            ".claude/agents/b.md",
            &agent("b", "B", &format!("tools: Read\nskills:\n{many}")),
        );
        let diagnostics = check(&path);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "CC-AG-005" && d.message.contains("first 32")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn cc_pl_014_plugin_agent_ignored_fields() {
        let temp = TempDir::new().unwrap();
        write(
            temp.path(),
            ".claude-plugin/plugin.json",
            r#"{"name": "p"}"#,
        );
        let path = write(
            temp.path(),
            "agents/reviewer.md",
            &agent(
                "reviewer",
                "Reviews",
                "tools: Read\npermissionMode: plan\ninitialPrompt: Start\nmcpServers:\n  - github\n",
            ),
        );
        let diagnostics = check(&path);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-014")
            .collect();
        assert_eq!(hits.len(), 3, "{diagnostics:?}");
        assert!(hits.iter().any(|d| d.message.contains("initialPrompt")));
        // The same fields in a project agent are fine.
        let local = write(
            temp.path(),
            ".claude/agents/reviewer.md",
            &agent(
                "reviewer",
                "Reviews",
                "tools: Read\npermissionMode: plan\ninitialPrompt: Start\n",
            ),
        );
        assert_eq!(rules(&check(&local), "CC-PL-014"), 0);
    }

    #[test]
    fn frontmatter_hooks_get_handler_checks() {
        let temp = TempDir::new().unwrap();
        let path = write(
            temp.path(),
            ".claude/agents/a.md",
            &agent(
                "a",
                "A",
                "tools: Read\nhooks:\n  PreToolUse:\n    - matcher: mcp__memory\n      hooks:\n        - type: command\n          command: ${CLAUDE_PROJECT_DIR}/check.sh\n",
            ),
        );
        let diagnostics = check(&path);
        assert_eq!(rules(&diagnostics, "CC-HK-033"), 1, "{diagnostics:?}");
        assert_eq!(rules(&diagnostics, "CC-HK-030"), 1, "{diagnostics:?}");
        let hook_line = diagnostics
            .iter()
            .find(|d| d.rule == "CC-HK-033")
            .unwrap()
            .line;
        assert_eq!(hook_line, 5, "reported at the hooks key");
    }

    #[test]
    fn other_vendor_agent_dirs_are_skipped() {
        let temp = TempDir::new().unwrap();
        let bad = "---\ndescription: OpenCode agent\nmodel: anthropic/claude-sonnet-4-5\ntemperature: 0.1\n---\nBody\n";
        for rel in [
            ".opencode/agents/review.md",
            ".github/agents/review.md",
            "home/.config/opencode/agents/review.md",
        ] {
            let path = write(temp.path(), rel, bad);
            assert!(check(&path).is_empty(), "{rel}: {:?}", check(&path));
        }
    }
}
