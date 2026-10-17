//! Kiro custom agent schema data shared by the agent validator.
//!
//! Kiro CLI 2.x ships two agent engines: V2, the 2.x default, and V3, the
//! harness that becomes the default in CLI 3.0. The tables below record which
//! engine reads each agent field, tool name, and hook trigger, following the
//! Kiro CLI 2.32.1 V2 loader (probed with `kiro-cli agent validate`) and the
//! V3 engine bundled with it (`@kiro/agent` 0.66.48), plus the kiro.dev docs:
//! <https://kiro.dev/docs/custom-agents/configuration-reference/>,
//! <https://kiro.dev/docs/cli/v3/agent-config/>,
//! <https://kiro.dev/docs/cli/v3/migration-guide/>.
//!
//! The module also locates values inside agent files so diagnostics can point
//! at the offending key or value: [`strip_jsonc`] blanks the comments and
//! trailing commas V3 accepts without moving any byte, [`JsonNode`] indexes
//! the spans of a parsed JSON document, and [`FrontmatterIndex`] finds keys in
//! Markdown agent frontmatter.

use crate::schemas::kiro_hook_v3::{resolve_trigger, trigger_case_insensitive_match};

/// Which engine reads a top-level agent field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldOrigin {
    /// Read by both engines.
    Shared,
    /// A CLI 2.x field; V3 strips it (or skips the profile, see KR-AG-015).
    V2,
    /// A V3 field; V2 ignores it.
    V3,
    /// Bookkeeping both engines accept and ignore (`$schema`, `kiroUpgrade`).
    Marker,
}

/// Top-level agent fields and the engine that reads them (spec 1.3, 10.9, 10.10).
pub(crate) const AGENT_FIELDS: &[(&str, FieldOrigin)] = &[
    ("name", FieldOrigin::Shared),
    ("description", FieldOrigin::Shared),
    ("prompt", FieldOrigin::Shared),
    ("model", FieldOrigin::Shared),
    ("tools", FieldOrigin::Shared),
    ("resources", FieldOrigin::Shared),
    ("mcpServers", FieldOrigin::Shared),
    ("includeMcpJson", FieldOrigin::Shared),
    ("hooks", FieldOrigin::Shared),
    ("welcomeMessage", FieldOrigin::Shared),
    ("allowedTools", FieldOrigin::V2),
    ("toolsSettings", FieldOrigin::V2),
    ("toolAliases", FieldOrigin::V2),
    ("keyboardShortcut", FieldOrigin::V2),
    ("useLegacyMcpJson", FieldOrigin::V2),
    ("excludedTools", FieldOrigin::V3),
    ("includePowers", FieldOrigin::V3),
    ("permissions", FieldOrigin::V3),
    ("effortLevel", FieldOrigin::V3),
    ("dispatchKind", FieldOrigin::V3),
    ("$schema", FieldOrigin::Marker),
    ("kiroUpgrade", FieldOrigin::Marker),
];

pub(crate) fn field_origin(field: &str) -> Option<FieldOrigin> {
    AGENT_FIELDS
        .iter()
        .find(|(name, _)| *name == field)
        .map(|(_, origin)| *origin)
}

/// Fields whose presence marks a file as written for CLI 2.x (spec 0.5).
pub(crate) const V2_MARKER_FIELDS: &[&str] = &[
    "allowedTools",
    "toolsSettings",
    "toolAliases",
    "keyboardShortcut",
    "useLegacyMcpJson",
];

/// Fields whose presence marks a file as written for V3 (spec 0.5, 10.10).
/// `permissions` counts only when it is not null.
pub(crate) const V3_MARKER_FIELDS: &[&str] = &[
    "excludedTools",
    "includePowers",
    "effortLevel",
    "dispatchKind",
];

/// V3 `dispatchKind` values.
pub(crate) const DISPATCH_KINDS: &[&str] = &["sub-agent", "custom-agent", "spec"];

/// File agent ids V3 silently drops because built-in agents own them (spec 1.7).
pub(crate) const V3_RESERVED_AGENT_IDS: &[&str] = &[
    "vibe",
    "spec",
    "quick-spec",
    "bug-fix",
    "plan",
    "autonomous",
    "default-v2",
];

