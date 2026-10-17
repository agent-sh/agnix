//! Permission rule checks on `permissions.allow`, `permissions.deny` and
//! `permissions.ask` (code.claude.com/docs/en/permissions, page dated
//! 2026-10-17; startup warnings in code.claude.com/docs/en/errors):
//!
//! - CC-SET-016: path rules on Write, NotebookEdit, MultiEdit or Glob, which
//!   "Claude Code accepts ... but never consults".
//! - CC-SET-035: rules Claude Code skips, ignores, or reads differently from
//!   how they are written.
//! - CC-SET-036: allow rules that approve arbitrary code execution.
//! - CC-SET-039: secret files in the project with no Read deny rule.

use super::scope::SettingsScope;
use crate::{
    diagnostics::{Diagnostic, Fix},
    fs::FileSystem,
    schemas::kiro_json::{JsonNode, LineIndex},
};
use rust_i18n::t;
use std::path::Path;

/// One string entry of a permissions list.
pub(super) struct RuleEntry<'a> {
    pub list: &'static str,
    pub text: &'a str,
    /// Byte span of the JSON string token, quotes included.
    pub start: usize,
    pub end: usize,
}

pub(super) fn rule_entries(root: &JsonNode) -> Vec<RuleEntry<'_>> {
    let mut entries = Vec::new();
    let Some(permissions) = root.value.get("permissions") else {
        return entries;
    };
    for list in ["allow", "deny", "ask"] {
        let Some(member) = permissions.value.value.get(list) else {
            continue;
        };
        let Some(items) = member.value.value.as_array() else {
            continue;
        };
        for item in items {
            if let Some(text) = item.value.as_str() {
                entries.push(RuleEntry {
                    list,
                    text,
                    start: item.start,
                    end: item.end,
                });
            }
        }
    }
    entries
}

/// A rule split into its tool name and the specifier inside the parentheses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ParsedRule<'a> {
    pub tool: &'a str,
    pub specifier: Option<&'a str>,
}

/// "Rules take the form Tool or Tool(content) and must end at the closing
/// ")"; parentheses inside the content are literal". `None` is a malformed
/// rule, which Claude Code skips.
pub(super) fn parse_rule(text: &str) -> Option<ParsedRule<'_>> {
    match text.find('(') {
        Some(open) => {
            let tool = &text[..open];
            if tool.is_empty() || !text.ends_with(')') {
                return None;
            }
            Some(ParsedRule {
                tool,
                specifier: Some(&text[open + 1..text.len() - 1]),
            })
        }
        None if text.contains(')') || text.is_empty() => None,
        None => Some(ParsedRule {
            tool: text,
            specifier: None,
        }),
    }
}

/// Built-in tools from the tools reference, plus names of removed or legacy
/// tools, which "are exempt" from Claude Code's unknown-tool warning.
const KNOWN_TOOLS: &[&str] = &[
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
    "SubagentHandback",
    "TaskCreate",
    "TaskGet",
    "TaskList",
    "TaskStop",
    "TaskUpdate",
    "TodoWrite",
    "ToolSearch",
    "WaitForMcpServers",
    "WebFetch",
    "WebSearch",
    "Workflow",
    "Write",
    // Removed or renamed tools ("TaskOutput", the legacy MultiEdit, and the
    // Task name the Agent tool replaced)
    "BashOutput",
    "KillShell",
    "LS",
    "MultiEdit",
    "NotebookRead",
    "Task",
    "TaskOutput",
    "TodoRead",
];

/// Tools whose path rules Claude Code never consults, with the replacement.
const UNCONSULTED_PATH_TOOLS: &[(&str, &str)] = &[
    ("Write", "Edit"),
    ("NotebookEdit", "Edit"),
    ("MultiEdit", "Edit"),
    ("Glob", "Read"),
];

