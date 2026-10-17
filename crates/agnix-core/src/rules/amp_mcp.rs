//! Shared pieces of the Amp validators: a diagnostic sink positioned by byte
//! offsets, the MCP server entry checks (AMP-006, AMP-008, AMP-010) and a
//! node tree for SKILL.md frontmatter.
//!
//! Amp reads MCP server entries from three places with the same fields:
//! `amp.mcpServers` in a settings file, a skill's sibling `mcp.json` and the
//! `mcpServers` field of SKILL.md frontmatter. "MCP servers use the same
//! configuration fields as MCP servers in skills—`command`/`args`/`env` for
//! local servers, `url`/`headers` for remote" (ampcode.com/docs/customize/mcp).
//! The settings schema (ampcode.com/cli-settings.schema.json) types them as
//! `stdioMCPServer` (`command` required, `args`, `env`) or `httpMCPServer`
//! (`url` required, `headers`, `transport`, `oauth`).

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel, Fix},
    schemas::kiro_json::{JsonMember, JsonNode, JsonValue, LineIndex},
};
use rust_i18n::t;
use serde_yaml::Value as YamlValue;
use std::ops::Range;
use std::path::Path;

/// The Amp ID token placeholder (ampcode.com/docs/customize/mcp, "Amp ID Tokens").
pub(crate) const ID_TOKEN: &str = "${amp:id-token}";

/// `transport` values of the settings schema's `httpMCPServer`.
const TRANSPORTS: &[&str] = &["http", "sse", "http-first"];

/// Required members of the settings schema's `httpMCPServer.oauth`.
const OAUTH_REQUIRED: &[&str] = &["clientId", "authUrl", "tokenUrl"];

/// Diagnostics for one file, positioned by byte offsets of its content.
pub(crate) struct Sink<'a, 'c> {
    pub path: &'a Path,
    lines: LineIndex<'a>,
    config: &'a PerFileLintConfig<'c>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a, 'c> Sink<'a, 'c> {
    pub fn new(path: &'a Path, content: &'a str, config: &'a PerFileLintConfig<'c>) -> Self {
        Self {
            path,
            lines: LineIndex::new(content),
            config,
            diagnostics: Vec::new(),
        }
    }

    pub fn enabled(&self, rule: &str) -> bool {
        self.config.is_rule_enabled(rule)
    }

    /// 1-based line and column of a byte offset.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        self.lines.line_col(offset)
    }

    /// Report `message` on `span` when `rule` is enabled.
    pub fn push(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        span: (usize, usize),
        message: String,
        suggestion: String,
    ) {
        self.push_with_fix(level, rule, span, message, suggestion, None);
    }

    pub fn push_with_fix(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        span: (usize, usize),
        message: String,
        suggestion: String,
        fix: Option<Fix>,
    ) {
        if !self.enabled(rule) {
            return;
        }
        let (line, column) = self.lines.line_col(span.0);
        let (end_line, end_column) = self.lines.line_col(span.1.max(span.0));
        let path = self.path.to_path_buf();
        let diagnostic = match level {
            DiagnosticLevel::Error => Diagnostic::error(path, line, column, rule, message),
            DiagnosticLevel::Warning => Diagnostic::warning(path, line, column, rule, message),
            DiagnosticLevel::Info => Diagnostic::info(path, line, column, rule, message),
        }
        .with_span(end_line, end_column)
        .with_suggestion(suggestion);
        self.diagnostics.push(match fix {
            Some(fix) => diagnostic.with_fix(fix),
            None => diagnostic,
        });
    }
}

/// Members of an object as `JSON.parse` keeps them: the last of each key.
pub(crate) fn last_members(members: &[JsonMember]) -> impl Iterator<Item = (usize, &JsonMember)> {
    members
        .iter()
        .enumerate()
        .filter(|(index, member)| !members[index + 1..].iter().any(|m| m.key == member.key))
}

/// JSON type name of a node, for messages.
pub(crate) fn type_name(node: &JsonNode) -> &'static str {
    node.value.type_name()
}