/// Built-in agents an agent can spawn besides the agent files: CLI 2.x
/// built-ins and V3 bundled agents (spec 1.7). V3's reserved ids are in
/// [`V3_RESERVED_AGENT_IDS`].
pub(crate) const KNOWN_BUILT_IN_AGENTS: &[&str] = &[
    "kiro_default",
    "kiro_help",
    "kiro_planner",
    "kiro_guide",
    "wf-auto-researcher",
    "wf-code-reviewer",
    "wf-coder",
    "wf-design",
    "wf-design-reviewer",
    "wf-planner",
    "wf-pr-responder",
    "wf-pr-submitter",
    "wf-review-aggregator",
    "wf-workflow-creator",
    "semantic_reviewer",
    "functional_task_alignment",
    "Explore",
    "validation",
    "create-spec",
];

/// CLI 2.x built-in tool names; each group names one tool (spec 7.1).
/// Names are case-sensitive.
pub(crate) const V2_TOOL_GROUPS: &[&[&str]] = &[
    &["read", "fs_read", "fsRead"],
    &["write", "fs_write", "fsWrite"],
    &[
        "shell",
        "execute_bash",
        "execute_cmd",
        "executeBash",
        "executeCmd",
    ],
    &["aws", "use_aws"],
    &["subagent", "use_subagent", "agent_crew", "agentCrew"],
    &["todo", "todo_list", "task"],
    &["glob"],
    &["grep"],
    &["web_search"],
    &["web_fetch"],
    &["code"],
    &["introspect"],
    &["knowledge"],
    &["thinking"],
    &["delegate"],
    &["report"],
    &["tool_search"],
    &["session"],
    &["goal"],
    &["switch_to_execution"],
];

/// The group of CLI 2.x names that select the same tool as `name`.
pub(crate) fn v2_tool_group(name: &str) -> Option<&'static [&'static str]> {
    V2_TOOL_GROUPS
        .iter()
        .copied()
        .find(|group| group.contains(&name))
}

/// V3 tool tags usable as `tools` selectors (spec 7.2).
pub(crate) const V3_TOOL_TAGS: &[&str] = &[
    "read", "write", "shell", "web", "spec", "subagent", "context",
];

/// V3 built-in tool ids (spec 7.2).
pub(crate) const V3_TOOL_IDS: &[&str] = &[
    "read_file",
    "list_directory",
    "file_search",
    "grep_search",
    "introspect",
    "code",
    "tool_load",
    "tool_call",
    "fs_write",
    "fs_append",
    "str_replace",
    "delete_file",
    "execute_bash",
    "execute_pwsh",
    "control_bash_process",
    "control_pwsh_process",
    "list_processes",
    "get_process_output",
    "web_fetch",
    "remote_web_search",
    "analyze_requirements",
    "code_to_spec",
    "task_get",
    "task_list",
    "task_update",
    "update_pbt_status",
    "validate_spec_format",
    "verify_requirements",
    "prework",
    "user_input",
    "invoke_sub_agent",
    "memory",
    "knowledge",
    "subagent_response",
    "run_workflow",
    "inspect_workflow",
    "update_workflow",
    "save_workflow_definition",
    "send_message",
    "kiro_powers",
    "disclose_context",
    "report_progress",
    "switch_to_execution",
    "update_session_information",
    "create_diagram",
];

/// What V3 offers for a CLI 2.x tool name that selects nothing in V3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum V3Replacement {
    /// The V3 selector that covers the tool.
    Selector(&'static str),
    /// V3 has no AWS tool; an MCP server replaces it.
    McpServer,
}

/// CLI 2.x names V3 does not alias, so they select nothing there (spec 7.4,
/// 10.7). Names that are also V3 ids or tags (read, write, shell, fs_write,
/// execute_bash, web_fetch, code, introspect, knowledge, subagent) are absent,
/// and so are names of features V3 does not have (todo, thinking, ...).
pub(crate) fn v3_replacement_for_v2_tool(name: &str) -> Option<V3Replacement> {
    use V3Replacement::{McpServer, Selector};
    Some(match name {
        "fs_read" | "fsRead" | "grep" | "glob" => Selector("read"),
        "fsWrite" => Selector("write"),
        "execute_cmd" | "executeBash" | "executeCmd" => Selector("shell"),
        "web_search" => Selector("web"),
        "use_subagent" | "agent_crew" | "agentCrew" => Selector("subagent"),
        "use_aws" | "aws" => McpServer,
        _ => return None,
    })
}

