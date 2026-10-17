//! Kiro V3 standalone hook file schema helpers.
//!
//! Covers `.kiro/hooks/*.json` documents read by the Kiro CLI V3 engine and
//! Kiro IDE 1.0: `{ "version": "v1", "hooks": [ { name, trigger, matcher?,
//! action, timeout?, enabled?, description?, confirm? } ] }`.
//!
//! The tables below mirror the V3 engine (`@kiro/agent` 0.66.48, bundled with
//! kiro-cli 2.32.1): the trigger map is the engine's exact-case lookup table
//! and the tool ids and tags are the engine's built-in tool definitions.
//!
//! Legacy `.kiro/hooks/*.kiro.hook` payloads stay in [`crate::schemas::kiro_hook`].

use serde::Deserialize;

/// Canonical V3 trigger names.
pub const CANONICAL_KIRO_V3_HOOK_TRIGGERS: &[&str] = &[
    "SessionStart",
    "SessionEnd",
    "Stop",
    "PreToolUse",
    "PostToolUse",
    "PreTaskExec",
    "PostTaskExec",
    "UserPromptSubmit",
    "PostFileCreate",
    "PostFileSave",
    "PostFileDelete",
    "Manual",
];

/// Aliases the V3 engine accepts for a canonical trigger. The lookup is exact
/// and case-sensitive: `AgentSpawn` (PascalCase) is not an alias, although
/// the hook-types docs claim it is, and such a hook is skipped.
pub const KIRO_V3_HOOK_TRIGGER_ALIASES: &[(&str, &str)] = &[
    ("sessionStart", "SessionStart"),
    ("agentSpawn", "SessionStart"),
    ("agentStop", "Stop"),
    ("stop", "Stop"),
    ("promptSubmit", "UserPromptSubmit"),
    ("userPromptSubmit", "UserPromptSubmit"),
    ("preToolUse", "PreToolUse"),
    ("postToolUse", "PostToolUse"),
    ("preTaskExecution", "PreTaskExec"),
    ("postTaskExecution", "PostTaskExec"),
    ("fileEdited", "PostFileSave"),
    ("AfterFileEdit", "PostFileSave"),
    ("fileCreated", "PostFileCreate"),
    ("fileDeleted", "PostFileDelete"),
    ("userTriggered", "Manual"),
];

/// Every trigger name the V3 engine accepts (canonical names and aliases).
/// Kept for validators of agent-embedded array hooks, which share the hook
/// schema; prefer [`resolve_trigger`], which also returns the canonical name.
#[cfg(test)]
pub const VALID_KIRO_V3_HOOK_TRIGGERS: &[&str] = &[
    "SessionStart",
    "SessionEnd",
    "Stop",
    "PreToolUse",
    "PostToolUse",
    "PreTaskExec",
    "PostTaskExec",
    "UserPromptSubmit",
    "PostFileCreate",
    "PostFileSave",
    "PostFileDelete",
    "Manual",
    "sessionStart",
    "agentSpawn",
    "agentStop",
    "stop",
    "promptSubmit",
    "userPromptSubmit",
    "preToolUse",
    "postToolUse",
    "preTaskExecution",
    "postTaskExecution",
    "fileEdited",
    "AfterFileEdit",
    "fileCreated",
    "fileDeleted",
    "userTriggered",
];

/// The canonical trigger for an accepted alias. Canonical names and unknown
/// names return `None`.
pub fn canonical_trigger(trigger: &str) -> Option<&'static str> {
    KIRO_V3_HOOK_TRIGGER_ALIASES
        .iter()
        .find(|(alias, _)| *alias == trigger)
        .map(|(_, canonical)| *canonical)
}

/// The canonical trigger the V3 engine resolves `trigger` to, or `None` when
/// the engine does not recognize the name.
pub fn resolve_trigger(trigger: &str) -> Option<&'static str> {
    CANONICAL_KIRO_V3_HOOK_TRIGGERS
        .iter()
        .find(|canonical| **canonical == trigger)
        .copied()
        .or_else(|| canonical_trigger(trigger))
}

/// The canonical trigger whose canonical or alias spelling equals `trigger`
/// ignoring ASCII case, for an unrecognized `trigger`.
pub fn trigger_case_insensitive_match(trigger: &str) -> Option<&'static str> {
    CANONICAL_KIRO_V3_HOOK_TRIGGERS
        .iter()
        .map(|canonical| (*canonical, *canonical))
        .chain(KIRO_V3_HOOK_TRIGGER_ALIASES.iter().copied())
        .find(|(name, _)| name.eq_ignore_ascii_case(trigger))
        .map(|(_, canonical)| canonical)
}