fn key_span(member: &JsonMember) -> (usize, usize) {
    (member.key_start, member.key_end)
}

fn value_span(node: &JsonNode) -> (usize, usize) {
    (node.start, node.end)
}

/// Where an MCP server map comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpOrigin {
    /// `amp.mcpServers` in a settings file.
    Settings,
    /// A skill's sibling `mcp.json`.
    SkillFile,
    /// The `mcpServers` field of SKILL.md frontmatter.
    SkillFrontmatter,
}

/// Check every server of an MCP server map (an object node keyed by server
/// name): entry shape (AMP-010), `${amp:id-token}` placement (AMP-006) and,
/// for skill servers, a missing `includeTools` (AMP-008).
pub(crate) fn check_servers(sink: &mut Sink<'_, '_>, servers: &[JsonMember], origin: McpOrigin) {
    for (_, server) in last_members(servers) {
        check_server(sink, server, origin);
    }
}

fn check_server(sink: &mut Sink<'_, '_>, server: &JsonMember, origin: McpOrigin) {
    let name = server.key.as_str();
    // In settings files the generic MCP server checks (entry shape, missing
    // command or url, argument and env types, secrets) also run on
    // `amp.mcpServers`; AMP-010 keeps only what Amp's schema adds there.
    let generic_shape = origin != McpOrigin::Settings;
    let Some(fields) = server.value.value.as_object() else {
        if !generic_shape {
            return;
        }
        sink.push(
            DiagnosticLevel::Warning,
            "AMP-010",
            value_span(&server.value),
            t!(
                "rules.amp_010.entry_not_object",
                server = name,
                found = type_name(&server.value)
            )
            .to_string(),
            t!("rules.amp_010.suggestion").to_string(),
        );
        return;
    };
    let get = |key: &str| fields.iter().rev().find(|member| member.key == key);
    let command = get("command");
    let url = get("url");
    match (command, url) {
        (None, None) if generic_shape => sink.push(
            DiagnosticLevel::Warning,
            "AMP-010",
            key_span(server),
            t!("rules.amp_010.no_transport", server = name).to_string(),
            t!("rules.amp_010.suggestion").to_string(),
        ),
        (Some(_), Some(_)) => sink.push(
            DiagnosticLevel::Warning,
            "AMP-010",
            key_span(server),
            t!("rules.amp_010.both_transports", server = name).to_string(),
            t!("rules.amp_010.suggestion").to_string(),
        ),
        _ => {}
    }

    for (_, field) in last_members(fields) {
        if generic_shape || matches!(field.key.as_str(), "transport" | "oauth") {
            check_field_type(sink, name, field);
        }
    }

    check_id_token(sink, name, fields, url);

    if origin != McpOrigin::Settings
        && (command.is_some() || url.is_some())
        && get("includeTools").is_none()
    {
        sink.push(
            DiagnosticLevel::Info,
            "AMP-008",
            key_span(server),
            t!("rules.amp_008.message", server = name).to_string(),
            t!("rules.amp_008.suggestion").to_string(),
        );
    }
}