/// V3 tool ids CLI 2.x does not know, with the CLI 2.x tool that does the same
/// job (spec 7.4).
pub(crate) fn v2_equivalent_for_v3_tool(name: &str) -> Option<&'static str> {
    Some(match name {
        "read_file" | "list_directory" => "read",
        "file_search" => "glob",
        "grep_search" => "grep",
        "fs_append" | "str_replace" | "delete_file" => "write",
        "execute_pwsh" => "shell",
        "remote_web_search" => "web_search",
        "invoke_sub_agent" => "subagent",
        _ => return None,
    })
}

/// Claude Code tool names, a common porting mistake: Kiro names are
/// lowercase and case-sensitive, so these select and trust nothing.
pub(crate) fn claude_code_tool_replacement(name: &str) -> Option<&'static str> {
    Some(match name {
        "Read" | "NotebookRead" => "read",
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => "write",
        "Bash" | "BashOutput" | "KillShell" => "shell",
        "Glob" | "LS" => "glob",
        "Grep" => "grep",
        "WebFetch" => "web_fetch",
        "WebSearch" => "web_search",
        "Task" => "subagent",
        "TodoWrite" | "TodoRead" => "todo",
        _ => return None,
    })
}

/// A known Kiro tool name that equals `name` ignoring ASCII case, when `name`
/// itself is not a known name.
pub(crate) fn case_variant_of_known_tool(name: &str) -> Option<&'static str> {
    if is_known_tool_name(name) {
        return None;
    }
    V2_TOOL_GROUPS
        .iter()
        .flat_map(|group| group.iter())
        .chain(V3_TOOL_TAGS.iter())
        .chain(V3_TOOL_IDS.iter())
        .copied()
        .find(|known| known.eq_ignore_ascii_case(name))
}

pub(crate) fn is_known_tool_name(name: &str) -> bool {
    v2_tool_group(name).is_some() || V3_TOOL_TAGS.contains(&name) || V3_TOOL_IDS.contains(&name)
}

/// Object-form hook keys CLI 2.x accepts; any other key skips the agent
/// (spec 2.2).
pub(crate) const V2_HOOK_TRIGGERS: &[&str] = &[
    "agentSpawn",
    "userPromptSubmit",
    "preToolUse",
    "postToolUse",
    "stop",
];

/// The CLI 2.x object-form key a misspelled key most likely means, when the
/// two differ only in case or are V3 names for the same trigger.
pub(crate) fn v2_trigger_for(key: &str) -> Option<&'static str> {
    let canonical = resolve_trigger(key).or_else(|| trigger_case_insensitive_match(key))?;
    Some(match canonical {
        "SessionStart" => "agentSpawn",
        "UserPromptSubmit" => "userPromptSubmit",
        "PreToolUse" => "preToolUse",
        "PostToolUse" => "postToolUse",
        "Stop" => "stop",
        _ => return None,
    })
}

/// Keys of a CLI 2.x object-form hook entry (spec 2.2). `description` is
/// documentation only.
pub(crate) const V2_HOOK_ENTRY_FIELDS: &[&str] = &[
    "command",
    "matcher",
    "timeout_ms",
    "max_output_size",
    "cache_ttl_seconds",
    "description",
];

/// Keys of an array-form hook entry (spec 2.3); `cacheTtlSeconds` and
/// `maxOutputSize` are written by the CLI 2.x upgrader.
pub(crate) const ARRAY_HOOK_ENTRY_FIELDS: &[&str] = &[
    "name",
    "description",
    "trigger",
    "matcher",
    "action",
    "timeout",
    "enabled",
    "confirm",
    "cacheTtlSeconds",
    "maxOutputSize",
];

/// Claude Code model aliases. They are not Kiro model ids: CLI 2.x sends them
/// unchanged, and V3 accepts one only when it matches a served id
/// unambiguously (spec 8).
pub(crate) const CLAUDE_CODE_MODEL_ALIASES: &[&str] =
    &["inherit", "sonnet", "opus", "haiku", "opusplan", "default"];