/// "You can't match a tool's primary content field this way: `command` for
/// Bash and PowerShell, `file_path` for Read, Edit, and Write, `path` for Grep
/// and Glob, `notebook_path` for NotebookEdit, and `url` for WebFetch."
const PRIMARY_FIELDS: &[(&str, &str)] = &[
    ("Bash", "command"),
    ("PowerShell", "command"),
    ("Read", "file_path"),
    ("Edit", "file_path"),
    ("Write", "file_path"),
    ("Grep", "path"),
    ("Glob", "path"),
    ("NotebookEdit", "notebook_path"),
    ("WebFetch", "url"),
];

/// Programs whose second word is a subcommand. The startup warning targets an
/// allow rule "whose `*` comes before a later word that determines which
/// command it is"; agnix checks it only for these programs.
const SUBCOMMAND_PROGRAMS: &[&str] = &[
    "apt",
    "apt-get",
    "aws",
    "az",
    "brew",
    "bun",
    "cargo",
    "docker",
    "dotnet",
    "gcloud",
    "gh",
    "git",
    "go",
    "gradle",
    "helm",
    "kubectl",
    "mvn",
    "npm",
    "pip",
    "pip3",
    "pnpm",
    "podman",
    "poetry",
    "systemctl",
    "terraform",
    "uv",
    "yarn",
];

/// First path segments that mean an absolute path was written with one `/`
/// ("A pattern like `/Users/alice/file` isn't an absolute path").
const ABSOLUTE_ROOTS: &[&str] = &[
    "Users", "home", "etc", "tmp", "var", "opt", "private", "root", "usr", "Library", "Volumes",
    "mnt",
];

/// Interpreters: "Wildcarded interpreters like `Bash(python*)`".
const INTERPRETERS: &[&str] = &[
    "bash",
    "deno",
    "fish",
    "lua",
    "node",
    "nodejs",
    "osascript",
    "perl",
    "php",
    "powershell",
    "pwsh",
    "python",
    "python2",
    "python3",
    "ruby",
    "sh",
    "zsh",
];

/// Commands that run whatever follows them: environment runners ("a rule
/// like `Bash(devbox run *)` matches whatever comes after `run`") and package
/// runners that download and execute a package.
const RUNNERS: &[&str] = &[
    "bunx",
    "devbox run",
    "direnv exec",
    "docker exec",
    "docker run",
    "env",
    "eval",
    "exec",
    "mise exec",
    "mise x",
    "nix run",
    "npm exec",
    "npx",
    "pipx run",
    "pnpm dlx",
    "pnpm exec",
    "podman exec",
    "podman run",
    "sudo",
    "uv run",
    "uvx",
    "xargs",
    "yarn dlx",
    "yarn exec",
];

/// Package managers whose bare wildcard also covers their exec and dlx forms.
const PACKAGE_MANAGERS: &[&str] = &["bun", "npm", "pnpm", "yarn"];

/// "Package-manager run commands", which auto mode drops. The permissions page
/// uses `Bash(npm run *)` as an example, so these are reported as advice.
const SCRIPT_RUNNERS: &[&str] = &[
    "bun run",
    "cargo run",
    "go run",
    "npm run",
    "pnpm run",
    "poetry run",
    "yarn run",
];

fn shell_tool(tool: &str) -> bool {
    matches!(tool, "Bash" | "PowerShell")
}

fn diag_at(
    path: &Path,
    lines: &LineIndex<'_>,
    offset: usize,
    level: crate::diagnostics::DiagnosticLevel,
    rule: &str,
    message: String,
    suggestion: String,
) -> Diagnostic {
    let (line, column) = lines.line_col(offset);
    let diagnostic = match level {
        crate::diagnostics::DiagnosticLevel::Error => {
            Diagnostic::error(path.to_path_buf(), line, column, rule, message)
        }
        crate::diagnostics::DiagnosticLevel::Warning => {
            Diagnostic::warning(path.to_path_buf(), line, column, rule, message)
        }
        crate::diagnostics::DiagnosticLevel::Info => {
            Diagnostic::info(path.to_path_buf(), line, column, rule, message)
        }
    };
    diagnostic.with_suggestion(suggestion)
}