/// AMP-010 type checks for the documented server fields.
fn check_field_type(sink: &mut Sink<'_, '_>, server: &str, field: &JsonMember) {
    let value = &field.value;
    let wrong_type = |expected: &str| {
        t!(
            "rules.amp_010.field_type",
            server = server,
            field = field.key.as_str(),
            expected = expected,
            found = type_name(value)
        )
        .to_string()
    };
    let message = match field.key.as_str() {
        "command" | "url" if value.value.as_str().is_none() => Some(wrong_type("a string")),
        "args" | "includeTools" if !is_string_array(value) => {
            Some(wrong_type("an array of strings"))
        }
        "env" | "headers" if !is_string_map(value) => Some(wrong_type("an object of strings")),
        "transport" => match value.value.as_str() {
            Some(transport) if TRANSPORTS.contains(&transport) => None,
            Some(transport) => Some(
                t!(
                    "rules.amp_010.transport",
                    server = server,
                    value = transport
                )
                .to_string(),
            ),
            None => Some(wrong_type("\"http\", \"sse\" or \"http-first\"")),
        },
        "oauth" => match value.value.as_object() {
            Some(oauth) => {
                let missing: Vec<&str> = OAUTH_REQUIRED
                    .iter()
                    .copied()
                    .filter(|key| {
                        !oauth.iter().any(|member| {
                            member.key == *key && member.value.value.as_str().is_some()
                        })
                    })
                    .collect();
                (!missing.is_empty()).then(|| {
                    t!(
                        "rules.amp_010.oauth_missing",
                        server = server,
                        fields = missing.join(", ")
                    )
                    .to_string()
                })
            }
            None => Some(wrong_type("an object")),
        },
        _ => None,
    };
    if let Some(message) = message {
        sink.push(
            DiagnosticLevel::Warning,
            "AMP-010",
            value_span(value),
            message,
            t!("rules.amp_010.suggestion").to_string(),
        );
    }
}

fn is_string_array(node: &JsonNode) -> bool {
    node.value
        .as_array()
        .is_some_and(|items| items.iter().all(|item| item.value.as_str().is_some()))
}

fn is_string_map(node: &JsonNode) -> bool {
    node.value.as_object().is_some_and(|members| {
        members
            .iter()
            .all(|member| member.value.value.as_str().is_some())
    })
}

fn contains_id_token(node: &JsonNode) -> bool {
    node.value
        .as_str()
        .is_some_and(|text| text.contains(ID_TOKEN))
}

/// Where `${amp:id-token}` was found.
#[derive(Clone, Copy)]
enum TokenPlace {
    Url,
    Command,
    Args,
    Env,
    HeadersNotHttps,
    HeadersOauth,
}

fn id_token_message(server: &str, place: TokenPlace) -> String {
    match place {
        TokenPlace::Url => t!("rules.amp_006.url", server = server),
        TokenPlace::Command => t!("rules.amp_006.command", server = server),
        TokenPlace::Args => t!("rules.amp_006.args", server = server),
        TokenPlace::Env => t!("rules.amp_006.env", server = server),
        TokenPlace::HeadersNotHttps => t!("rules.amp_006.headers_not_https", server = server),
        TokenPlace::HeadersOauth => t!("rules.amp_006.headers_oauth", server = server),
    }
    .to_string()
}