/// Permission policy presets V3 accepts in `permissions.policies` (spec 6.3).
pub(crate) const PERMISSION_POLICY_PRESETS: &[&str] = &[
    "allow-all",
    "edit-workspace",
    "read-workspace",
    "read-all",
    "read-only-shell",
    "dev-shell",
];

/// Inline MCP server fields and their expected JSON shape (spec 3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum McpFieldShape {
    String,
    StringArray,
    StringMap,
    Bool,
    Number,
    Object,
}

/// `(field, shape, typed by the CLI 2.x loader)`. V3 validates every field.
pub(crate) const MCP_SERVER_FIELDS: &[(&str, McpFieldShape, bool)] = &[
    ("command", McpFieldShape::String, true),
    ("args", McpFieldShape::StringArray, true),
    ("env", McpFieldShape::StringMap, true),
    ("headers", McpFieldShape::StringMap, true),
    ("url", McpFieldShape::String, true),
    ("type", McpFieldShape::String, true),
    ("timeout", McpFieldShape::Number, true),
    ("disabled", McpFieldShape::Bool, true),
    ("oauth", McpFieldShape::Object, true),
    ("oauthScopes", McpFieldShape::StringArray, true),
    ("cwd", McpFieldShape::String, false),
    ("disabledTools", McpFieldShape::StringArray, false),
    ("autoApprove", McpFieldShape::StringArray, false),
    ("waitForReady", McpFieldShape::Bool, false),
];

/// Replace the comments and trailing commas V3 accepts in agent JSON with
/// spaces, keeping every other byte (and every line break) in place so byte
/// offsets into the result are offsets into the original. Returns the
/// stripped text and the offset of the first byte changed, if any.
pub(crate) fn strip_jsonc(content: &str) -> (String, Option<usize>) {
    fn blank(out: &mut [u8], index: usize, first: &mut Option<usize>) {
        if out[index] != b'\n' && out[index] != b'\r' {
            out[index] = b' ';
        }
        first.get_or_insert(index);
    }

    let bytes = content.as_bytes();
    let mut out = bytes.to_vec();
    let mut first_change: Option<usize> = None;
    let mut index = 0;
    let mut in_string = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            match byte {
                b'\\' => index += 2,
                b'"' => {
                    in_string = false;
                    index += 1;
                }
                _ => index += 1,
            }
            continue;
        }
        match (byte, bytes.get(index + 1)) {
            (b'"', _) => {
                in_string = true;
                index += 1;
            }
            (b'/', Some(b'/')) => {
                while index < bytes.len() && bytes[index] != b'\n' {
                    blank(&mut out, index, &mut first_change);
                    index += 1;
                }
            }
            (b'/', Some(b'*')) => {
                let close = content[index + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |offset| index + 2 + offset + 2);
                for position in index..close {
                    blank(&mut out, position, &mut first_change);
                }
                index = close;
            }
            _ => index += 1,
        }
    }

    // Trailing commas: a comma whose next non-blank byte closes a container.
    let mut index = 0;
    let mut in_string = false;
    while index < out.len() {
        let byte = out[index];
        if in_string {
            match byte {
                b'\\' => index += 1,
                b'"' => in_string = false,
                _ => {}
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
        } else if byte == b',' {
            let next = out[index + 1..]
                .iter()
                .find(|candidate| !candidate.is_ascii_whitespace());
            if matches!(next, Some(b'}' | b']')) {
                out[index] = b' ';
                first_change = Some(first_change.map_or(index, |first| first.min(index)));
            }
        }
        index += 1;
    }

    // Only ASCII bytes were written, and comments were blanked whole, so the
    // result is still UTF-8.
    let stripped = String::from_utf8(out).unwrap_or_else(|_| content.to_string());
    (stripped, first_change)
}

/// A JSON value with its byte span.
#[derive(Debug)]
pub(crate) struct JsonNode {
    pub start: usize,
    pub end: usize,
    pub kind: JsonKind,
}

#[derive(Debug)]
pub(crate) enum JsonKind {
    Object(Vec<JsonMember>),
    Array(Vec<JsonNode>),
    Scalar,
}

