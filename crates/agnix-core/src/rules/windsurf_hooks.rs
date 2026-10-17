//! Cascade hooks files (WS-HK-001 to WS-HK-005, and WS-007 for the legacy
//! `.windsurf/hooks.json`), run by [`super::windsurf::WindsurfValidator`].
//!
//! Source: https://docs.devin.ai/desktop/cascade/hooks. A hooks file is a JSON
//! object whose `hooks` object maps each of the twelve hook events to an
//! array of hooks. Each hook takes `command` (macOS/Linux, run via `bash -c`),
//! `powershell` (Windows), `show_output` and `working_directory`: "At least one
//! of `command` or `powershell` must be specified." Workspace hooks live in
//! `.devin/hooks.json`; "the legacy `.windsurf/hooks.json` is used only when
//! `.devin/hooks.json` is absent or defines no hooks". The Devin Local agent
//! has its own lifecycle hooks (`.devin/hooks.v1.json`), which are not checked
//! here.

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    schemas::kiro_json::{
        JsonDialect, JsonMember, JsonNode, LineIndex, closest_name, parse_json_document,
    },
};
use rust_i18n::t;
use std::path::Path;

/// The twelve Cascade hook events ("Hook Events").
pub(crate) const HOOK_EVENTS: &[&str] = &[
    "pre_read_code",
    "post_read_code",
    "pre_write_code",
    "post_write_code",
    "pre_run_command",
    "post_run_command",
    "pre_mcp_tool_use",
    "post_mcp_tool_use",
    "pre_user_prompt",
    "post_cascade_response",
    "post_cascade_response_with_transcript",
    "post_setup_worktree",
];

/// Events for which "The `show_output` configuration option does not apply
/// to this hook."
const NO_SHOW_OUTPUT_EVENTS: &[&str] = &[
    "pre_user_prompt",
    "post_cascade_response",
    "post_cascade_response_with_transcript",
];

/// Where a hooks file sits, which decides the workspace-only checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// `.devin/hooks.json`
    Workspace,
    /// `.windsurf/hooks.json`
    LegacyWorkspace,
    /// User and system files
    Machine,
}

fn scope_of(path: &Path) -> Scope {
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    match parent {
        Some(name) if name.eq_ignore_ascii_case(".devin") => Scope::Workspace,
        Some(name) if name.eq_ignore_ascii_case(".windsurf") => Scope::LegacyWorkspace,
        _ => Scope::Machine,
    }
}

struct Sink<'a> {
    path: &'a Path,
    lines: LineIndex<'a>,
    out: Vec<Diagnostic>,
}

impl Sink<'_> {
    fn at(&self, offset: usize) -> (usize, usize) {
        self.lines.line_col(offset)
    }

    fn error(&mut self, offset: usize, rule: &str, message: String, suggestion: String) {
        let (line, column) = self.at(offset);
        self.out.push(
            Diagnostic::error(self.path.to_path_buf(), line, column, rule, message)
                .with_suggestion(suggestion),
        );
    }

    fn warning(&mut self, offset: usize, rule: &str, message: String, suggestion: String) {
        let (line, column) = self.at(offset);
        self.out.push(
            Diagnostic::warning(self.path.to_path_buf(), line, column, rule, message)
                .with_suggestion(suggestion),
        );
    }

    fn info(&mut self, offset: usize, rule: &str, message: String, suggestion: String) {
        let (line, column) = self.at(offset);
        self.out.push(
            Diagnostic::info(self.path.to_path_buf(), line, column, rule, message)
                .with_suggestion(suggestion),
        );
    }
}

/// True when a parsed hooks file has at least one event with a non-empty
/// hook array, which is what "defines hooks" means for the legacy fallback.
fn defines_hooks(root: &JsonNode) -> bool {
    root.value
        .get("hooks")
        .and_then(|hooks| hooks.value.value.as_object())
        .is_some_and(|events| {
            events.iter().any(|event| {
                event
                    .value
                    .value
                    .as_array()
                    .is_some_and(|items| !items.is_empty())
            })
        })
}