/// AMP-006: "`${amp:id-token}` works only in headers of HTTPS servers without
/// OAuth configuration. Amp rejects it in URLs and in the command,
/// arguments, and environment of local servers."
fn check_id_token(
    sink: &mut Sink<'_, '_>,
    server: &str,
    fields: &[JsonMember],
    url: Option<&JsonMember>,
) {
    let report = |sink: &mut Sink<'_, '_>, node: &JsonNode, place: TokenPlace| {
        sink.push(
            DiagnosticLevel::Error,
            "AMP-006",
            value_span(node),
            id_token_message(server, place),
            t!("rules.amp_006.suggestion").to_string(),
        );
    };
    let has_oauth = fields.iter().any(|member| member.key == "oauth");
    // A literal URL that is not https. A URL built from a variable
    // (`${SRC_ENDPOINT}/...`) may resolve to https, so it is not judged.
    let not_https = url
        .and_then(|member| member.value.value.as_str())
        .is_some_and(|url| {
            let url = url.trim();
            !url.starts_with("${")
                && !url
                    .get(..8)
                    .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
        });
    for (_, field) in last_members(fields) {
        let value = &field.value;
        match field.key.as_str() {
            "url" if contains_id_token(value) => report(sink, value, TokenPlace::Url),
            "command" if contains_id_token(value) => report(sink, value, TokenPlace::Command),
            "args" => {
                for item in value.value.as_array().unwrap_or_default() {
                    if contains_id_token(item) {
                        report(sink, item, TokenPlace::Args);
                    }
                }
            }
            "env" => {
                for member in value.value.as_object().unwrap_or_default() {
                    if contains_id_token(&member.value) {
                        report(sink, &member.value, TokenPlace::Env);
                    }
                }
            }
            "headers" if has_oauth || not_https => {
                let place = if has_oauth {
                    TokenPlace::HeadersOauth
                } else {
                    TokenPlace::HeadersNotHttps
                };
                for member in value.value.as_object().unwrap_or_default() {
                    if contains_id_token(&member.value) {
                        report(sink, &member.value, place);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Byte span that deletes `members[index]` from its object and leaves valid
/// JSON: the member and the comma after it, or for the last member the comma
/// before it. Whole lines are removed when the member sits alone on its line.
/// `None` when the key repeats (deleting one occurrence would change which
/// value wins).
pub(crate) fn member_deletion_span(
    content: &str,
    members: &[JsonMember],
    index: usize,
) -> Option<(usize, usize)> {
    let member = members.get(index)?;
    if members.iter().filter(|m| m.key == member.key).count() > 1 {
        return None;
    }
    let line_start = |offset: usize| {
        let start = content[..offset].rfind('\n').map_or(0, |i| i + 1);
        content[start..offset]
            .trim_matches([' ', '\t'])
            .is_empty()
            .then_some(start)
    };
    if let Some(next) = members.get(index + 1) {
        let start = line_start(member.key_start).unwrap_or(member.key_start);
        let end = line_start(next.key_start).unwrap_or(next.key_start);
        return Some((start, end));
    }
    if index > 0 {
        return Some((members[index - 1].value.end, member.value.end));
    }
    // The only member: remove it and a trailing comma.
    let mut end = member.value.end;
    let rest = &content[end..];
    let trimmed = rest.trim_start_matches([' ', '\t']);
    if trimmed.starts_with(',') {
        end += rest.len() - trimmed.len() + 1;
    }
    Some((member.key_start, end))
}

// ---------------------------------------------------------------------------
// YAML frontmatter as a node tree
// ---------------------------------------------------------------------------

/// One line of `content`: byte range without the newline, and indentation.
struct Line {
    start: usize,
    end: usize,
    indent: usize,
}

fn lines_in(content: &str, range: Range<usize>) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = range.start;
    while start < range.end {
        let end = content[start..range.end]
            .find('\n')
            .map_or(range.end, |i| start + i);
        let text = &content[start..end];
        let indent = text.len() - text.trim_start_matches([' ', '\t']).len();
        lines.push(Line { start, end, indent });
        start = end + 1;
    }
    lines
}

/// Convert a YAML value from SKILL.md frontmatter into a node tree whose
/// spans point into `content`. serde_yaml keeps no positions, so each
/// mapping key is found as text on the lines of its parent block (block
/// style, at the block's indentation) and a value gets the span of the text
/// after its key. Values that cannot be located (flow style such as
/// `{url: x}`) get the span of `fallback`.
pub(crate) fn yaml_node(
    content: &str,
    value: &YamlValue,
    block: Range<usize>,
    fallback: (usize, usize),
) -> JsonNode {
    let (start, end) = fallback;
    let value = match value {
        YamlValue::Mapping(mapping) => {
            let lines = lines_in(content, block.clone());
            let indent = lines
                .iter()
                .find(|line| {
                    let text = content[line.start..line.end].trim();
                    !text.is_empty() && !text.starts_with('#')
                })
                .map(|line| line.indent);
            let mut members = Vec::new();
            let mut cursor = 0;
            for (key, child) in mapping {
                let key = yaml_key_text(key);
                let found = indent.and_then(|indent| {
                    (cursor..lines.len()).find(|&index| {
                        let line = &lines[index];
                        line.indent == indent
                            && key_at(&content[line.start + line.indent..line.end], &key).is_some()
                    })
                });
                let member = match found {
                    Some(index) => {
                        cursor = index + 1;
                        let line = &lines[index];
                        let text_start = line.start + line.indent;
                        let key_len = key_at(&content[text_start..line.end], &key).unwrap_or(0);
                        let key_span = (text_start, text_start + key_len);
                        let after = text_start + key_len;
                        // The value starts after the colon on the same line,
                        // or is the block of deeper lines that follows.
                        let inline = content[after..line.end]
                            .trim_start_matches([' ', '\t', ':'])
                            .trim_end();
                        let block_end = lines[index + 1..]
                            .iter()
                            .find(|next| {
                                let text = content[next.start..next.end].trim();
                                !text.is_empty() && next.indent <= line.indent
                            })
                            .map_or(block.end, |next| next.start);
                        let child_block = (line.end + 1).min(block_end)..block_end;
                        let value_span = if inline.is_empty() || inline.starts_with('#') {
                            key_span
                        } else {
                            let rest =
                                content[after..line.end].trim_start_matches([' ', '\t', ':']);
                            let value_start = line.end - rest.len();
                            (value_start, value_start + inline_value_len(rest))
                        };
                        JsonMember {
                            key: key.clone(),
                            key_start: key_span.0,
                            key_end: key_span.1,
                            value: yaml_node(content, child, child_block, value_span),
                        }
                    }
                    None => JsonMember {
                        key: key.clone(),
                        key_start: start,
                        key_end: end,
                        value: yaml_node(content, child, block.end..block.end, fallback),
                    },
                };
                members.push(member);
            }
            JsonValue::Object(members)
        }
        YamlValue::Sequence(items) => JsonValue::Array(
            items
                .iter()
                .map(|item| yaml_node(content, item, block.end..block.end, fallback))
                .collect(),
        ),
        YamlValue::String(text) => JsonValue::String(text.clone()),
        YamlValue::Bool(flag) => JsonValue::Bool(*flag),
        YamlValue::Number(number) => number
            .as_i64()
            .map(serde_json::Number::from)
            .or_else(|| number.as_u64().map(serde_json::Number::from))
            .or_else(|| number.as_f64().and_then(serde_json::Number::from_f64))
            .map_or(JsonValue::Null, JsonValue::Number),
        YamlValue::Null => JsonValue::Null,
        YamlValue::Tagged(tagged) => return yaml_node(content, &tagged.value, block, fallback),
    };
    JsonNode { start, end, value }
}

/// Length of an inline YAML value without a trailing comment: a quoted
/// scalar through its closing quote, otherwise the text before ` #`.
fn inline_value_len(text: &str) -> usize {
    match text.chars().next() {
        Some('"') => {
            let mut escaped = false;
            for (index, c) in text.char_indices().skip(1) {
                match c {
                    '\\' if !escaped => escaped = true,
                    '"' if !escaped => return index + 1,
                    _ => escaped = false,
                }
            }
            text.trim_end().len()
        }
        Some('\'') => {
            let bytes = text.as_bytes();
            let mut index = 1;
            while index < bytes.len() {
                if bytes[index] == b'\'' {
                    if bytes.get(index + 1) == Some(&b'\'') {
                        index += 2;
                        continue;
                    }
                    return index + 1;
                }
                index += 1;
            }
            text.trim_end().len()
        }
        _ => {
            let end = text
                .char_indices()
                .find(|&(index, c)| c == '#' && text[..index].ends_with([' ', '\t']))
                .map_or(text.len(), |(index, _)| index);
            text[..end].trim_end().len()
        }
    }
}

/// A mapping key as text (non-string keys as their YAML form).
fn yaml_key_text(key: &YamlValue) -> String {
    match key {
        YamlValue::String(text) => text.clone(),
        other => serde_yaml::to_string(other)
            .map(|text| text.trim().to_string())
            .unwrap_or_default(),
    }
}

/// Length of `key` (bare or quoted) at the start of `text` when a colon
/// follows it.
fn key_at(text: &str, key: &str) -> Option<usize> {
    for quote in ["", "\"", "'"] {
        let Some(rest) = text.strip_prefix(quote) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(key) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(quote) else {
            continue;
        };
        if rest.trim_start_matches([' ', '\t']).starts_with(':') {
            return Some(quote.len() * 2 + key.len());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::kiro_json::parse_json_document;

    #[test]
    fn deletion_spans_keep_json_valid() {
        let cases = [
            "{\n  \"a\": 1,\n  \"bad\": true,\n  \"c\": 2\n}",
            "{\n  \"a\": 1,\n  \"bad\": true\n}",
            "{\n  \"bad\": [1, 2]\n}",
            "{\"bad\": 1, \"a\": 2}",
            "{\"a\": 2, \"bad\": {\"x\": 1}}",
            "{\n  // note\n  \"a\": 1, // tail\n  \"bad\": true,\n}\n",
        ];
        for content in cases {
            let doc = parse_json_document(content, true).unwrap();
            let members = doc.root.value.as_object().unwrap();
            let index = members.iter().position(|m| m.key == "bad").unwrap();
            let (start, end) = member_deletion_span(content, members, index).unwrap();
            let mut fixed = content.to_string();
            fixed.replace_range(start..end, "");
            let doc = parse_json_document(&fixed, true)
                .unwrap_or_else(|e| panic!("{content:?} -> {fixed:?}: {e:?}"));
            let keys: Vec<_> = doc
                .root
                .value
                .as_object()
                .unwrap()
                .iter()
                .map(|m| m.key.as_str())
                .collect();
            assert!(!keys.contains(&"bad"), "{fixed:?}");
            assert_eq!(keys.len(), members.len() - 1, "{fixed:?}");
            if !content.contains("//") && !content.contains(",\n}") {
                assert!(
                    parse_json_document(&fixed, false).is_ok(),
                    "strict JSON stays strict: {fixed:?}"
                );
            }
        }
    }

    #[test]
    fn deletion_span_skips_repeated_keys() {
        let content = r#"{"bad": 1, "bad": 2}"#;
        let doc = parse_json_document(content, false).unwrap();
        let members = doc.root.value.as_object().unwrap();
        assert!(member_deletion_span(content, members, 0).is_none());
    }

    #[test]
    fn yaml_node_finds_block_keys() {
        let content = "---\nname: x\nmcpServers:\n  linear:\n    url: https://mcp.linear.app/mcp\n    env:\n      TOKEN: abc\n  inline: {command: npx}\n---\n";
        let yaml: YamlValue = serde_yaml::from_str(&content[4..content.len() - 4]).unwrap();
        let node = yaml_node(content, &yaml, 4..content.len() - 4, (0, 3));
        let servers = node.value.get("mcpServers").unwrap();
        let linear = servers.value.value.get("linear").unwrap();
        assert_eq!(&content[linear.key_start..linear.key_end], "linear");
        let url = linear.value.value.get("url").unwrap();
        assert_eq!(
            &content[url.value.start..url.value.end],
            "https://mcp.linear.app/mcp"
        );
        let env = linear.value.value.get("env").unwrap();
        let token = env.value.value.get("TOKEN").unwrap();
        assert_eq!(&content[token.key_start..token.key_end], "TOKEN");
        // Flow style: the inner key is not located and falls back to the
        // span of the text after `inline:`.
        let inline = servers.value.value.get("inline").unwrap();
        let command = inline.value.value.get("command").unwrap();
        assert_eq!(
            &content[command.key_start..command.key_end],
            "{command: npx}"
        );
    }

    #[test]
    fn inline_values_drop_comments() {
        assert_eq!(inline_value_len("high # note"), 4);
        assert_eq!(inline_value_len("\"a # b\" # c"), 7);
        assert_eq!(inline_value_len("'it''s' # c"), 7);
        assert_eq!(inline_value_len("a#b"), 3);
        assert_eq!(inline_value_len("plain  "), 5);
    }

    #[test]
    fn key_at_accepts_quotes() {
        assert_eq!(key_at("url: x", "url"), Some(3));
        assert_eq!(key_at("\"url\": x", "url"), Some(5));
        assert_eq!(key_at("'url' : x", "url"), Some(5));
        assert_eq!(key_at("urls: x", "url"), None);
    }
}