/// What a hook matcher is tested against, per canonical trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatcherSubject {
    /// Tool triggers: a selector over tool ids and tags, or a regex over ids.
    Tool,
    /// File triggers: a regex over the file path.
    FilePath,
    /// Every other trigger ignores the matcher.
    None,
}

pub fn matcher_subject(canonical_trigger: &str) -> MatcherSubject {
    match canonical_trigger {
        "PreToolUse" | "PostToolUse" => MatcherSubject::Tool,
        "PostFileCreate" | "PostFileSave" | "PostFileDelete" => MatcherSubject::FilePath,
        _ => MatcherSubject::None,
    }
}

/// Documented `version` values for the standalone hook document.
pub const VALID_KIRO_V3_HOOK_DOC_VERSIONS: &[&str] = &["v1"];

/// Keys the V3 schema reads at each level. Other keys are stripped silently.
pub const KIRO_V3_HOOK_KEYS: &[&str] = &[
    "name",
    "description",
    "trigger",
    "matcher",
    "action",
    "timeout",
    "enabled",
    "confirm",
];
pub const KIRO_V3_HOOK_ACTION_KEYS: &[&str] = &["type", "command", "prompt"];

/// Built-in V3 tool ids with their tags (engine tool definitions). Tools the
/// IDE registers under the fs_write capability are included so that matchers
/// naming them are not reported.
const V3_BUILTIN_TOOLS: &[(&str, &[&str])] = &[
    ("read_file", &["read", "@builtin"]),
    ("list_directory", &["read", "@builtin"]),
    ("file_search", &["read", "@builtin"]),
    ("grep_search", &["read", "@builtin"]),
    ("introspect", &["read", "@builtin"]),
    ("code", &["read", "write", "@builtin"]),
    ("tool_load", &["read", "@builtin", "@mcp-disclosure"]),
    ("tool_call", &["read", "@builtin", "@mcp-disclosure"]),
    ("fs_write", &["write", "@builtin"]),
    ("fs_append", &["write", "@builtin"]),
    ("str_replace", &["write", "@builtin"]),
    ("delete_file", &["write", "@builtin"]),
    ("edit_code", &["write"]),
    ("semantic_rename", &["write"]),
    ("smart_relocate", &["write"]),
    ("create_hook", &["write"]),
    ("execute_bash", &["shell", "@builtin"]),
    ("execute_pwsh", &["shell", "@builtin"]),
    ("control_bash_process", &["shell", "@builtin"]),
    ("control_pwsh_process", &["shell", "@builtin"]),
    ("list_processes", &["shell", "@builtin"]),
    ("get_process_output", &["shell", "@builtin"]),
    ("web_fetch", &["web", "@builtin"]),
    ("web_search", &["web", "@builtin"]),
    ("remote_web_search", &["web", "@builtin"]),
    ("analyze_requirements", &["spec", "@builtin"]),
    ("code_to_spec", &["spec", "@builtin"]),
    ("task_get", &["spec", "@builtin"]),
    ("task_list", &["spec", "@builtin"]),
    ("task_update", &["spec", "@builtin"]),
    ("update_pbt_status", &["spec", "@builtin"]),
    ("validate_spec_format", &["spec", "@builtin"]),
    ("verify_requirements", &["spec", "@builtin"]),
    ("prework", &["spec", "@builtin"]),
    ("user_input", &["spec", "@builtin"]),
    ("invoke_sub_agent", &["@builtin", "@subagent"]),
    // Per-agent dispatch tools are `subagent_<agent>`.
    (
        "subagent_agent",
        &["subagent", "@subagent", "subagent/agent"],
    ),
    ("memory", &["context", "@builtin"]),
    ("remote_get_learnings", &["context"]),
    ("remote_get_learning", &["context"]),
    ("remote_create_feedback_learning", &["context"]),
    ("remote_delete_learning", &["context"]),
    ("remote_get_learnings_for_prompt", &["context"]),
    ("remote_get_steering_files", &["context"]),
    ("knowledge", &["@builtin"]),
    ("subagent_response", &["@builtin"]),
    ("run_workflow", &["@builtin"]),
    ("inspect_workflow", &["@builtin"]),
    ("update_workflow", &["@builtin"]),
    ("save_workflow_definition", &["@builtin"]),
    ("send_message", &["@builtin"]),
    ("kiro_powers", &["@powers"]),
    ("disclose_context", &[]),
    ("report_progress", &[]),
    ("switch_to_execution", &[]),
    ("update_session_information", &[]),
    ("create_diagram", &[]),
];