// ============================================================================
// CC-SET-016
// ============================================================================

pub(super) fn check_unconsulted_path_rules(
    path: &Path,
    content: &str,
    entries: &[RuleEntry<'_>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    use crate::diagnostics::DiagnosticLevel::{Error, Warning};
    let lines = LineIndex::new(content);
    for entry in entries {
        let Some(rule) = parse_rule(entry.text) else {
            continue;
        };
        let Some(specifier) = rule.specifier else {
            // "Claude Code doesn't warn about a tool-name rule with no path"
            continue;
        };
        if specifier.is_empty() {
            continue;
        }
        let Some((_, replacement_tool)) = UNCONSULTED_PATH_TOOLS
            .iter()
            .find(|(tool, _)| *tool == rule.tool)
        else {
            continue;
        };
        let replacement = format!("{replacement_tool}({specifier})");
        // A deny or ask rule that is never consulted protects nothing.
        let level = if entry.list == "allow" {
            Warning
        } else {
            Error
        };
        let message = if entry.list == "allow" {
            t!(
                "rules.cc_set_016.message",
                rule = entry.text,
                list = entry.list,
                replacement = replacement.as_str()
            )
        } else {
            t!(
                "rules.cc_set_016.deny_message",
                rule = entry.text,
                list = entry.list,
                replacement = replacement.as_str()
            )
        };
        let diagnostic = diag_at(
            path,
            &lines,
            entry.start,
            level,
            "CC-SET-016",
            message.to_string(),
            t!(
                "rules.cc_set_016.suggestion",
                rule = entry.text,
                replacement = replacement.as_str()
            )
            .to_string(),
        )
        // The replacement makes a rule that was ignored start applying, so
        // it changes permissions: opt-in only.
        .with_fix(Fix::replace(
            entry.start,
            entry.end,
            serde_json::to_string(&replacement).expect("string serialization"),
            t!(
                "rules.cc_set_016.fix",
                rule = entry.text,
                replacement = replacement.as_str()
            ),
            false,
        ));
        diagnostics.push(diagnostic);
    }
}

// ============================================================================
// CC-SET-035
// ============================================================================

/// True when an allow rule's tool-name glob is anchored the way Claude Code
/// requires: "Allow rules accept tool-name globs only after a literal
/// `mcp__<server>__` prefix. The server segment must be glob-free".
fn anchored_allow_glob(tool: &str) -> bool {
    let Some(rest) = tool.strip_prefix("mcp__") else {
        return false;
    };
    let Some(separator) = rest.find("__") else {
        return false;
    };
    let server = &rest[..separator];
    !server.is_empty() && !server.contains('*')
}

/// The first `*` that comes before the subcommand of a known program.
fn wildcard_before_subcommand(command: &str) -> bool {
    let mut tokens = command.split_whitespace();
    let Some(program) = tokens.next() else {
        return false;
    };
    if !SUBCOMMAND_PROGRAMS.contains(&program) {
        return false;
    }
    let mut saw_wildcard = false;
    for token in tokens {
        if token.contains('*') {
            saw_wildcard = true;
            continue;
        }
        if token.starts_with('-') {
            continue;
        }
        // The first plain word after the program is the subcommand.
        return saw_wildcard;
    }
    false
}

/// `Read(/Users/alice/x)`: one leading slash before an absolute root.
fn single_slash_absolute(specifier: &str) -> bool {
    let Some(rest) = specifier.strip_prefix('/') else {
        return false;
    };
    if rest.starts_with('/') {
        return false;
    }
    let first = rest.split('/').next().unwrap_or_default();
    ABSOLUTE_ROOTS.contains(&first)
        || (first.len() == 1 && first.chars().all(|c| c.is_ascii_alphabetic()))
}

pub(super) fn check_ineffective_rules(
    path: &Path,
    content: &str,
    entries: &[RuleEntry<'_>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    use crate::diagnostics::DiagnosticLevel::{Error, Warning};
    let lines = LineIndex::new(content);
    let mut push = |entry: &RuleEntry<'_>, level, message: String, suggestion: String| {
        diagnostics.push(diag_at(
            path,
            &lines,
            entry.start,
            level,
            "CC-SET-035",
            message,
            suggestion,
        ));
    };

    for entry in entries {
        let Some(rule) = parse_rule(entry.text) else {
            push(
                entry,
                Error,
                t!("rules.cc_set_035.malformed_message", rule = entry.text).to_string(),
                t!("rules.cc_set_035.malformed_suggestion").to_string(),
            );
            continue;
        };

        if rule.tool.starts_with("mcp__") && rule.specifier.is_some() {
            push(
                entry,
                Error,
                t!(
                    "rules.cc_set_035.mcp_parentheses_message",
                    rule = entry.text
                )
                .to_string(),
                t!("rules.cc_set_035.mcp_parentheses_suggestion").to_string(),
            );
            continue;
        }

        if let Some(specifier) = rule.specifier
            && let Some((field, _)) = specifier.split_once(':')
            && let Some((_, primary)) = PRIMARY_FIELDS
                .iter()
                .find(|(tool, primary)| *tool == rule.tool && *primary == field.trim())
        {
            push(
                entry,
                Error,
                t!(
                    "rules.cc_set_035.primary_field_message",
                    rule = entry.text,
                    field = *primary
                )
                .to_string(),
                t!(
                    "rules.cc_set_035.primary_field_suggestion",
                    tool = rule.tool
                )
                .to_string(),
            );
            continue;
        }

        if entry.list == "allow" && rule.tool.contains('*') && !anchored_allow_glob(rule.tool) {
            push(
                entry,
                Error,
                t!(
                    "rules.cc_set_035.unanchored_glob_message",
                    rule = entry.text
                )
                .to_string(),
                t!("rules.cc_set_035.unanchored_glob_suggestion").to_string(),
            );
            continue;
        }

        if let Some(specifier) = rule.specifier
            && shell_tool(rule.tool)
        {
            if let Some(index) = specifier.find(":*")
                && index + 2 < specifier.len()
            {
                push(
                    entry,
                    Warning,
                    t!("rules.cc_set_035.colon_star_message", rule = entry.text).to_string(),
                    t!("rules.cc_set_035.colon_star_suggestion").to_string(),
                );
                continue;
            }
            if entry.list == "allow" && wildcard_before_subcommand(specifier) {
                push(
                    entry,
                    Warning,
                    t!(
                        "rules.cc_set_035.wildcard_subcommand_message",
                        rule = entry.text
                    )
                    .to_string(),
                    t!("rules.cc_set_035.wildcard_subcommand_suggestion").to_string(),
                );
                continue;
            }
        }

        if let Some(specifier) = rule.specifier
            && matches!(rule.tool, "Read" | "Edit")
            && single_slash_absolute(specifier)
        {
            push(
                entry,
                Warning,
                t!(
                    "rules.cc_set_035.single_slash_message",
                    rule = entry.text,
                    specifier = specifier
                )
                .to_string(),
                t!(
                    "rules.cc_set_035.single_slash_suggestion",
                    tool = rule.tool,
                    specifier = specifier
                )
                .to_string(),
            );
            continue;
        }

        // "A deny or ask rule whose tool name matches no known tool produces a
        // startup warning to catch typos. Tool names containing `_` or `*` are
        // exempt".
        if entry.list != "allow"
            && !rule.tool.contains('_')
            && !rule.tool.contains('*')
            && !KNOWN_TOOLS.contains(&rule.tool)
        {
            let suggestion = match KNOWN_TOOLS
                .iter()
                .find(|known| known.eq_ignore_ascii_case(rule.tool))
            {
                Some(known) => t!(
                    "rules.cc_set_035.unknown_tool_case_suggestion",
                    tool = *known
                )
                .to_string(),
                None => t!("rules.cc_set_035.unknown_tool_suggestion").to_string(),
            };
            push(
                entry,
                Warning,
                t!(
                    "rules.cc_set_035.unknown_tool_message",
                    rule = entry.text,
                    tool = rule.tool,
                    list = entry.list
                )
                .to_string(),
                suggestion,
            );
        }
    }
}

// ============================================================================
// CC-SET-036
// ============================================================================

/// Why an allow rule approves arbitrary code, and whether it is advice only.
fn broad_execution(rule: &ParsedRule<'_>) -> Option<(&'static str, bool)> {
    match (rule.tool, rule.specifier) {
        ("Bash" | "PowerShell", None) => return Some(("rules.cc_set_036.blanket_message", false)),
        ("Bash" | "PowerShell", Some(spec)) if spec.trim() == "*" => {
            return Some(("rules.cc_set_036.blanket_message", false));
        }
        ("Agent", None) => return Some(("rules.cc_set_036.agent_message", true)),
        ("Agent", Some(spec)) if spec.trim() == "*" => {
            return Some(("rules.cc_set_036.agent_message", true));
        }
        ("Monitor", None) => return Some(("rules.cc_set_036.monitor_message", true)),
        ("Monitor", Some(spec)) if spec.trim() == "*" => {
            return Some(("rules.cc_set_036.monitor_message", true));
        }
        _ => {}
    }
    if !shell_tool(rule.tool) {
        return None;
    }
    let command = rule.specifier?.trim();
    if !command.contains('*') {
        return None;
    }
    // `Bash(* --version)`: "the `*` stands in for the program, so any program
    // matches".
    if command.starts_with('*') {
        return Some(("rules.cc_set_036.program_wildcard_message", false));
    }
    let program = command.split_whitespace().next().unwrap_or_default();
    let program_name = program.trim_end_matches('*');
    if INTERPRETERS.contains(&program_name) {
        return Some(("rules.cc_set_036.interpreter_message", false));
    }
    let wildcard_after = |prefix: &str| {
        command
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.trim_start().starts_with('*'))
    };
    if RUNNERS.iter().any(|runner| wildcard_after(runner)) {
        return Some(("rules.cc_set_036.runner_message", false));
    }
    if PACKAGE_MANAGERS.iter().any(|pm| wildcard_after(pm)) {
        return Some(("rules.cc_set_036.runner_message", false));
    }
    if SCRIPT_RUNNERS.iter().any(|runner| wildcard_after(runner)) {
        return Some(("rules.cc_set_036.script_runner_message", true));
    }
    None
}