/// The `.devin/hooks.json` next to a legacy `.windsurf/hooks.json`, when it
/// parses and defines hooks.
fn preferred_file_defines_hooks(path: &Path, config: &PerFileLintConfig<'_>) -> bool {
    let Some(workspace) = path.parent().and_then(Path::parent) else {
        return false;
    };
    let sibling = workspace.join(".devin").join("hooks.json");
    let fs = config.fs();
    if !fs.is_file(&sibling) {
        return false;
    }
    fs.read_to_string(&sibling)
        .ok()
        .and_then(|text| parse_json_document(&text, true).ok())
        .is_some_and(|document| defines_hooks(&document.root))
}

pub(super) fn validate(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let scope = scope_of(path);
    let mut sink = Sink {
        path,
        lines: LineIndex::new(content),
        out: Vec::new(),
    };

    // WS-007: the legacy workspace location still works, as a fallback.
    if scope == Scope::LegacyWorkspace && config.is_rule_enabled("WS-007") {
        sink.info(
            0,
            "WS-007",
            t!(
                "rules.ws_007.message",
                legacy = ".windsurf/hooks.json",
                preferred = ".devin/hooks.json"
            )
            .into(),
            t!("rules.ws_007.suggestion", preferred = ".devin/hooks.json").into(),
        );
    }

    let document = match parse_json_document(content, true) {
        Ok(document) => document,
        Err(error) => {
            if config.is_rule_enabled("WS-HK-001") {
                let (line, column) = (error.line, error.column.max(1));
                sink.out.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        column,
                        "WS-HK-001",
                        t!("rules.ws_hk_001.parse", error = error.message),
                    )
                    .with_suggestion(t!("rules.ws_hk_001.parse_suggestion")),
                );
            }
            return sink.out;
        }
    };

    if config.is_rule_enabled("WS-HK-001")
        && let JsonDialect::Jsonc { offset } = document.dialect
    {
        sink.warning(
            offset,
            "WS-HK-001",
            t!("rules.ws_hk_001.jsonc").into(),
            t!("rules.ws_hk_001.jsonc_suggestion").into(),
        );
    }

    let root = &document.root;
    let Some(members) = root.value.as_object() else {
        if config.is_rule_enabled("WS-HK-001") {
            sink.error(
                root.start,
                "WS-HK-001",
                t!("rules.ws_hk_001.root", found = root.value.type_name()).into(),
                t!("rules.ws_hk_001.root_suggestion").into(),
            );
        }
        return sink.out;
    };

    if config.is_rule_enabled("WS-HK-001") {
        for member in members
            .iter()
            .filter(|member| HOOK_EVENTS.contains(&member.key.as_str()))
        {
            sink.warning(
                member.key_start,
                "WS-HK-001",
                t!(
                    "rules.ws_hk_001.top_level_event",
                    event = member.key.as_str()
                )
                .into(),
                t!("rules.ws_hk_001.top_level_event_suggestion").into(),
            );
        }
    }

    let Some(hooks) = root.value.get("hooks") else {
        return sink.out;
    };
    let Some(events) = hooks.value.value.as_object() else {
        if config.is_rule_enabled("WS-HK-001") {
            sink.error(
                hooks.value.start,
                "WS-HK-001",
                t!(
                    "rules.ws_hk_001.hooks_type",
                    found = hooks.value.value.type_name()
                )
                .into(),
                t!("rules.ws_hk_001.root_suggestion").into(),
            );
        }
        return sink.out;
    };

    for event in events {
        check_event(&mut sink, event, scope, config);
    }

    // WS-HK-003: Devin Desktop ignores the legacy file when `.devin/hooks.json`
    // defines hooks.
    if scope == Scope::LegacyWorkspace
        && config.is_rule_enabled("WS-HK-003")
        && defines_hooks(root)
        && preferred_file_defines_hooks(path, config)
    {
        sink.warning(
            hooks.key_start,
            "WS-HK-003",
            t!("rules.ws_hk_003.shadowed").into(),
            t!("rules.ws_hk_003.shadowed_suggestion").into(),
        );
    }

    sink.out
}