/// Tool names from Kiro CLI 2.x and from other agents (Claude Code) that are
/// neither V3 tool ids nor V3 tags, with the V3 replacement to suggest.
const FOREIGN_TOOL_NAMES: &[(&str, &str, &str)] = &[
    ("fs_read", "Kiro CLI 2.x", "read or read_file"),
    ("fsRead", "Kiro CLI 2.x", "read or read_file"),
    ("fsWrite", "Kiro CLI 2.x", "write or fs_write"),
    ("execute_cmd", "Kiro CLI 2.x", "shell or execute_bash"),
    ("executeCmd", "Kiro CLI 2.x", "shell or execute_bash"),
    ("executeBash", "Kiro CLI 2.x", "shell or execute_bash"),
    (
        "use_aws",
        "Kiro CLI 2.x",
        "the tools of an AWS MCP server (@<server>)",
    ),
    (
        "useAws",
        "Kiro CLI 2.x",
        "the tools of an AWS MCP server (@<server>)",
    ),
    (
        "aws",
        "Kiro CLI 2.x",
        "the tools of an AWS MCP server (@<server>)",
    ),
    ("glob", "Kiro CLI 2.x", "file_search"),
    ("grep", "Kiro CLI 2.x", "grep_search"),
    ("use_subagent", "Kiro CLI 2.x", "subagent"),
    ("agent_crew", "Kiro CLI 2.x", "subagent"),
    ("todo_list", "Kiro CLI 2.x", "no V3 equivalent"),
    ("report_issue", "Kiro CLI 2.x", "no V3 equivalent"),
    ("Bash", "Claude Code", "shell or execute_bash"),
    ("Read", "Claude Code", "read or read_file"),
    ("Write", "Claude Code", "write or fs_write"),
    ("Edit", "Claude Code", "write or str_replace"),
    ("MultiEdit", "Claude Code", "write or str_replace"),
    ("NotebookEdit", "Claude Code", "write"),
    ("Glob", "Claude Code", "file_search"),
    ("Grep", "Claude Code", "grep_search"),
    ("LS", "Claude Code", "list_directory"),
    ("Task", "Claude Code", "subagent"),
    ("WebFetch", "Claude Code", "web_fetch"),
    ("WebSearch", "Claude Code", "web_search"),
    ("TodoWrite", "Claude Code", "no V3 equivalent"),
];

/// V3 treats these selector names as equivalent tool ids.
fn selector_alias(name: &str) -> Option<&'static str> {
    match name {
        "execute_bash" => Some("execute_pwsh"),
        "execute_pwsh" => Some("execute_bash"),
        "control_bash_process" => Some("control_pwsh_process"),
        "control_pwsh_process" => Some("control_bash_process"),
        _ => None,
    }
}

/// True when a matcher selects every event its trigger can produce: no
/// matcher, an empty one, the `*` selector (tool triggers), or a regex that
/// matches any subject such as `.*`.
pub fn matches_everything(matcher: Option<&str>, subject: MatcherSubject) -> bool {
    let Some(matcher) = matcher else {
        return true;
    };
    let any_regex = matches!(
        matcher,
        "" | ".*" | "^.*" | ".*$" | "^.*$" | "(.*)" | "^(.*)$" | ".+" | "^.+" | ".+$" | "^.+$"
    );
    any_regex || (subject == MatcherSubject::Tool && matches!(matcher, "*" | "**"))
}

/// Glob patterns that select every file (legacy IDE `when.patterns`).
pub fn is_match_all_glob(pattern: &str) -> bool {
    matches!(
        pattern.trim(),
        "*" | "**" | "**/*" | "**/*.*" | "*.*" | "./**" | "./**/*" | "**/**"
    )
}

/// True when V3 treats a tool matcher as a selector as well as a regex: it
/// contains none of the regex metacharacters `\ ^ $ ( ) [ ] { } | +`.
pub fn is_tool_selector(matcher: &str) -> bool {
    !matcher.contains(['\\', '^', '$', '(', ')', '[', ']', '{', '}', '|', '+'])
}