/// An object member: the decoded key, the span of its quoted key, and the value.
#[derive(Debug)]
pub(crate) struct JsonMember {
    pub key: String,
    pub key_start: usize,
    pub key_end: usize,
    pub value: JsonNode,
}

impl JsonNode {
    /// Index the spans of `text`, which must be JSON that `serde_json` accepts
    /// (comments already blanked by [`strip_jsonc`]). Returns `None` on
    /// anything unexpected; callers then fall back to coarse locations.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parser = SpanParser {
            text,
            bytes: text.as_bytes(),
            index: 0,
        };
        parser.skip_whitespace();
        let node = parser.value(0)?;
        Some(node)
    }

    pub fn members(&self) -> &[JsonMember] {
        match &self.kind {
            JsonKind::Object(members) => members,
            _ => &[],
        }
    }

    pub fn items(&self) -> &[JsonNode] {
        match &self.kind {
            JsonKind::Array(items) => items,
            _ => &[],
        }
    }

    /// The member `serde_json` keeps for `key`: the last one, like V3's
    /// `JSON.parse`.
    pub fn member(&self, key: &str) -> Option<&JsonMember> {
        self.members().iter().rev().find(|member| member.key == key)
    }
}

struct SpanParser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    index: usize,
}

impl SpanParser<'_> {
    const MAX_DEPTH: usize = 256;

    fn skip_whitespace(&mut self) {
        while self.index < self.bytes.len() && self.bytes[self.index].is_ascii_whitespace() {
            self.index += 1;
        }
    }

    fn value(&mut self, depth: usize) -> Option<JsonNode> {
        if depth > Self::MAX_DEPTH {
            return None;
        }
        let start = self.index;
        match *self.bytes.get(self.index)? {
            b'{' => {
                self.index += 1;
                let mut members = Vec::new();
                loop {
                    self.skip_whitespace();
                    match *self.bytes.get(self.index)? {
                        b'}' => {
                            self.index += 1;
                            break;
                        }
                        b',' => {
                            self.index += 1;
                            continue;
                        }
                        b'"' => {}
                        _ => return None,
                    }
                    let key_start = self.index;
                    self.string()?;
                    let key_end = self.index;
                    let key =
                        serde_json::from_str::<String>(&self.text[key_start..key_end]).ok()?;
                    self.skip_whitespace();
                    if self.bytes.get(self.index) != Some(&b':') {
                        return None;
                    }
                    self.index += 1;
                    self.skip_whitespace();
                    let value = self.value(depth + 1)?;
                    members.push(JsonMember {
                        key,
                        key_start,
                        key_end,
                        value,
                    });
                }
                Some(JsonNode {
                    start,
                    end: self.index,
                    kind: JsonKind::Object(members),
                })
            }
            b'[' => {
                self.index += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_whitespace();
                    match *self.bytes.get(self.index)? {
                        b']' => {
                            self.index += 1;
                            break;
                        }
                        b',' => {
                            self.index += 1;
                            continue;
                        }
                        _ => {}
                    }
                    items.push(self.value(depth + 1)?);
                }
                Some(JsonNode {
                    start,
                    end: self.index,
                    kind: JsonKind::Array(items),
                })
            }
            b'"' => {
                self.string()?;
                Some(JsonNode {
                    start,
                    end: self.index,
                    kind: JsonKind::Scalar,
                })
            }
            _ => {
                while self.index < self.bytes.len()
                    && !matches!(self.bytes[self.index], b',' | b'}' | b']')
                    && !self.bytes[self.index].is_ascii_whitespace()
                {
                    self.index += 1;
                }
                (self.index > start).then_some(JsonNode {
                    start,
                    end: self.index,
                    kind: JsonKind::Scalar,
                })
            }
        }
    }

    /// Advance past a string starting at the current `"`.
    fn string(&mut self) -> Option<()> {
        self.index += 1;
        while self.index < self.bytes.len() {
            match self.bytes[self.index] {
                b'\\' => self.index += 2,
                b'"' => {
                    self.index += 1;
                    return Some(());
                }
                _ => self.index += 1,
            }
        }
        None
    }
}