pub(super) fn check_broad_execution(
    path: &Path,
    content: &str,
    entries: &[RuleEntry<'_>],
    scope: SettingsScope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use crate::diagnostics::DiagnosticLevel::{Info, Warning};
    let lines = LineIndex::new(content);
    for entry in entries.iter().filter(|e| e.list == "allow") {
        let Some(rule) = parse_rule(entry.text) else {
            continue;
        };
        let Some((message_key, advice_only)) = broad_execution(&rule) else {
            continue;
        };
        // A personal user-settings rule affects only its owner; a project
        // rule applies to everyone who clones the repository.
        let level = if advice_only || scope == SettingsScope::User {
            Info
        } else {
            Warning
        };
        diagnostics.push(diag_at(
            path,
            &lines,
            entry.start,
            level,
            "CC-SET-036",
            t!(message_key, rule = entry.text).to_string(),
            t!("rules.cc_set_036.suggestion").to_string(),
        ));
    }
}

// ============================================================================
// CC-SET-039
// ============================================================================

/// Files in the project root that "hold API keys, secrets, or environment
/// values".
fn secret_files(project_dir: &Path, fs: &dyn FileSystem) -> Vec<String> {
    let Ok(entries) = fs.read_dir(project_dir) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .into_iter()
        .filter(|entry| !entry.metadata.is_symlink)
        .filter_map(|entry| {
            let name = entry.path.file_name()?.to_str()?.to_string();
            let secret = if entry.metadata.is_dir {
                name == "secrets"
            } else {
                let template = ["example", "sample", "template", "dist", "defaults"]
                    .iter()
                    .any(|t| name.ends_with(&format!(".{t}")));
                (name == ".env" || (name.starts_with(".env.") && !template))
                    || name.ends_with(".pem")
                    || name.ends_with(".key")
            };
            secret.then(|| {
                if entry.metadata.is_dir {
                    format!("{name}/")
                } else {
                    name
                }
            })
        })
        .collect();
    found.sort();
    found
}

/// Whether a deny pattern blocks reads of a top-level project file, with the
/// gitignore-style matching Read rules use.
fn pattern_covers(pattern: &str, file: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() || pattern.starts_with('!') {
        return false;
    }
    let pattern = pattern
        .trim_start_matches("./")
        .trim_start_matches('/')
        .trim_start_matches("**/");
    let file = file.trim_end_matches('/');
    let pattern = pattern.trim_end_matches("/**").trim_end_matches('/');
    pattern == file
        || glob::Pattern::new(pattern)
            .map(|glob| glob.matches(file))
            .unwrap_or(false)
}

fn deny_patterns(root: &JsonNode) -> Vec<String> {
    let mut patterns = Vec::new();
    for entry in rule_entries(root).into_iter().filter(|e| e.list == "deny") {
        match parse_rule(entry.text) {
            Some(ParsedRule {
                tool: "Read",
                specifier: None,
            }) => patterns.push("*".to_string()),
            Some(ParsedRule {
                tool: "Read",
                specifier: Some(spec),
            }) => patterns.push(spec.to_string()),
            _ => {}
        }
    }
    if let Some(sandbox) = root.value.get("sandbox")
        && let Some(filesystem) = sandbox.value.value.get("filesystem")
        && let Some(deny_read) = filesystem.value.value.get("denyRead")
        && let Some(items) = deny_read.value.value.as_array()
    {
        patterns.extend(
            items
                .iter()
                .filter_map(|i| i.value.as_str().map(String::from)),
        );
    }
    patterns
}

pub(super) fn check_secrets_not_denied(
    path: &Path,
    content: &str,
    root: &JsonNode,
    fs: &dyn FileSystem,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(claude_dir) = path.parent() else {
        return;
    };
    let Some(project_dir) = claude_dir.parent() else {
        return;
    };
    let secrets = secret_files(project_dir, fs);
    if secrets.is_empty() {
        return;
    }
    let mut patterns = deny_patterns(root);
    // Deny rules in the local file count too.
    let local = claude_dir.join("settings.local.json");
    if let Ok(text) = fs.read_to_string(&local)
        && let Ok(document) = crate::schemas::kiro_json::parse_json_document(&text, false)
    {
        patterns.extend(deny_patterns(&document.root));
    }
    let unprotected: Vec<&String> = secrets
        .iter()
        .filter(|file| !patterns.iter().any(|p| pattern_covers(p, file)))
        .collect();
    if unprotected.is_empty() {
        return;
    }
    let offset = root
        .value
        .get("permissions")
        .map(|member| member.key_start)
        .unwrap_or(root.start);
    let (line, column) = LineIndex::new(content).line_col(offset);
    let names = unprotected
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let first = unprotected[0].trim_end_matches('/');
    let example = if unprotected[0].ends_with('/') {
        format!("Read(./{first}/**)")
    } else {
        format!("Read(./{first})")
    };
    diagnostics.push(
        Diagnostic::info(
            path.to_path_buf(),
            line,
            column,
            "CC-SET-039",
            t!("rules.cc_set_039.message", files = names.as_str()),
        )
        .with_suggestion(t!(
            "rules.cc_set_039.suggestion",
            example = example.as_str()
        )),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rule_shapes() {
        assert_eq!(
            parse_rule("Bash"),
            Some(ParsedRule {
                tool: "Bash",
                specifier: None
            })
        );
        assert_eq!(
            parse_rule("Edit(./Finance (2024)/**)"),
            Some(ParsedRule {
                tool: "Edit",
                specifier: Some("./Finance (2024)/**")
            })
        );
        assert_eq!(parse_rule("Bash(ls) x"), None);
        assert_eq!(parse_rule("Bash(ls"), None);
        assert_eq!(parse_rule("(ls)"), None);
        assert_eq!(parse_rule("Bash ls)"), None);
    }

    #[test]
    fn anchored_allow_globs() {
        assert!(anchored_allow_glob("mcp__puppeteer__*"));
        assert!(anchored_allow_glob("mcp__github__get_*"));
        assert!(!anchored_allow_glob("mcp__*"));
        assert!(!anchored_allow_glob("mcp__git*__x"));
        assert!(!anchored_allow_glob("*"));
        assert!(!anchored_allow_glob("B*"));
    }

    #[test]
    fn wildcard_before_subcommand_cases() {
        assert!(wildcard_before_subcommand("git * main"));
        assert!(wildcard_before_subcommand("git -C * status *"));
        assert!(!wildcard_before_subcommand("git log * main"));
        assert!(!wildcard_before_subcommand("git status *"));
        assert!(!wildcard_before_subcommand("npm run *"));
        // Programs without subcommands are not judged.
        assert!(!wildcard_before_subcommand("rm * foo"));
    }

    #[test]
    fn single_slash_absolute_cases() {
        assert!(single_slash_absolute("/Users/alice/secrets/**"));
        assert!(single_slash_absolute("/home/me/.ssh/**"));
        assert!(single_slash_absolute("/c/Users/me/x"));
        assert!(!single_slash_absolute("//Users/alice/x"));
        assert!(!single_slash_absolute("/src/**"));
        assert!(!single_slash_absolute("./src/**"));
    }

    #[test]
    fn broad_execution_cases() {
        let check = |text: &str| broad_execution(&parse_rule(text).unwrap());
        assert_eq!(
            check("Bash"),
            Some(("rules.cc_set_036.blanket_message", false))
        );
        assert_eq!(
            check("Bash(*)"),
            Some(("rules.cc_set_036.blanket_message", false))
        );
        assert_eq!(
            check("Bash(python*)"),
            Some(("rules.cc_set_036.interpreter_message", false))
        );
        assert_eq!(
            check("Bash(node *)"),
            Some(("rules.cc_set_036.interpreter_message", false))
        );
        assert_eq!(
            check("Bash(devbox run *)"),
            Some(("rules.cc_set_036.runner_message", false))
        );
        assert_eq!(
            check("Bash(npx *)"),
            Some(("rules.cc_set_036.runner_message", false))
        );
        assert_eq!(
            check("Bash(npm *)"),
            Some(("rules.cc_set_036.runner_message", false))
        );
        assert_eq!(
            check("Bash(npm run *)"),
            Some(("rules.cc_set_036.script_runner_message", true))
        );
        assert_eq!(
            check("Bash(* --version)"),
            Some(("rules.cc_set_036.program_wildcard_message", false))
        );
        assert_eq!(
            check("Agent"),
            Some(("rules.cc_set_036.agent_message", true))
        );
        assert_eq!(check("Bash(npm run test)"), None);
        assert_eq!(check("Bash(devbox run npm test)"), None);
        assert_eq!(check("Bash(git status *)"), None);
        assert_eq!(check("Agent(Explore)"), None);
        assert_eq!(check("Read"), None);
    }

    #[test]
    fn deny_pattern_coverage() {
        assert!(pattern_covers("./.env", ".env"));
        assert!(pattern_covers("**/.env", ".env"));
        assert!(pattern_covers(".env*", ".env.local"));
        assert!(pattern_covers("./secrets/**", "secrets/"));
        assert!(pattern_covers("*", ".env"));
        assert!(!pattern_covers("./.env", ".env.local"));
        assert!(!pattern_covers("!.env", ".env"));
    }
}