fn is_glob_selector(selector: &str) -> bool {
    selector.contains(['*', '?'])
}

/// Anchored `*`/`?` glob match, as V3 does for tool selectors.
fn glob_matches(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut p, mut t) = (0, 0);
    let (mut star, mut star_text) = (None, 0);
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            star_text = t;
            p += 1;
        } else if let Some(star_pos) = star {
            p = star_pos + 1;
            star_text += 1;
            t = star_text;
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|ch| *ch == '*')
}

fn selector_matches(selector: &str, id: &str, tags: &[&str]) -> bool {
    let glob = is_glob_selector(selector);
    let matches = |candidate: &str| {
        if glob {
            glob_matches(selector, candidate)
        } else {
            candidate == selector
        }
    };
    matches(id)
        || (!glob && selector_alias(selector) == Some(id))
        || tags.iter().any(|t| matches(t))
}

/// Compile a matcher with JavaScript regex syntax (V3 uses `new RegExp`).
/// `regress` implements ECMAScript syntax; it never rejects a pattern that
/// V8 accepts (it is slightly more lenient), so a compile error is certain.
pub fn compile_js_regex(pattern: &str) -> Result<regress::Regex, regress::Error> {
    regress::Regex::new(pattern)
}

/// Whether a tool-trigger matcher can match any V3 built-in tool, using the
/// V3 rules: a selector-shaped matcher matches ids or tags (with globs) and
/// is also an unanchored regex over ids; any other matcher is only a regex.
/// Returns `None` when the regex cannot be evaluated.
pub fn tool_matcher_matches_builtin(matcher: &str) -> Option<bool> {
    let selector = is_tool_selector(matcher);
    let regex = compile_js_regex(matcher).ok();
    if regex.is_none() && !(selector && is_glob_selector(matcher)) {
        return None;
    }
    Some(V3_BUILTIN_TOOLS.iter().any(|(id, tags)| {
        (selector && selector_matches(matcher, id, tags))
            || regex.as_ref().is_some_and(|re| re.find(id).is_some())
    }))
}

/// Foreign tool names (CLI 2.x or Claude Code) named in a matcher, with their
/// origin and V3 replacement, in first-occurrence order.
pub fn foreign_tool_names(matcher: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    let mut found = Vec::new();
    for word in matcher.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')) {
        if let Some(entry) = FOREIGN_TOOL_NAMES.iter().find(|(name, ..)| *name == word)
            && !found.contains(entry)
        {
            found.push(*entry);
        }
    }
    found
}

/// One step of a JSON path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Seg<'a> {
    Key(&'a str),
    Index(usize),
}

/// A located JSON value: the value's byte span and the 1-based location of
/// the value and, for object members, of the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Located {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub key_line: usize,
    pub key_column: usize,
}

#[derive(Debug)]
struct Node {
    located: Located,
    key: Option<String>,
    children: Vec<usize>,
}

/// Value spans for a JSON document, keyed by decoded path. Duplicate keys are
/// kept: lookups follow the last occurrence (what `JSON.parse` keeps), and
/// [`JsonSpans::unique`] refuses any path that crosses a duplicated key, so
/// fixes never edit a value that another copy could shadow or restore.
#[derive(Debug, Default)]
pub(crate) struct JsonSpans {
    nodes: Vec<Node>,
}

#[cfg(test)]
thread_local! {
    pub(crate) static SPAN_INDEX_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Containers deeper than this are located but not descended into, which
/// bounds the work on deeply nested values (each level reparses its bytes).
const MAX_SPAN_DEPTH: usize = 6;

struct Cursor<'a> {
    content: &'a str,
    offset: usize,
    line: usize,
    column: usize,
}

impl Cursor<'_> {
    /// Advance to `offset` (never backwards) and return its location.
    fn at(&mut self, offset: usize) -> (usize, usize) {
        debug_assert!(offset >= self.offset, "span locations must be monotonic");
        if offset > self.offset {
            for ch in self.content[self.offset..offset].chars() {
                if ch == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
            }
            self.offset = offset;
        }
        (self.line, self.column)
    }
}