/// Locations of keys and list items in Markdown agent frontmatter. YAML spans
/// are approximated line by line: block mappings and block sequences are
/// followed, flow collections resolve to their key.
pub(crate) struct FrontmatterIndex {
    /// `(byte offset of the line start, indent, line text)` for each line.
    lines: Vec<(usize, usize, String)>,
}

impl FrontmatterIndex {
    /// `content` is the whole file; the frontmatter is `content[start..end]`.
    pub fn new(content: &str, start: usize, end: usize) -> Self {
        let mut lines = Vec::new();
        let mut offset = start;
        for line in content[start..end].split_inclusive('\n') {
            let text = line.trim_end_matches(['\n', '\r']);
            let indent = text.len() - text.trim_start_matches(' ').len();
            lines.push((offset, indent, text.to_string()));
            offset += line.len();
        }
        Self { lines }
    }

    /// Byte offset of the best match for `path`: the deepest segment found.
    pub fn locate(&self, path: &[PathSegment<'_>]) -> Option<usize> {
        let (first, rest) = path.split_first()?;
        let PathSegment::Key(top) = first else {
            return None;
        };
        let top_line = self
            .lines
            .iter()
            .position(|(_, indent, text)| *indent == 0 && key_matches(text, top))?;
        let block_end = (top_line + 1..self.lines.len())
            .find(|&line| {
                let (_, indent, text) = &self.lines[line];
                let trimmed = text.trim_start();
                *indent == 0 && !trimmed.is_empty() && !trimmed.starts_with(['#', '-'])
            })
            .unwrap_or(self.lines.len());

        let mut best = self.lines[top_line].0;
        let mut cursor = top_line;
        let mut after_item = false;
        for segment in rest {
            match segment {
                PathSegment::Key(key) => {
                    let from = if after_item { cursor } else { cursor + 1 };
                    let Some(line) = (from..block_end).find(|&line| {
                        let text = self.lines[line].2.trim_start();
                        key_matches(text.strip_prefix("- ").unwrap_or(text).trim_start(), key)
                    }) else {
                        break;
                    };
                    let (start, indent, text) = &self.lines[line];
                    let trimmed = text.trim_start();
                    let dash = trimmed
                        .strip_prefix("- ")
                        .map_or(0, |rest| trimmed.len() - rest.trim_start().len());
                    best = start + indent + dash;
                    cursor = line;
                    after_item = false;
                }
                PathSegment::Index(index) => {
                    let mut dash_indent = None;
                    let mut seen = 0;
                    let mut hit = None;
                    for line in cursor + 1..block_end {
                        let (_, indent, text) = &self.lines[line];
                        let trimmed = text.trim_start();
                        if trimmed.is_empty() || trimmed.starts_with('#') {
                            continue;
                        }
                        if dash_indent.is_some_and(|dash| *indent < dash) {
                            break;
                        }
                        if trimmed.starts_with("- ") || trimmed == "-" {
                            if *dash_indent.get_or_insert(*indent) != *indent {
                                continue;
                            }
                            if seen == *index {
                                hit = Some(line);
                                break;
                            }
                            seen += 1;
                        } else if dash_indent.is_none() {
                            break;
                        }
                    }
                    let Some(line) = hit else {
                        break;
                    };
                    let (start, indent, _) = &self.lines[line];
                    best = start + indent + 2;
                    cursor = line;
                    after_item = true;
                }
            }
        }
        Some(best)
    }
}

/// Whether a YAML line (indentation removed) starts with `key:`, quoted or not.
fn key_matches(text: &str, key: &str) -> bool {
    let rest = ["\"", "'"]
        .iter()
        .find_map(|quote| {
            text.strip_prefix(quote)
                .and_then(|rest| rest.strip_prefix(key))
                .and_then(|rest| rest.strip_prefix(quote))
        })
        .or_else(|| text.strip_prefix(key));
    rest.is_some_and(|rest| rest.trim_start().starts_with(':'))
}

/// One step of a path into an agent document.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PathSegment<'a> {
    Key(&'a str),
    Index(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_jsonc_keeps_offsets_and_reports_first_change() {
        let content = "{\n  // note é\n  \"a\": \"//not a comment\", /* x */\n  \"b\": [1, 2,],\n}";
        let (stripped, first) = strip_jsonc(content);
        assert_eq!(stripped.len(), content.len());
        assert_eq!(first, Some(content.find("//").unwrap()));
        let value: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(value["a"], "//not a comment");
        assert_eq!(value["b"], serde_json::json!([1, 2]));
        assert_eq!(
            stripped.find("\"b\""),
            content.find("\"b\""),
            "offsets must not move"
        );
    }

    #[test]
    fn strip_jsonc_leaves_strict_json_untouched() {
        let content = r#"{"a": "x\"/*y*/", "b": [1, {"c": ","}]}"#;
        let (stripped, first) = strip_jsonc(content);
        assert_eq!(stripped, content);
        assert_eq!(first, None);
    }

    #[test]
    fn json_node_spans_keys_values_and_duplicates() {
        let content = r#"{ "name": "a", "hooks": {"stop": [{"command": "x"}]}, "name": "b" }"#;
        let node = JsonNode::parse(content).unwrap();
        assert_eq!(node.members().len(), 3);
        let name = node.member("name").unwrap();
        assert_eq!(&content[name.value.start..name.value.end], r#""b""#);
        let hooks = node.member("hooks").unwrap();
        assert_eq!(&content[hooks.key_start..hooks.key_end], r#""hooks""#);
        let stop = hooks.value.member("stop").unwrap();
        let command = stop.value.items()[0].member("command").unwrap();
        assert_eq!(&content[command.value.start..command.value.end], r#""x""#);
    }

    #[test]
    fn frontmatter_index_finds_nested_keys_and_items() {
        let content = "---\nname: a\ntools:\n  - read\n  - shell\npermissions:\n  rules:\n    - capability: shell\n      effect: allow\n    - capability: nope\n      effect: deny\n---\nBody";
        let end = content.rfind("\n---").unwrap();
        let index = FrontmatterIndex::new(content, 4, end);
        let at = |path: &[PathSegment<'_>]| {
            let offset = index.locate(path).unwrap();
            content[offset..].lines().next().unwrap().to_string()
        };
        assert_eq!(at(&[PathSegment::Key("name")]), "name: a");
        assert_eq!(
            at(&[PathSegment::Key("tools"), PathSegment::Index(1)]),
            "shell"
        );
        assert_eq!(
            at(&[
                PathSegment::Key("permissions"),
                PathSegment::Key("rules"),
                PathSegment::Index(1),
                PathSegment::Key("capability"),
            ]),
            "capability: nope"
        );
        assert_eq!(
            at(&[
                PathSegment::Key("permissions"),
                PathSegment::Key("rules"),
                PathSegment::Index(0),
                PathSegment::Key("effect"),
            ]),
            "effect: allow"
        );
    }

    #[test]
    fn tool_tables_classify_names() {
        assert_eq!(
            v2_tool_group("fsRead"),
            Some(&["read", "fs_read", "fsRead"][..])
        );
        assert_eq!(v2_tool_group("Read"), None);
        assert_eq!(case_variant_of_known_tool("Read"), Some("read"));
        assert_eq!(case_variant_of_known_tool("read"), None);
        assert_eq!(
            v3_replacement_for_v2_tool("fs_read"),
            Some(V3Replacement::Selector("read"))
        );
        assert_eq!(v3_replacement_for_v2_tool("read"), None);
        assert_eq!(
            v3_replacement_for_v2_tool("use_aws"),
            Some(V3Replacement::McpServer)
        );
        assert_eq!(v2_equivalent_for_v3_tool("grep_search"), Some("grep"));
        assert_eq!(claude_code_tool_replacement("Bash"), Some("shell"));
    }

    #[test]
    fn trigger_tables_follow_v3_alias_map() {
        assert_eq!(v2_trigger_for("PreToolUse"), Some("preToolUse"));
        assert_eq!(v2_trigger_for("SessionStart"), Some("agentSpawn"));
        assert_eq!(v2_trigger_for("AgentSpawn"), Some("agentSpawn"));
        assert_eq!(v2_trigger_for("PostFileSave"), None);
        assert_eq!(v2_trigger_for("beforePrompt"), None);
    }
}