fn check_event(
    sink: &mut Sink<'_>,
    event: &JsonMember,
    scope: Scope,
    config: &PerFileLintConfig<'_>,
) {
    let name = event.key.as_str();
    let known = HOOK_EVENTS.contains(&name);
    if !known && config.is_rule_enabled("WS-HK-002") {
        let suggestion = match closest_name(name, HOOK_EVENTS) {
            Some(closest) => t!("rules.ws_hk_002.suggestion_closest", closest = closest),
            None => t!("rules.ws_hk_002.suggestion"),
        };
        sink.warning(
            event.key_start,
            "WS-HK-002",
            t!("rules.ws_hk_002.message", event = name).into(),
            suggestion.into(),
        );
    }

    let Some(items) = event.value.value.as_array() else {
        if config.is_rule_enabled("WS-HK-001") {
            sink.error(
                event.value.start,
                "WS-HK-001",
                t!(
                    "rules.ws_hk_001.event_type",
                    event = name,
                    found = event.value.value.type_name()
                )
                .into(),
                t!("rules.ws_hk_001.root_suggestion").into(),
            );
        }
        return;
    };

    for item in items {
        check_hook(sink, name, item, scope, config);
    }
}

fn check_hook(
    sink: &mut Sink<'_>,
    event: &str,
    item: &JsonNode,
    scope: Scope,
    config: &PerFileLintConfig<'_>,
) {
    let Some(_) = item.value.as_object() else {
        if config.is_rule_enabled("WS-HK-001") {
            sink.error(
                item.start,
                "WS-HK-001",
                t!("rules.ws_hk_001.entry_type", found = item.value.type_name()).into(),
                t!("rules.ws_hk_001.missing_command_suggestion").into(),
            );
        }
        return;
    };
    let field = |key: &str| item.value.get(key).filter(|m| !m.value.value.is_null());

    // Option types ("Configuration Options").
    if config.is_rule_enabled("WS-HK-001") {
        for (key, expected) in [
            ("command", "string"),
            ("powershell", "string"),
            ("working_directory", "string"),
            ("show_output", "boolean"),
        ] {
            if let Some(member) = field(key)
                && member.value.value.type_name() != expected
            {
                sink.error(
                    member.value.start,
                    "WS-HK-001",
                    t!(
                        "rules.ws_hk_001.field_type",
                        field = key,
                        expected = expected,
                        found = member.value.value.type_name()
                    )
                    .into(),
                    t!(
                        "rules.ws_hk_001.field_type_suggestion",
                        field = key,
                        expected = expected
                    )
                    .into(),
                );
            }
        }
    }

    let command = field("command");
    let powershell = field("powershell");
    if command.is_none() && powershell.is_none() {
        if config.is_rule_enabled("WS-HK-001") {
            sink.error(
                item.start,
                "WS-HK-001",
                t!("rules.ws_hk_001.missing_command").into(),
                t!("rules.ws_hk_001.missing_command_suggestion").into(),
            );
        }
        return;
    }

    if config.is_rule_enabled("WS-HK-003") {
        if command.is_none()
            && let Some(member) = powershell
        {
            sink.warning(
                member.key_start,
                "WS-HK-003",
                t!("rules.ws_hk_003.powershell_only").into(),
                t!("rules.ws_hk_003.powershell_only_suggestion").into(),
            );
        }
        if let Some(member) = field("working_directory")
            && let Some(dir) = member.value.value.as_str()
            && dir.trim_start().starts_with('~')
        {
            sink.warning(
                member.value.start,
                "WS-HK-003",
                t!("rules.ws_hk_003.tilde").into(),
                t!("rules.ws_hk_003.tilde_suggestion").into(),
            );
        }
        if NO_SHOW_OUTPUT_EVENTS.contains(&event)
            && let Some(member) = field("show_output")
        {
            sink.info(
                member.key_start,
                "WS-HK-003",
                t!("rules.ws_hk_003.show_output", event = event).into(),
                t!("rules.ws_hk_003.show_output_suggestion").into(),
            );
        }
    }

    // Workspace hooks are committed with the project.
    if scope == Scope::Machine {
        return;
    }
    for (member, windows) in [(command, false), (powershell, true)] {
        let Some(member) = member else { continue };
        let Some(text) = member.value.value.as_str() else {
            continue;
        };
        if config.is_rule_enabled("WS-HK-004")
            && let Some(script) = relative_script_path(text, windows)
        {
            sink.info(
                member.value.start,
                "WS-HK-004",
                t!("rules.ws_hk_004.message", path = script).into(),
                t!("rules.ws_hk_004.suggestion").into(),
            );
        }
        if config.is_rule_enabled("WS-HK-005") && command_has_secret(text) {
            sink.warning(
                member.value.start,
                "WS-HK-005",
                t!("rules.ws_hk_005.message", field = member.key.as_str()).into(),
                t!("rules.ws_hk_005.suggestion").into(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// WS-HK-004: relative script paths
// ---------------------------------------------------------------------------

/// Interpreters whose first non-flag argument is the script they run.
const INTERPRETERS: &[&str] = &[
    "python",
    "python2",
    "python3",
    "node",
    "bash",
    "sh",
    "zsh",
    "ruby",
    "perl",
    "php",
    "bun",
    "deno",
    "pwsh",
    "powershell",
];

fn unquote(token: &str) -> &str {
    let token = token.trim_matches(|c| c == '"' || c == '\'');
    token.trim_start_matches('&').trim()
}

fn is_env_assignment(token: &str) -> bool {
    token.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    })
}

/// A path that is neither absolute nor built from a variable: it contains a
/// separator and does not start with `/`, `\`, `~`, `$`, `%` or a drive
/// letter. URLs and npm scopes (`@scope/pkg`) are not paths.
fn is_relative_path(token: &str, windows: bool) -> bool {
    let has_separator = token.contains('/') || (windows && token.contains('\\'));
    let bytes = token.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    has_separator
        && !drive
        && !token.contains("://")
        && !token.contains('$')
        && !matches!(
            bytes.first(),
            Some(b'/' | b'\\' | b'~' | b'%' | b'@' | b'-')
        )
        && !token.contains(['>', '<', '|'])
}

/// The script a hook command runs, when it is given as a relative path: the
/// program itself (`./scripts/check.sh`), or the first non-flag argument of
/// an interpreter (`python3 hooks/check.py`). Inline code (`bash -c ...`) and
/// commands found on PATH are not reported.
fn relative_script_path(command: &str, windows: bool) -> Option<&str> {
    let mut tokens = command
        .split_whitespace()
        .map(unquote)
        .filter(|t| !t.is_empty())
        .skip_while(|t| is_env_assignment(t));
    let program = tokens.next()?;
    let base = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(program)
        .trim_end_matches(".exe")
        .to_ascii_lowercase();
    let script = if INTERPRETERS.contains(&base.as_str()) {
        let mut script = None;
        for token in tokens {
            let lower = token.to_ascii_lowercase();
            if matches!(
                lower.as_str(),
                "-c" | "-e" | "--eval" | "-command" | "-encodedcommand"
            ) {
                return None;
            }
            if token.starts_with('-') || lower == "run" {
                continue;
            }
            script = Some(token);
            break;
        }
        script?
    } else {
        program
    };
    is_relative_path(script, windows).then_some(script)
}

// ---------------------------------------------------------------------------
// WS-HK-005: literal secrets (same detector shape as KR-HK-010)
// ---------------------------------------------------------------------------

/// Token prefixes and the minimum length of the token body after them.
const TOKEN_PREFIXES: &[(&str, usize)] = &[
    ("sk-", 16),
    ("ghp_", 30),
    ("gho_", 30),
    ("github_pat_", 30),
    ("xoxb-", 10),
    ("xoxp-", 10),
    ("glpat-", 20),
];

/// `key=value` spellings, matched case-insensitively anywhere, so
/// `GITHUB_TOKEN=` and `--token=` both count.
const SECRET_ASSIGNMENT_KEYS: &[&str] = &[
    "api_key=",
    "apikey=",
    "api-key=",
    "token=",
    "password=",
    "secret=",
    "access_key=",
    "private_key=",
];

/// Flags whose next argument is a credential (`--token value`).
const SECRET_FLAGS: &[&str] = &[
    "--token",
    "--api-key",
    "--apikey",
    "--password",
    "--secret",
    "--auth-token",
    "--access-token",
];

fn is_token_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

/// A value that the shell resolves at run time rather than a literal.
fn is_reference(value: &str) -> bool {
    value.starts_with('$')
        || value.starts_with('%')
        || value.starts_with('<')
        || value.starts_with("{{")
}

fn literal_secret(value: &str) -> bool {
    let value: String = value
        .trim_start_matches(['"', '\''])
        .chars()
        .take_while(|ch| !ch.is_whitespace() && !matches!(ch, '"' | '\'' | '&' | ';' | '|'))
        .collect();
    !is_reference(&value) && super::seems_plaintext_secret(&value)
}

/// Literal credentials in a hook command: a token prefix that starts a word
/// and has a token-shaped body, `AKIA` access key IDs, and literal values of
/// `KEY=value`, `--token value` and `Bearer value`. References (`$VAR`,
/// `${VAR}`, `$(cmd)`, `%VAR%`) are not secrets.
fn command_has_secret(command: &str) -> bool {
    let bytes = command.as_bytes();
    let lower = command.to_ascii_lowercase();
    let word_start = |at: usize| at == 0 || !bytes[at - 1].is_ascii_alphanumeric();

    for (prefix, min_body) in TOKEN_PREFIXES {
        for (at, _) in lower.match_indices(prefix) {
            let body = bytes[at + prefix.len()..]
                .iter()
                .take_while(|b| is_token_char(**b))
                .count();
            if word_start(at) && body >= *min_body {
                return true;
            }
        }
    }
    for (at, _) in command.match_indices("AKIA") {
        let body = &bytes[at + 4..];
        if word_start(at)
            && body.len() >= 16
            && body[..16]
                .iter()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            && body.get(16).is_none_or(|b| !b.is_ascii_alphanumeric())
        {
            return true;
        }
    }
    for key in SECRET_ASSIGNMENT_KEYS {
        for (at, _) in lower.match_indices(key) {
            if literal_secret(&command[at + key.len()..]) {
                return true;
            }
        }
    }
    let tokens: Vec<&str> = command.split_whitespace().collect();
    for pair in tokens.windows(2) {
        if SECRET_FLAGS
            .iter()
            .any(|flag| pair[0].eq_ignore_ascii_case(flag))
            && literal_secret(pair[1])
        {
            return true;
        }
    }
    lower
        .match_indices("bearer ")
        .any(|(at, _)| literal_secret(&command[at + "bearer ".len()..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;
    use crate::fs::MockFileSystem;
    use crate::rules::Validator;
    use crate::rules::windsurf::WindsurfValidator;
    use std::sync::Arc;

    fn run(path: &str, content: &str) -> Vec<Diagnostic> {
        WindsurfValidator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn rules(diagnostics: &[Diagnostic]) -> Vec<(&str, DiagnosticLevel)> {
        diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.level))
            .collect()
    }

    const DOC_EXAMPLE: &str = r#"{
  "hooks": {
    "pre_read_code": [
      {
        "command": "python3 /path/to/your/script.py",
        "powershell": "python3 C:\\path\\to\\your\\script.py",
        "show_output": true
      }
    ],
    "post_write_code": [
      {
        "command": "python3 /path/to/another/script.py",
        "show_output": true
      }
    ]
  }
}"#;

    #[test]
    fn documented_examples_are_clean() {
        assert!(run(".devin/hooks.json", DOC_EXAMPLE).is_empty());
        let worktree = r#"{"hooks": {"post_setup_worktree": [{"command": "bash $ROOT_WORKSPACE_PATH/hooks/setup_worktree.sh", "show_output": true}]}}"#;
        assert!(run(".devin/hooks.json", worktree).is_empty());
        let every_event = format!(
            "{{\"hooks\": {{{}}}}}",
            HOOK_EVENTS
                .iter()
                .map(|e| format!("\"{e}\": [{{\"command\": \"/usr/local/bin/audit\"}}]"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        assert!(run(".devin/hooks.json", &every_event).is_empty());
        assert!(run(".devin/hooks.json", "{}").is_empty());
        assert!(run("/home/u/.codeium/windsurf/hooks.json", DOC_EXAMPLE).is_empty());
    }

    #[test]
    fn ws_hk_001_structure_errors() {
        let d = run(".devin/hooks.json", "{\"hooks\": ");
        assert_eq!(rules(&d), [("WS-HK-001", DiagnosticLevel::Error)]);
        assert!(d[0].message.contains("not valid JSON"), "{}", d[0].message);

        let d = run(".devin/hooks.json", "[]");
        assert_eq!(rules(&d), [("WS-HK-001", DiagnosticLevel::Error)]);

        let d = run(".devin/hooks.json", r#"{"hooks": []}"#);
        assert_eq!(rules(&d), [("WS-HK-001", DiagnosticLevel::Error)]);
        assert_eq!((d[0].line, d[0].column), (1, 11));

        let d = run(
            ".devin/hooks.json",
            "{\n  \"hooks\": {\n    \"pre_run_command\": {\"command\": \"/bin/true\"}\n  }\n}",
        );
        assert_eq!(rules(&d), [("WS-HK-001", DiagnosticLevel::Error)]);
        assert_eq!((d[0].line, d[0].column), (3, 24));

        let d = run(
            ".devin/hooks.json",
            r#"{"hooks": {"pre_run_command": ["/bin/true", {"show_output": true}]}}"#,
        );
        assert_eq!(
            rules(&d),
            [
                ("WS-HK-001", DiagnosticLevel::Error),
                ("WS-HK-001", DiagnosticLevel::Error)
            ]
        );
        assert!(d[1].message.contains("neither"), "{}", d[1].message);
    }

    #[test]
    fn ws_hk_001_option_types() {
        let d = run(
            ".devin/hooks.json",
            r#"{"hooks": {"post_write_code": [{"command": ["black"], "show_output": "yes", "working_directory": 3}]}}"#,
        );
        let messages: Vec<&str> = d.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(d.len(), 3, "{messages:?}");
        assert!(
            d.iter()
                .all(|d| d.rule == "WS-HK-001" && d.level == DiagnosticLevel::Error)
        );
        assert!(
            messages[0].contains("'command' must be a string"),
            "{messages:?}"
        );
        assert!(
            messages
                .iter()
                .any(|m| m.contains("'show_output' must be a boolean"))
        );
    }

    #[test]
    fn ws_hk_001_jsonc_is_a_warning_and_top_level_events() {
        let d = run(
            ".devin/hooks.json",
            "{\n  // audit\n  \"hooks\": {\"post_read_code\": [{\"command\": \"/bin/log\"}]}\n}",
        );
        assert_eq!(rules(&d), [("WS-HK-001", DiagnosticLevel::Warning)]);
        assert_eq!(d[0].line, 2);

        let d = run(
            ".devin/hooks.json",
            r#"{"pre_read_code": [{"command": "/bin/log"}]}"#,
        );
        assert_eq!(rules(&d), [("WS-HK-001", DiagnosticLevel::Warning)]);
        assert!(d[0].message.contains("top level"), "{}", d[0].message);
    }

    #[test]
    fn ws_hk_002_unknown_events() {
        let d = run(
            ".devin/hooks.json",
            r#"{"hooks": {"pre_read_file": [{"command": "/bin/log"}], "PreToolUse": [{"command": "/bin/log"}]}}"#,
        );
        assert_eq!(
            rules(&d),
            [
                ("WS-HK-002", DiagnosticLevel::Warning),
                ("WS-HK-002", DiagnosticLevel::Warning)
            ]
        );
        assert!(
            d[0].suggestion
                .as_deref()
                .unwrap()
                .contains("pre_read_code"),
            "{:?}",
            d[0].suggestion
        );
        assert_eq!((d[0].line, d[0].column), (1, 12));
    }

    #[test]
    fn ws_hk_003_platform_tilde_show_output() {
        let d = run(
            ".devin/hooks.json",
            r#"{"hooks": {"pre_run_command": [{"powershell": "C:\\hooks\\check.ps1"}], "post_run_command": [{"command": "/bin/log", "working_directory": "~/project"}], "post_cascade_response": [{"command": "/bin/log", "show_output": true}]}}"#,
        );
        assert_eq!(
            rules(&d),
            [
                ("WS-HK-003", DiagnosticLevel::Warning),
                ("WS-HK-003", DiagnosticLevel::Warning),
                ("WS-HK-003", DiagnosticLevel::Info)
            ]
        );
        assert!(d[0].message.contains("macOS and Linux"), "{}", d[0].message);
        assert!(d[1].message.contains('~'), "{}", d[1].message);
        assert!(
            d[2].message.contains("post_cascade_response"),
            "{}",
            d[2].message
        );
    }

    #[test]
    fn ws_hk_003_legacy_file_shadowed_by_devin_file() {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file(
            "/p/.devin/hooks.json",
            r#"{"hooks": {"post_write_code": [{"command": "/bin/fmt"}]}}"#,
        );
        let mut config = LintConfig::default();
        config.set_fs(fs.clone());
        let legacy = r#"{"hooks": {"post_read_code": [{"command": "/bin/log"}]}}"#;
        let d = WindsurfValidator.validate(Path::new("/p/.windsurf/hooks.json"), legacy, &config);
        assert_eq!(
            rules(&d),
            [
                ("WS-007", DiagnosticLevel::Info),
                ("WS-HK-003", DiagnosticLevel::Warning)
            ]
        );
        assert!(d[1].message.contains("ignored"), "{}", d[1].message);

        // An empty preferred file does not shadow the legacy one.
        fs.add_file("/p/.devin/hooks.json", r#"{"hooks": {}}"#);
        let d = WindsurfValidator.validate(Path::new("/p/.windsurf/hooks.json"), legacy, &config);
        assert_eq!(rules(&d), [("WS-007", DiagnosticLevel::Info)]);
    }

    #[test]
    fn ws_hk_004_relative_script_paths() {
        for (command, expected) in [
            ("python3 hooks/check.py", Some("hooks/check.py")),
            ("./scripts/format.sh", Some("./scripts/format.sh")),
            ("FOO=1 bash scripts/x.sh --fast", Some("scripts/x.sh")),
            ("deno run tools/hook.ts", Some("tools/hook.ts")),
            ("python3 /Users/me/hooks/check.py", None),
            ("bash $ROOT_WORKSPACE_PATH/hooks/setup_worktree.sh", None),
            ("~/bin/hook", None),
            ("black .", None),
            ("npx @biomejs/biome check", None),
            ("bash -c 'cd hooks && ./run.sh'", None),
            ("curl https://example.com/hook", None),
            ("jq -r .tool_info.file_path", None),
        ] {
            assert_eq!(relative_script_path(command, false), expected, "{command}");
        }
        assert_eq!(
            relative_script_path(r"powershell -File .\hooks\check.ps1", true),
            Some(r".\hooks\check.ps1")
        );
        assert_eq!(
            relative_script_path(r"python3 C:\hooks\check.py", true),
            None
        );

        let content = r#"{"hooks": {"post_write_code": [{"command": "python3 hooks/fmt.py"}]}}"#;
        let d = run(".devin/hooks.json", content);
        assert_eq!(rules(&d), [("WS-HK-004", DiagnosticLevel::Info)]);
        assert!(d[0].message.contains("hooks/fmt.py"), "{}", d[0].message);
        // User and system hooks are not workspace hooks.
        assert!(run("/home/u/.codeium/windsurf/hooks.json", content).is_empty());
    }

    #[test]
    fn ws_hk_005_literal_secrets() {
        for command in [
            "curl -H 'Authorization: Bearer sk-live-4f9a8b7c6d5e4f3a2b1c' https://x",
            "GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz0123456789 /bin/report",
            "/bin/notify --token s3cr3tvalue99",
            "/bin/upload --api-key=abcd1234efgh5678",
            "AWS_ACCESS_KEY_ID=AKIAABCDEFGHIJKLMNOP /bin/sync",
        ] {
            assert!(command_has_secret(command), "{command}");
        }
        for command in [
            "/bin/notify --token $NOTIFY_TOKEN",
            "curl -H \"Authorization: Bearer ${API_TOKEN}\" https://x",
            "flask-lint --fix",
            "task-check",
            "/bin/notify --token=%TOKEN%",
        ] {
            assert!(!command_has_secret(command), "{command}");
        }
        let content = r#"{"hooks": {"post_cascade_response": [{"command": "/usr/bin/curl -H 'Authorization: Bearer sk-live-4f9a8b7c6d5e4f3a2b1c' https://audit"}]}}"#;
        let d = run(".devin/hooks.json", content);
        assert_eq!(rules(&d), [("WS-HK-005", DiagnosticLevel::Warning)]);
        assert!(run("/etc/devin/hooks.json", content).is_empty());
    }

    #[test]
    fn ws_007_on_legacy_workspace_hooks_only() {
        let d = run(".windsurf/hooks.json", DOC_EXAMPLE);
        assert_eq!(rules(&d), [("WS-007", DiagnosticLevel::Info)]);
        assert!(
            d[0].message.contains(".devin/hooks.json"),
            "{}",
            d[0].message
        );
        assert!(run("/etc/windsurf/hooks.json", DOC_EXAMPLE).is_empty());
    }

    #[test]
    fn rules_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["WS-HK-001".to_string(), "WS-HK-002".to_string()];
        let d = WindsurfValidator.validate(
            Path::new(".devin/hooks.json"),
            r#"{"hooks": {"nope": 1}}"#,
            &config,
        );
        assert!(d.is_empty(), "{d:?}");
        config.rules_mut().windsurf = false;
        let d = WindsurfValidator.validate(Path::new(".devin/hooks.json"), "{", &config);
        assert!(d.is_empty());
    }
}