impl JsonSpans {
    /// Index `content`, which has already passed JSON parsing. Children are
    /// visited in document order, so locations are computed in one forward
    /// pass however many values are located.
    pub fn new(content: &str) -> Self {
        #[cfg(test)]
        SPAN_INDEX_BUILDS.with(|builds| builds.set(builds.get() + 1));

        let mut spans = Self::default();
        let trimmed = content.trim_start();
        let start = content.len() - trimmed.len();
        let Some((_, length)) = first_value::<serde::de::IgnoredAny>(trimmed) else {
            return spans;
        };
        let mut cursor = Cursor {
            content,
            offset: 0,
            line: 1,
            column: 1,
        };
        let (line, column) = cursor.at(start);
        spans.nodes.push(Node {
            located: Located {
                start,
                end: start + length,
                line,
                column,
                key_line: line,
                key_column: column,
            },
            key: None,
            children: Vec::new(),
        });
        if spans.descend(content, 0, 0, &mut cursor).is_none() {
            return Self::default();
        }
        spans
    }

    fn descend(
        &mut self,
        content: &str,
        node: usize,
        depth: usize,
        cursor: &mut Cursor<'_>,
    ) -> Option<()> {
        let Located { start, end, .. } = self.nodes[node].located;
        let text = &content[start..end];
        if depth >= MAX_SPAN_DEPTH || !(text.starts_with('{') || text.starts_with('[')) {
            return Some(());
        }
        let mut children = Vec::new();
        visit_json_children(
            text,
            start,
            &mut |key, key_start, value_start, value_end| {
                children.push((key.map(str::to_owned), key_start, value_start, value_end));
            },
        )?;
        for (key, key_start, value_start, value_end) in children {
            let (key_line, key_column) = cursor.at(key_start);
            let (line, column) = cursor.at(value_start);
            let child = self.nodes.len();
            self.nodes.push(Node {
                located: Located {
                    start: value_start,
                    end: value_end,
                    line,
                    column,
                    key_line,
                    key_column,
                },
                key,
                children: Vec::new(),
            });
            self.nodes[node].children.push(child);
            self.descend(content, child, depth + 1, cursor)?;
        }
        Some(())
    }

    fn find(&self, path: &[Seg<'_>], unique: bool) -> Option<Located> {
        let mut node = self.nodes.first()?;
        for seg in path {
            let next = match seg {
                Seg::Key(key) => {
                    let mut matches = node
                        .children
                        .iter()
                        .map(|&i| &self.nodes[i])
                        .filter(|child| child.key.as_deref() == Some(*key));
                    let last = matches.next_back()?;
                    if unique && matches.next().is_some() {
                        return None;
                    }
                    last
                }
                Seg::Index(index) => {
                    let child = &self.nodes[*node.children.get(*index)?];
                    if child.key.is_some() {
                        return None;
                    }
                    child
                }
            };
            node = next;
        }
        Some(node.located)
    }

    /// The value at `path`, following the last occurrence of duplicate keys.
    pub fn get(&self, path: &[Seg<'_>]) -> Option<Located> {
        self.find(path, false)
    }

    /// The value at `path` only when no key along the path is duplicated.
    pub fn unique(&self, path: &[Seg<'_>]) -> Option<Located> {
        self.find(path, true)
    }

    /// The closest located ancestor-or-self of `path` (falls back to the root).
    pub fn nearest(&self, path: &[Seg<'_>]) -> Option<Located> {
        (0..=path.len())
            .rev()
            .find_map(|len| self.get(&path[..len]))
    }
}

fn first_value<'de, T: Deserialize<'de>>(content: &'de str) -> Option<(T, usize)> {
    let mut stream = serde_json::Deserializer::from_str(content).into_iter::<T>();
    let value = stream.next()?.ok()?;
    Some((value, stream.byte_offset()))
}

/// Visit the immediate children of the object or array in `content`, using
/// serde's parser for keys and values (no second string/escape parser).
/// Offsets passed to the visitor are absolute (`base` + local offset) and
/// remain valid for UTF-8 and escapes: (decoded key, key start, value start,
/// value end). For array elements the key is `None` and the key start equals
/// the value start.
fn visit_json_children(
    content: &str,
    base: usize,
    visitor: &mut impl FnMut(Option<&str>, usize, usize, usize),
) -> Option<()> {
    let trimmed = content.trim_start();
    let base = base + content.len() - trimmed.len();
    let object = trimmed.starts_with('{');
    if !object && !trimmed.starts_with('[') {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let skip_ws = |mut pos: usize| {
        while bytes.get(pos).is_some_and(u8::is_ascii_whitespace) {
            pos += 1;
        }
        pos
    };
    let mut pos = 1;
    loop {
        pos = skip_ws(pos);
        if matches!(bytes.get(pos), Some(b'}' | b']')) {
            break;
        }
        let key_start = pos;
        let key = if object {
            let (key, length) = first_value::<String>(trimmed.get(pos..)?)?;
            pos = skip_ws(pos + length);
            if bytes.get(pos) != Some(&b':') {
                return None;
            }
            pos = skip_ws(pos + 1);
            Some(key)
        } else {
            None
        };
        let (_, length) = first_value::<serde::de::IgnoredAny>(trimmed.get(pos..)?)?;
        visitor(
            key.as_deref(),
            base + key_start,
            base + pos,
            base + pos + length,
        );
        pos = skip_ws(pos + length);
        if bytes.get(pos) == Some(&b',') {
            pos += 1;
        } else {
            break;
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_table_matches_v3_engine() {
        for canonical in CANONICAL_KIRO_V3_HOOK_TRIGGERS {
            assert_eq!(resolve_trigger(canonical), Some(*canonical));
            assert_eq!(canonical_trigger(canonical), None);
        }
        for (alias, canonical) in KIRO_V3_HOOK_TRIGGER_ALIASES {
            assert_eq!(resolve_trigger(alias), Some(*canonical));
            assert_eq!(canonical_trigger(alias), Some(*canonical));
        }
        let mut names: Vec<&str> = CANONICAL_KIRO_V3_HOOK_TRIGGERS
            .iter()
            .copied()
            .chain(KIRO_V3_HOOK_TRIGGER_ALIASES.iter().map(|(alias, _)| *alias))
            .collect();
        let mut valid = VALID_KIRO_V3_HOOK_TRIGGERS.to_vec();
        names.sort_unstable();
        valid.sort_unstable();
        assert_eq!(names, valid);
        // Case-sensitive: the engine rejects these spellings.
        for unknown in ["AgentSpawn", "sessionstart", "PostFileEdit", "Stop ", ""] {
            assert_eq!(resolve_trigger(unknown), None, "{unknown}");
        }
        assert_eq!(
            trigger_case_insensitive_match("AgentSpawn"),
            Some("SessionStart")
        );
        assert_eq!(
            trigger_case_insensitive_match("postfilesave"),
            Some("PostFileSave")
        );
        assert_eq!(trigger_case_insensitive_match("OnFileChange"), None);
    }

    #[test]
    fn tool_matchers_follow_v3_selector_and_regex_rules() {
        for (matcher, matches) in [
            ("shell", true),
            ("read", true),
            ("@builtin", true),
            ("*", true),
            ("fs_*", true),
            ("execute_bash", true),
            ("grep", true), // unanchored regex: grep_search
            ("^(fs_write|str_replace)$", true),
            ("([Ss]hell|[Bb]ash)", true), // execute_bash contains "bash"
            ("^Bash$|^Write$|^Edit$", false),
            ("^Task$", false),
            ("fs_read", false),
            ("Bash", false),
            ("glob", false),
            ("aws", false),
        ] {
            assert_eq!(
                tool_matcher_matches_builtin(matcher),
                Some(matches),
                "{matcher}"
            );
        }
        assert_eq!(tool_matcher_matches_builtin("(?<=a)*"), None);
        assert_eq!(
            foreign_tool_names("^Bash$|^Write$|^Bash$")
                .iter()
                .map(|(name, ..)| *name)
                .collect::<Vec<_>>(),
            ["Bash", "Write"]
        );
    }

    #[test]
    fn js_regex_compile_errors_match_v8() {
        for bad in ["*.ts", "**/*.ts", "src/**", "*|read", "(", "[a", "a{2}{3}"] {
            assert!(compile_js_regex(bad).is_err(), "{bad}");
        }
        for good in ["\\.ts$", "src/*.ts", "a{", "]", "(?=a)*", "\\k<x>", "[]"] {
            assert!(compile_js_regex(good).is_ok(), "{good}");
        }
    }

    #[test]
    fn match_all_matchers() {
        for (matcher, subject, expected) in [
            (None, MatcherSubject::Tool, true),
            (Some(""), MatcherSubject::FilePath, true),
            (Some("*"), MatcherSubject::Tool, true),
            (Some("*"), MatcherSubject::FilePath, false), // a compile error instead
            (Some(".*"), MatcherSubject::FilePath, true),
            (Some("^.*$"), MatcherSubject::Tool, true),
            (Some("shell"), MatcherSubject::Tool, false),
            (Some("\\.ts$"), MatcherSubject::FilePath, false),
        ] {
            assert_eq!(
                matches_everything(matcher, subject),
                expected,
                "{matcher:?}"
            );
        }
        assert!(is_match_all_glob("**/*") && is_match_all_glob(" * "));
        assert!(!is_match_all_glob("src/**/*.ts"));
    }

    #[test]
    fn glob_matching_is_anchored() {
        assert!(glob_matches("fs_*", "fs_write"));
        assert!(glob_matches("*", ""));
        assert!(glob_matches("?s_w*e", "fs_write"));
        assert!(!glob_matches("fs_*", "xfs_write"));
        assert!(!glob_matches("fs?", "fs"));
    }

    #[test]
    fn spans_decode_keys_and_follow_last_duplicate() {
        let content = r#"{
          "note": "é🙂 \\\"trigger\\\": \\\"stop\\\"",
          "nested": {"hooks": [{"trigger": "stop"}], "version": "1"},
          "hooks": [
            {"name": "é🙂", "trigger": "agentSpawn", "action": {"trigger": "stop"}},
            {"name": "no trigger"},
            {"trigger": "stop", "trigger": "Stop"}
          ],
          "version": "1.0"
        }"#;
        let _: serde_json::Value = serde_json::from_str(content).unwrap();
        let spans = JsonSpans::new(content);
        let version = spans.unique(&[Seg::Key("version")]).unwrap();
        let trigger = spans
            .unique(&[Seg::Key("hooks"), Seg::Index(0), Seg::Key("trigger")])
            .unwrap();
        for (span, expected, key) in [
            (version, r#""1.0""#, r#""version""#),
            (trigger, r#""agentSpawn""#, r#""trigger""#),
        ] {
            assert_eq!(&content[span.start..span.end], expected);
            assert_eq!(
                (span.line, span.column),
                crate::rules::line_col_at_offset(content, span.start)
            );
            let key_offset = content[..span.start].rfind(key).unwrap();
            assert_eq!(
                (span.key_line, span.key_column),
                crate::rules::line_col_at_offset(content, key_offset)
            );
        }
        assert!(
            spans
                .get(&[Seg::Key("hooks"), Seg::Index(1), Seg::Key("trigger")])
                .is_none()
        );
        let last = [Seg::Key("hooks"), Seg::Index(2), Seg::Key("trigger")];
        let located = spans.get(&last).unwrap();
        assert_eq!(&content[located.start..located.end], r#""Stop""#);
        assert!(spans.unique(&last).is_none());
    }

    #[test]
    fn spans_reject_duplicate_containers_for_unique_lookups() {
        for content in [
            r#"{"hooks":[],"hooks":[{"trigger":"stop"}]}"#,
            r#"{"hooks":[{"trigger":"stop"}],"hooks":[]}"#,
            r#"{"hooks":null,"hooks":[{"trigger":"stop"}]}"#,
        ] {
            let spans = JsonSpans::new(content);
            let path = [Seg::Key("hooks"), Seg::Index(0), Seg::Key("trigger")];
            assert!(spans.unique(&path).is_none(), "{content}");
        }
        let spans = JsonSpans::new(r#"{"hooks":[{"trigger":"stop", "note":1, "note":2}]}"#);
        assert!(
            spans
                .unique(&[Seg::Key("hooks"), Seg::Index(0), Seg::Key("trigger")])
                .is_some(),
            "unrelated duplicate keys are not ambiguous"
        );
    }

    #[test]
    fn spans_locate_keys_and_nearest_ancestor() {
        let content = "{\n  \"hooks\": [\n    {\"name\": \"a\"}\n  ]\n}";
        let spans = JsonSpans::new(content);
        let name = spans
            .get(&[Seg::Key("hooks"), Seg::Index(0), Seg::Key("name")])
            .unwrap();
        assert_eq!((name.key_line, name.key_column), (3, 6));
        assert_eq!((name.line, name.column), (3, 14));
        let entry = spans
            .nearest(&[Seg::Key("hooks"), Seg::Index(0), Seg::Key("action")])
            .unwrap();
        assert_eq!((entry.line, entry.column), (3, 5));
    }
}
