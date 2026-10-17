//! Kiro MCP validation rules (KR-MCP-001 to KR-MCP-015).
//!
//! Validates three flavors of Kiro `mcp.json`:
//!
//! - `.kiro/settings/mcp.json` and `~/.kiro/settings/mcp.json`. Kiro CLI 2.x
//!   (the V2 engine) parses them with strict serde_json: a syntax error or one
//!   wrongly typed field anywhere rejects the WHOLE file. The V3 engine parses
//!   them with comment-json (comments and trailing commas allowed) and checks
//!   each server separately: an invalid server is skipped, the others load.
//! - A legacy power's `mcp.json` (next to `POWER.md`): same format, V3 only,
//!   read with `JSON.parse`; KR-PW-004 on the POWER.md reports a file that
//!   does not parse or lacks `mcpServers`, so this validator does not.
//! - An Agent Plugins power's `mcp.json` (next to `plugin.json`): strict JSON
//!   and a strict schema (V3 only). An unknown top-level key disables every
//!   server of the power; an invalid server entry is skipped.
//!
//! Rules:
//! - KR-MCP-001: file cannot be parsed, `mcpServers` missing, or a server has
//!   neither `command` nor `url`
//! - KR-MCP-002: hardcoded secret in `env`, `headers` or `args`
//! - KR-MCP-003: `command` contains its arguments
//! - KR-MCP-004: invalid remote `url`
//! - KR-MCP-005: server defined twice in one file
//! - KR-MCP-006: invalid OAuth configuration
//! - KR-MCP-007: server field has the wrong type or value
//! - KR-MCP-008: remote `url` uses http for a non-local host (V3 rejects)
//! - KR-MCP-009: variable reference Kiro does not expand
//! - KR-MCP-010: unknown server field
//! - KR-MCP-011: server has both `command` and `url`
//! - KR-MCP-012: Agent Plugins `mcp.json` `$schema` missing or mismatched
//! - KR-MCP-013: Agent Plugins `mcp.json` unknown top-level field
//! - KR-MCP-014: invalid Agent Plugins MCP server entry
//! - KR-MCP-015: `autoApprove` wildcard
//!
//! Sources: spec section 3 (kiro.dev/docs/mcp/configuration, the V3 engine
//! schema and loaders) and probes of `kiro-cli mcp list` 2.32.1.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    rules::{
        Validator, ValidatorMetadata,
        kiro_engine::{KiroEngine, KiroEngines, engines_for},
        seems_plaintext_secret,
    },
    schemas::kiro_json::{
        JsonDialect, JsonMember, JsonNode, JsonValue, LineIndex, closest_name, parse_json_document,
    },
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "KR-MCP-001",
    "KR-MCP-002",
    "KR-MCP-003",
    "KR-MCP-004",
    "KR-MCP-005",
    "KR-MCP-006",
    "KR-MCP-007",
    "KR-MCP-008",
    "KR-MCP-009",
    "KR-MCP-010",
    "KR-MCP-011",
    "KR-MCP-012",
    "KR-MCP-013",
    "KR-MCP-014",
    "KR-MCP-015",
];

/// Server fields read by Kiro CLI 2.x or the V3 engine, plus fields Kiro
/// documents (`requestTimeout`) or Kiro IDE reads (`autoApprove`).
const KNOWN_SERVER_FIELDS: &[&str] = &[
    "command",
    "args",
    "env",
    "cwd",
    "url",
    "headers",
    "type",
    "timeout",
    "requestTimeout",
    "disabled",
    "disabledTools",
    "autoApprove",
    "waitForReady",
    "versionNegotiation",
    "oauth",
    "oauthScopes",
];

pub struct KiroMcpValidator;

impl Validator for KiroMcpValidator {
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
        let mut cx = Cx::new(path, content, config);
        match mcp_variant(path, content, config) {
            McpVariant::AgentPlugin { manifest_schema } => {
                cx.engines = KiroEngines::V3_ONLY;
                validate_plugin_mcp(&mut cx, manifest_schema.as_deref());
            }
            McpVariant::LegacyPower { power_md } => {
                // Powers exist only in the V3 engine, which reads a legacy
                // power's mcp.json with JSON.parse. When the power has a
                // POWER.md, KR-PW-004 already reports a file that does not
                // parse or lacks mcpServers, so this validator does not repeat it.
                cx.engines = KiroEngines::V3_ONLY;
                validate_kiro_mcp(&mut cx, false, !power_md);
            }
            McpVariant::Settings => {
                // mcp.json carries no reliable engine markers: the same file is
                // read by whichever engine runs, so check both unless pinned.
                cx.engines = engines_for(config, false, false);
                validate_kiro_mcp(&mut cx, true, true);
            }
        }
        cx.diagnostics
    }
}

/// Which Kiro loader reads this `mcp.json`.
#[derive(Debug, PartialEq, Eq)]
enum McpVariant {
    /// `.kiro/settings/mcp.json` (user or workspace).
    Settings,
    /// `<power>/mcp.json` of a legacy power: next to POWER.md (`power_md`) or
    /// inside `.kiro/powers/`.
    LegacyPower { power_md: bool },
    /// `<power>/mcp.json` next to plugin.json. `manifest_schema` is the
    /// plugin.json `$schema`, when it can be read.
    AgentPlugin { manifest_schema: Option<String> },
}

fn mcp_variant(path: &Path, content: &str, config: &PerFileLintConfig<'_>) -> McpVariant {
    let is_settings_file = path
        .parent()
        .and_then(|p| p.file_name())
        .is_some_and(|name| name.eq_ignore_ascii_case("settings"))
        && path
            .parent()
            .and_then(Path::parent)
            .and_then(|p| p.file_name())
            .is_some_and(|name| name.eq_ignore_ascii_case(".kiro"));
    if is_settings_file {
        return McpVariant::Settings;
    }
    let fs = config.fs();
    if let Some(dir) = path.parent() {
        let manifest = dir.join("plugin.json");
        if fs.exists(&manifest) {
            let manifest_schema = fs.read_to_string(&manifest).ok().and_then(|text| {
                serde_json::from_str::<serde_json::Value>(&text)
                    .ok()?
                    .get("$schema")?
                    .as_str()
                    .map(str::to_string)
            });
            return McpVariant::AgentPlugin { manifest_schema };
        }
        let power_md = fs.exists(&dir.join("POWER.md"));
        if power_md || path_is_under_kiro_powers(path) {
            return McpVariant::LegacyPower { power_md };
        }
    }
    // Without a manifest on disk, an Agent Plugins `$schema` still identifies
    // the plugin format.
    if serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|v| {
            v.get("$schema")
                .and_then(|s| s.as_str())
                .map(is_plugin_mcp_schema)
        })
        .unwrap_or(false)
    {
        return McpVariant::AgentPlugin {
            manifest_schema: None,
        };
    }
    McpVariant::Settings
}

fn path_is_under_kiro_powers(path: &Path) -> bool {
    let names: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect();
    names
        .windows(2)
        .any(|w| w[0] == ".kiro" && w[1] == "powers")
}

/// Shared state for one file.
struct Cx<'a> {
    path: &'a Path,
    content: &'a str,
    lines: LineIndex<'a>,
    config: &'a PerFileLintConfig<'a>,
    engines: KiroEngines,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Cx<'a> {
    fn new(path: &'a Path, content: &'a str, config: &'a PerFileLintConfig<'a>) -> Self {
        Self {
            path,
            content,
            lines: LineIndex::new(content),
            config,
            engines: KiroEngines::BOTH,
            diagnostics: Vec::new(),
        }
    }

    fn enabled(&self, rule: &str) -> bool {
        self.config.is_rule_enabled(rule)
    }

    /// Push a diagnostic covering `start..end` of the source.
    fn push(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        start: usize,
        end: usize,
        message: String,
        suggestion: String,
    ) {
        let (line, column) = self.lines.line_col(start);
        let (end_line, end_column) = self.lines.line_col(end.max(start));
        let path = self.path.to_path_buf();
        let diagnostic = match level {
            DiagnosticLevel::Error => Diagnostic::error(path, line, column, rule, message),
            DiagnosticLevel::Warning => Diagnostic::warning(path, line, column, rule, message),
            DiagnosticLevel::Info => Diagnostic::info(path, line, column, rule, message),
        };
        self.diagnostics.push(
            diagnostic
                .with_span(end_line, end_column)
                .with_suggestion(suggestion),
        );
    }
}

/// Which engines a condition is fatal in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Fatal {
    v2: bool,
    v3: bool,
}

impl Fatal {
    const BOTH: Self = Self { v2: true, v3: true };
    const V2: Self = Self {
        v2: true,
        v3: false,
    };
    const V3: Self = Self {
        v2: false,
        v3: true,
    };
}

/// Severity of a load failure for the targeted engines: an error when every
/// targeted engine fails, a warning naming the engine when only one does,
/// nothing when no targeted engine is affected.
fn fatal_level(engines: KiroEngines, fatal: Fatal) -> Option<(DiagnosticLevel, Fatal)> {
    let hit = Fatal {
        v2: engines.v2 && fatal.v2,
        v3: engines.v3 && fatal.v3,
    };
    match (hit.v2, hit.v3) {
        (false, false) => None,
        (true, true) => Some((DiagnosticLevel::Error, hit)),
        (true, false) if engines.only(KiroEngine::V2) => Some((DiagnosticLevel::Error, hit)),
        (false, true) if engines.only(KiroEngine::V3) => Some((DiagnosticLevel::Error, hit)),
        _ => Some((DiagnosticLevel::Warning, hit)),
    }
}

/// Localized "what Kiro does" phrase for a field-level load failure.
fn server_effect(hit: Fatal) -> String {
    match (hit.v2, hit.v3) {
        (true, true) => t!("rules.kr_mcp_007.effects.both").to_string(),
        (true, false) => t!("rules.kr_mcp_007.effects.v2").to_string(),
        _ => t!("rules.kr_mcp_007.effects.v3").to_string(),
    }
}

// ---------------------------------------------------------------------------
// Settings and legacy power mcp.json
// ---------------------------------------------------------------------------

/// `jsonc`: comments and trailing commas are accepted (V3 settings files).
/// `report_file`: report parse and `mcpServers` shape problems (KR-MCP-001).
fn validate_kiro_mcp(cx: &mut Cx<'_>, jsonc: bool, report_file: bool) {
    let doc = match parse_json_document(cx.content, jsonc) {
        Ok(doc) => doc,
        Err(error) => {
            if report_file && cx.enabled("KR-MCP-001") {
                let offset = offset_of(cx.content, error.line, error.column);
                cx.push(
                    DiagnosticLevel::Error,
                    "KR-MCP-001",
                    offset,
                    offset,
                    t!(
                        "rules.kr_mcp_001_parse.message",
                        error = error.message.as_str()
                    )
                    .to_string(),
                    t!("rules.kr_mcp_001_parse.suggestion").to_string(),
                );
            }
            return;
        }
    };

    // Comments and trailing commas: accepted by the V3 engine (comment-json),
    // rejected by Kiro CLI 2.x, which then loads no server from this file.
    if let JsonDialect::Jsonc { offset } = doc.dialect
        && cx.enabled("KR-MCP-001")
        && let Some((level, _)) = fatal_level(cx.engines, Fatal::V2)
    {
        let strict = doc
            .strict_error
            .as_ref()
            .map(|e| e.message.clone())
            .unwrap_or_default();
        cx.push(
            level,
            "KR-MCP-001",
            offset,
            offset,
            t!("rules.kr_mcp_001_jsonc.message", error = strict.as_str()).to_string(),
            t!("rules.kr_mcp_001_jsonc.suggestion").to_string(),
        );
    }

    let root = &doc.root;
    let servers = match root.value.get("mcpServers") {
        Some(member) => member,
        None => {
            if report_file && cx.enabled("KR-MCP-001") {
                cx.push(
                    DiagnosticLevel::Error,
                    "KR-MCP-001",
                    root.start,
                    root.start,
                    t!("rules.kr_mcp_001_root.message").to_string(),
                    t!("rules.kr_mcp_001_root.suggestion").to_string(),
                );
            }
            return;
        }
    };
    let Some(members) = servers.value.value.as_object() else {
        if report_file && cx.enabled("KR-MCP-001") {
            cx.push(
                DiagnosticLevel::Error,
                "KR-MCP-001",
                servers.value.start,
                servers.value.end,
                t!(
                    "rules.kr_mcp_001_root_type.message",
                    found = servers.value.value.type_name()
                )
                .to_string(),
                t!("rules.kr_mcp_001_root.suggestion").to_string(),
            );
        }
        return;
    };

    for (index, member) in members.iter().enumerate() {
        // A repeated name: JSON keeps the last definition.
        if let Some(later) = members[index + 1..].iter().find(|m| m.key == member.key) {
            if cx.enabled("KR-MCP-005") {
                let (later_line, _) = cx.lines.line_col(later.key_start);
                cx.push(
                    DiagnosticLevel::Warning,
                    "KR-MCP-005",
                    member.key_start,
                    member.key_end,
                    t!(
                        "rules.kr_mcp_005.message",
                        server = member.key.as_str(),
                        line = later_line
                    )
                    .to_string(),
                    t!("rules.kr_mcp_005.suggestion").to_string(),
                );
            }
            continue;
        }
        validate_server(cx, member);
    }
}

/// Byte offset of a 1-based (line, column) position, clamped to the text.
fn offset_of(content: &str, line: usize, column: usize) -> usize {
    let mut offset = 0;
    for (index, text) in content.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            let col_offset = text
                .char_indices()
                .nth(column.saturating_sub(1))
                .map_or(text.len(), |(i, _)| i);
            return (offset + col_offset).min(content.len());
        }
        offset += text.len();
    }
    content.len()
}

fn validate_server(cx: &mut Cx<'_>, member: &JsonMember) {
    let name = member.key.as_str();
    let node = &member.value;
    let Some(fields) = node.value.as_object() else {
        if cx.enabled("KR-MCP-007")
            && let Some((level, hit)) = fatal_level(cx.engines, Fatal::BOTH)
        {
            cx.push(
                level,
                "KR-MCP-007",
                node.start,
                node.end,
                t!(
                    "rules.kr_mcp_007.server_message",
                    server = name,
                    found = node.value.type_name(),
                    effect = server_effect(hit).as_str()
                )
                .to_string(),
                t!("rules.kr_mcp_007.suggestion").to_string(),
            );
        }
        return;
    };

    // Field types (KR-MCP-007). Remember which transport fields are unusable
    // so KR-MCP-001 does not report the same mistake twice.
    let mut broken_transport_field = false;
    for field in fields {
        if let Some(issue) = field_type_issue(&field.key, &field.value.value) {
            if matches!(field.key.as_str(), "command" | "url") {
                broken_transport_field = true;
            }
            report_type_issue(cx, name, &field.key, &field.value, issue);
        }
        if field.key == "oauth"
            && let Some(oauth_fields) = field.value.value.as_object()
        {
            for oauth_field in oauth_fields {
                if let Some(issue) =
                    oauth_field_type_issue(&oauth_field.key, &oauth_field.value.value)
                {
                    let qualified = format!("oauth.{}", oauth_field.key);
                    report_type_issue(cx, name, &qualified, &oauth_field.value, issue);
                }
            }
        }
    }

    let get = |key: &str| node.value.get(key);
    let disabled = get("disabled").and_then(|m| m.value.value.as_bool()) == Some(true);
    let registry = get("type").and_then(|m| m.value.value.as_str()) == Some("registry");
    let command = get("command").and_then(|m| m.value.value.as_str().map(|s| (m, s)));
    let url = get("url").and_then(|m| m.value.value.as_str().map(|s| (m, s)));
    let has_command = command.is_some_and(|(_, s)| !s.trim().is_empty());
    let has_url = url.is_some_and(|(_, s)| !s.trim().is_empty());

    // KR-MCP-001: no transport. A disabled server is never started, and
    // "registry" servers are resolved by Kiro.
    if !has_command
        && !has_url
        && !disabled
        && !registry
        && !broken_transport_field
        && cx.enabled("KR-MCP-001")
    {
        cx.push(
            DiagnosticLevel::Error,
            "KR-MCP-001",
            member.key_start,
            member.key_end,
            t!("rules.kr_mcp_001.message", server = name).to_string(),
            t!("rules.kr_mcp_001.suggestion").to_string(),
        );
    }

    // KR-MCP-011: command wins over url in the V3 engine.
    if has_command
        && has_url
        && cx.engines.includes(KiroEngine::V3)
        && cx.enabled("KR-MCP-011")
        && let Some((url_member, _)) = url
    {
        cx.push(
            DiagnosticLevel::Info,
            "KR-MCP-011",
            url_member.key_start,
            url_member.value.end,
            t!("rules.kr_mcp_011.message", server = name).to_string(),
            t!("rules.kr_mcp_011.suggestion").to_string(),
        );
    }

    // KR-MCP-003: arguments embedded in command.
    if let Some((command_member, command_text)) = command
        && command_has_embedded_args(command_text)
        && cx.enabled("KR-MCP-003")
    {
        cx.push(
            DiagnosticLevel::Warning,
            "KR-MCP-003",
            command_member.value.start,
            command_member.value.end,
            t!(
                "rules.kr_mcp_003.message",
                server = name,
                command = command_text.trim()
            )
            .to_string(),
            t!("rules.kr_mcp_003.suggestion").to_string(),
        );
    }

    // URL checks apply when url is the transport (command wins otherwise).
    let http_transport = has_url && !has_command && !disabled && !registry;
    if http_transport && let Some((url_member, url_text)) = url {
        check_remote_url(cx, name, url_member, url_text);
    }

    check_references(cx, name, node);
    check_secrets(cx, name, node);
    check_auto_approve(cx, name, node);
    if let Some(oauth) = get("oauth") {
        // OAuth applies to a remote http(s) server; the V3 engine validates
        // its URLs only when the server is enabled.
        let remote = has_url
            && !has_command
            && url.is_some_and(|(_, u)| {
                let lower = u.trim().to_ascii_lowercase();
                lower.starts_with("https://")
                    || lower.starts_with("http://")
                    || lower.contains("${")
            });
        check_oauth(cx, name, oauth, remote, http_transport);
    }

    if cx.enabled("KR-MCP-010") {
        for field in fields {
            if KNOWN_SERVER_FIELDS.contains(&field.key.as_str()) {
                continue;
            }
            let suggestion = match closest_name(&field.key, KNOWN_SERVER_FIELDS) {
                Some(close) => t!("rules.kr_mcp_010.did_you_mean", field = close).to_string(),
                None => t!("rules.kr_mcp_010.suggestion").to_string(),
            };
            cx.push(
                DiagnosticLevel::Info,
                "KR-MCP-010",
                field.key_start,
                field.key_end,
                t!(
                    "rules.kr_mcp_010.message",
                    server = name,
                    field = field.key.as_str()
                )
                .to_string(),
                suggestion,
            );
        }
    }
}

/// A field value Kiro cannot load: which engines fail and what is expected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TypeIssue {
    fatal: Fatal,
    expected: &'static str,
}

fn is_string_array(value: &JsonValue) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().all(|i| i.value.as_str().is_some()))
}

fn is_string_map(value: &JsonValue) -> bool {
    value
        .as_object()
        .is_some_and(|members| members.iter().all(|m| m.value.value.as_str().is_some()))
}

/// Server field types. V2 behavior was observed with `kiro-cli mcp list`
/// 2.32.1 (one bad field fails the whole file; `null` fails except for `env`,
/// `type` and `oauth`); V3 follows the engine's zod schema, where `null` never
/// passes an optional field. Fields V2 does not read (`cwd`, `waitForReady`,
/// `versionNegotiation`, `autoApprove`) are ignored by V2 whatever their type.
fn field_type_issue(key: &str, value: &JsonValue) -> Option<TypeIssue> {
    let issue = |fatal, expected| Some(TypeIssue { fatal, expected });
    match key {
        "command" | "url" if value.as_str().is_none() => issue(Fatal::BOTH, "a string"),
        "headers" if !is_string_map(value) => issue(Fatal::BOTH, "an object of strings"),
        "env" if value.is_null() => issue(Fatal::V3, "an object of strings"),
        "env" if !is_string_map(value) => issue(Fatal::BOTH, "an object of strings"),
        "args" | "disabledTools" | "oauthScopes" if !is_string_array(value) => {
            issue(Fatal::BOTH, "an array of strings")
        }
        "disabled" if value.as_bool().is_none() => issue(Fatal::BOTH, "a boolean"),
        "type" if value.is_null() => issue(Fatal::V3, "a string"),
        "type" if value.as_str().is_none() => issue(Fatal::BOTH, "a string"),
        "timeout" => {
            let number = value.as_number();
            let v2 = !number.is_some_and(|n| n.as_u64().is_some());
            let v3 = !number
                .and_then(serde_json::Number::as_f64)
                .is_some_and(|n| n >= 1.0);
            (v2 || v3).then_some(TypeIssue {
                fatal: Fatal { v2, v3 },
                expected: "a whole number of milliseconds of at least 1",
            })
        }
        "oauth" if value.is_null() => issue(Fatal::V3, "an object"),
        "oauth" if value.as_object().is_none() => issue(Fatal::BOTH, "an object"),
        "cwd" if value.as_str().is_none() => issue(Fatal::V3, "a string"),
        "waitForReady" if value.as_bool().is_none() => issue(Fatal::V3, "a boolean"),
        "autoApprove" if !is_string_array(value) => issue(Fatal::V3, "an array of strings"),
        "versionNegotiation" if !matches!(value.as_str(), Some("auto" | "legacy")) => {
            issue(Fatal::V3, "\"auto\" or \"legacy\"")
        }
        _ => None,
    }
}

/// `oauth.*` field types. V2 accepts `null` for these optional strings; V3
/// keeps only clientId, redirectUri and clientMetadataUrl (others stripped).
fn oauth_field_type_issue(key: &str, value: &JsonValue) -> Option<TypeIssue> {
    let not_string = value.as_str().is_none();
    match key {
        "clientId" | "redirectUri" | "clientMetadataUrl" if not_string => Some(TypeIssue {
            fatal: Fatal {
                v2: !value.is_null(),
                v3: true,
            },
            expected: "a string",
        }),
        "clientSecret" if not_string && !value.is_null() => Some(TypeIssue {
            fatal: Fatal::V2,
            expected: "a string",
        }),
        "oauthScopes" if !value.is_null() && !is_string_array(value) => Some(TypeIssue {
            fatal: Fatal::V2,
            expected: "an array of strings",
        }),
        _ => None,
    }
}

fn report_type_issue(
    cx: &mut Cx<'_>,
    server: &str,
    field: &str,
    node: &JsonNode,
    issue: TypeIssue,
) {
    if !cx.enabled("KR-MCP-007") {
        return;
    }
    let Some((level, hit)) = fatal_level(cx.engines, issue.fatal) else {
        return;
    };
    cx.push(
        level,
        "KR-MCP-007",
        node.start,
        node.end,
        t!(
            "rules.kr_mcp_007.message",
            server = server,
            field = field,
            expected = issue.expected,
            found = describe_value(&node.value).as_str(),
            effect = server_effect(hit).as_str()
        )
        .to_string(),
        t!("rules.kr_mcp_007.suggestion").to_string(),
    );
}

/// Short description of a value for messages.
fn describe_value(value: &JsonValue) -> String {
    match value {
        JsonValue::Number(n) => format!("number {n}"),
        JsonValue::Bool(b) => format!("boolean {b}"),
        JsonValue::String(s) if s.chars().count() <= 40 => format!("string \"{s}\""),
        other => other.type_name().to_string(),
    }
}

/// `command` that carries its arguments ("npx -y pkg"): Kiro starts the
/// command as one executable without a shell (Kiro CLI 2.x on every platform,
/// the V3 engine through cross-spawn on macOS and Linux), so it fails to start.
/// A first word containing a path separator may be a path with spaces.
fn command_has_embedded_args(command: &str) -> bool {
    let trimmed = command.trim();
    let Some(split) = trimmed.find(char::is_whitespace) else {
        return false;
    };
    let head = &trimmed[..split];
    !head.contains(['/', '\\', '"', '\'']) && !trimmed.contains("${")
}

// ---------------------------------------------------------------------------
// URLs
// ---------------------------------------------------------------------------

/// Hosts the V3 engine accepts over plain http (the engine compares the URL
/// hostname with these exact strings).
fn is_loopback(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(host)) => host == "localhost",
        Some(url::Host::Ipv4(ip)) => ip == std::net::Ipv4Addr::LOCALHOST,
        Some(url::Host::Ipv6(ip)) => ip == std::net::Ipv6Addr::LOCALHOST,
        None => false,
    }
}

fn check_remote_url(cx: &mut Cx<'_>, server: &str, member: &JsonMember, text: &str) {
    // `${VAR}` references are expanded before the V3 engine checks the URL.
    if text.contains("${") {
        return;
    }
    let (start, end) = (member.value.start, member.value.end);
    let parsed = url::Url::parse(text.trim());
    let reason = match &parsed {
        Err(_) => Some((
            DiagnosticLevel::Error,
            t!("rules.kr_mcp_004.reasons.not_url").to_string(),
        )),
        Ok(url) if url.scheme() == "https" => None,
        Ok(url) if url.scheme() == "http" => {
            if !is_loopback(url)
                && cx.enabled("KR-MCP-008")
                && let Some((level, _)) = fatal_level(cx.engines, Fatal::V3)
            {
                cx.push(
                    level,
                    "KR-MCP-008",
                    start,
                    end,
                    t!("rules.kr_mcp_008.message", server = server, url = text).to_string(),
                    t!("rules.kr_mcp_008.suggestion").to_string(),
                );
            }
            None
        }
        // Other schemes: the V3 engine rejects them for non-local hosts and
        // both engines connect over HTTP, which ws://, sse:// and similar
        // URLs cannot serve. Kiro documents https, or http on localhost.
        Ok(url) => {
            let level = if is_loopback(url) {
                DiagnosticLevel::Warning
            } else {
                DiagnosticLevel::Error
            };
            Some((
                level,
                t!("rules.kr_mcp_004.reasons.scheme", scheme = url.scheme()).to_string(),
            ))
        }
    };
    if let Some((level, reason)) = reason
        && cx.enabled("KR-MCP-004")
    {
        cx.push(
            level,
            "KR-MCP-004",
            start,
            end,
            t!(
                "rules.kr_mcp_004.message",
                server = server,
                url = text,
                reason = reason.as_str()
            )
            .to_string(),
            t!("rules.kr_mcp_004.suggestion").to_string(),
        );
    }
}

// ---------------------------------------------------------------------------
// Variable references (KR-MCP-009)
// ---------------------------------------------------------------------------

/// VS Code and Cursor variables that look like `${NAME}`. Kiro only replaces
/// `${NAME}` with an environment variable of that name, which is normally
/// unset for these, so the text reaches the server unchanged.
const EDITOR_VARIABLES: &[&str] = &[
    "workspaceFolder",
    "workspaceFolderBasename",
    "workspaceRoot",
    "userHome",
    "pathSeparator",
    "execPath",
    "file",
    "fileDirname",
    "cwd",
];

const SHELLS: &[&str] = &[
    "sh",
    "bash",
    "zsh",
    "dash",
    "ksh",
    "fish",
    "cmd",
    "cmd.exe",
    "powershell",
    "powershell.exe",
    "pwsh",
    "pwsh.exe",
];

/// The first reference in `text` that Kiro does not expand.
fn unsupported_reference(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        let escaped = i > 0 && matches!(bytes[i - 1], b'$' | b'\\');
        match bytes.get(i + 1) {
            Some(b'{') => {
                if let Some(close) = text[i + 2..].find('}') {
                    let inner = &text[i + 2..i + 2 + close];
                    let reference = &text[i..i + 3 + close];
                    let is_name = inner
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                        && inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                    let editor_prefix = ["env:", "input:", "config:", "command:"]
                        .iter()
                        .any(|p| inner.starts_with(p));
                    if !escaped && (editor_prefix || (is_name && EDITOR_VARIABLES.contains(&inner)))
                    {
                        return Some(reference.to_string());
                    }
                    i += 3 + close;
                    continue;
                }
                return None;
            }
            Some(&c) if !escaped && (c.is_ascii_alphabetic() || c == b'_') => {
                let end = text[i + 1..]
                    .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                    .map_or(text.len(), |rel| i + 1 + rel);
                return Some(text[i..end].to_string());
            }
            _ => i += 1,
        }
    }
    None
}

fn check_references(cx: &mut Cx<'_>, server: &str, node: &JsonNode) {
    // Kiro CLI 2.x expansion beyond ${NAME} is undocumented: stay quiet when
    // only V2 is targeted.
    if !cx.engines.includes(KiroEngine::V3) || !cx.enabled("KR-MCP-009") {
        return;
    }
    let command = node
        .value
        .get("command")
        .and_then(|m| m.value.value.as_str());
    let shell_args = command.is_some_and(|c| {
        let base = c.trim().rsplit(['/', '\\']).next().unwrap_or_default();
        SHELLS.contains(&base.to_ascii_lowercase().as_str())
    });
    let mut strings: Vec<(String, &JsonNode)> = Vec::new();
    for field in ["command", "url", "cwd"] {
        if let Some(member) = node.value.get(field)
            && member.value.value.as_str().is_some()
        {
            strings.push((field.to_string(), &member.value));
        }
    }
    if !shell_args
        && let Some(args) = node
            .value
            .get("args")
            .and_then(|m| m.value.value.as_array())
    {
        for (index, arg) in args.iter().enumerate() {
            strings.push((format!("args[{index}]"), arg));
        }
    }
    for field in ["env", "headers"] {
        if let Some(entries) = node
            .value
            .get(field)
            .and_then(|m| m.value.value.as_object())
        {
            for entry in entries {
                strings.push((format!("{field}.{}", entry.key), &entry.value));
            }
        }
    }
    for (field, value_node) in strings {
        let Some(text) = value_node.value.as_str() else {
            continue;
        };
        let Some(reference) = unsupported_reference(text) else {
            continue;
        };
        let editor = reference.starts_with("${");
        let message = if editor {
            t!(
                "rules.kr_mcp_009.editor_message",
                server = server,
                field = field.as_str(),
                reference = reference.as_str()
            )
        } else {
            t!(
                "rules.kr_mcp_009.message",
                server = server,
                field = field.as_str(),
                reference = reference.as_str()
            )
        };
        cx.push(
            DiagnosticLevel::Warning,
            "KR-MCP-009",
            value_node.start,
            value_node.end,
            message.to_string(),
            t!("rules.kr_mcp_009.suggestion").to_string(),
        );
    }
}

// ---------------------------------------------------------------------------
// Secrets (KR-MCP-002)
// ---------------------------------------------------------------------------

/// Split an env or header name into upper-case words: `GITHUB_PAT`,
/// `x-api-key`, `clientSecret` -> [GITHUB, PAT], [X, API, KEY], [CLIENT, SECRET].
fn name_words(name: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut prev_lower = false;
    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            prev_lower = false;
            continue;
        }
        if ch.is_ascii_uppercase() && prev_lower && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        prev_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        current.push(ch.to_ascii_uppercase());
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// Whether an env or header name denotes a credential. A last word such as
/// DIR, PATH, URL or NAME describes something else (`TOKEN_INDEX_DIR`).
fn is_sensitive_name(name: &str) -> bool {
    const NOT_SECRET_LAST: &[&str] = &[
        "DIR",
        "DIRECTORY",
        "PATH",
        "FILE",
        "URL",
        "URI",
        "HOST",
        "PORT",
        "NAME",
        "USER",
        "USERNAME",
        "ID",
        "TYPE",
        "MODE",
        "LEVEL",
        "COUNT",
        "LIMIT",
        "MAX",
        "MIN",
        "TIMEOUT",
        "TTL",
        "EXPIRY",
        "ENDPOINT",
        "REGION",
        "ENABLED",
        "PREFIX",
        "FORMAT",
        "LENGTH",
        "SIZE",
        "VERSION",
        "HEADER",
    ];
    const SECRET_WORDS: &[&str] = &[
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "APIKEY",
        "PAT",
        "CREDENTIAL",
        "CREDENTIALS",
        "AUTHORIZATION",
    ];
    const KEY_QUALIFIERS: &[&str] = &["API", "ACCESS", "PRIVATE", "AUTH", "CLIENT", "SECRET"];
    let words = name_words(name);
    if words
        .last()
        .is_some_and(|last| NOT_SECRET_LAST.contains(&last.as_str()))
    {
        return false;
    }
    words.iter().any(|w| SECRET_WORDS.contains(&w.as_str()))
        || words
            .windows(2)
            .any(|pair| pair[1] == "KEY" && KEY_QUALIFIERS.contains(&pair[0].as_str()))
}

/// Strip an HTTP authentication scheme (`Bearer`, `Basic`, `Token`).
fn strip_auth_scheme(value: &str) -> &str {
    ["Bearer ", "bearer ", "Basic ", "basic ", "Token ", "token "]
        .iter()
        .find_map(|scheme| value.strip_prefix(scheme))
        .unwrap_or(value)
}

/// A credential passed on the command line: `--api-key=VALUE`, `--token VALUE`
/// (the value is the next argument) or a header such as
/// `Authorization: Bearer VALUE`. Returns the flag or header name.
fn secret_in_args(args: &[JsonNode]) -> Vec<(&JsonNode, String)> {
    let mut found = Vec::new();
    let texts: Vec<Option<&str>> = args.iter().map(|a| a.value.as_str()).collect();
    for (index, text) in texts.iter().enumerate() {
        let Some(text) = text else { continue };
        if let Some(flag) = text.strip_prefix("--").or_else(|| text.strip_prefix('-')) {
            if let Some((name, value)) = flag.split_once('=') {
                if is_sensitive_name(name) && seems_plaintext_secret(strip_auth_scheme(value)) {
                    found.push((&args[index], format!("--{name}")));
                }
            } else if is_sensitive_name(flag)
                && let Some(Some(next)) = texts.get(index + 1)
                && !next.starts_with('-')
                && seems_plaintext_secret(strip_auth_scheme(next))
            {
                found.push((&args[index + 1], format!("--{flag}")));
            }
            continue;
        }
        if let Some((name, value)) = text.split_once(':')
            && !name.contains(char::is_whitespace)
            && is_sensitive_name(name)
            && seems_plaintext_secret(strip_auth_scheme(value.trim()))
        {
            found.push((&args[index], name.to_string()));
        }
    }
    found
}

fn check_secrets(cx: &mut Cx<'_>, server: &str, node: &JsonNode) {
    if !cx.enabled("KR-MCP-002") {
        return;
    }
    if let Some(args) = node
        .value
        .get("args")
        .and_then(|m| m.value.value.as_array())
    {
        for (arg, name) in secret_in_args(args) {
            cx.push(
                DiagnosticLevel::Warning,
                "KR-MCP-002",
                arg.start,
                arg.end,
                t!(
                    "rules.kr_mcp_002.message",
                    server = server,
                    field = "args",
                    key = name.as_str()
                )
                .to_string(),
                t!("rules.kr_mcp_002.suggestion").to_string(),
            );
        }
    }
    for field in ["env", "headers"] {
        let Some(entries) = node
            .value
            .get(field)
            .and_then(|m| m.value.value.as_object())
        else {
            continue;
        };
        for entry in entries {
            let Some(value) = entry.value.value.as_str() else {
                continue;
            };
            if !is_sensitive_name(&entry.key) {
                continue;
            }
            // `Authorization: Bearer <token>` carries the secret after the scheme.
            if seems_plaintext_secret(strip_auth_scheme(value)) {
                cx.push(
                    DiagnosticLevel::Warning,
                    "KR-MCP-002",
                    entry.value.start,
                    entry.value.end,
                    t!(
                        "rules.kr_mcp_002.message",
                        server = server,
                        field = field,
                        key = entry.key.as_str()
                    )
                    .to_string(),
                    t!("rules.kr_mcp_002.suggestion").to_string(),
                );
            }
        }
    }
}

/// KR-MCP-015: `autoApprove: ["*"]` approves every tool of the server
/// without prompting (Kiro IDE; Kiro CLI V3 approves MCP tools through
/// permission rules and ignores autoApprove).
fn check_auto_approve(cx: &mut Cx<'_>, server: &str, node: &JsonNode) {
    if !cx.enabled("KR-MCP-015") {
        return;
    }
    let Some(items) = node
        .value
        .get("autoApprove")
        .and_then(|m| m.value.value.as_array())
    else {
        return;
    };
    if let Some(star) = items.iter().find(|i| i.value.as_str() == Some("*")) {
        cx.push(
            DiagnosticLevel::Warning,
            "KR-MCP-015",
            star.start,
            star.end,
            t!("rules.kr_mcp_015.message", server = server).to_string(),
            t!("rules.kr_mcp_015.suggestion").to_string(),
        );
    }
}

// ---------------------------------------------------------------------------
// OAuth (KR-MCP-006)
// ---------------------------------------------------------------------------

fn check_oauth(cx: &mut Cx<'_>, server: &str, oauth: &JsonMember, remote: bool, validated: bool) {
    if !cx.enabled("KR-MCP-006") {
        return;
    }
    // Wrong types are KR-MCP-007.
    let Some(_) = oauth.value.value.as_object() else {
        return;
    };
    let get = |key: &str| oauth.value.value.get(key);
    let report = |cx: &mut Cx<'_>, level, node: &JsonNode, reason: String| {
        cx.push(
            level,
            "KR-MCP-006",
            node.start,
            node.end,
            t!(
                "rules.kr_mcp_006.message",
                server = server,
                reason = reason.as_str()
            )
            .to_string(),
            t!("rules.kr_mcp_006.suggestion").to_string(),
        );
    };

    let client_id = get("clientId").and_then(|m| m.value.value.as_str().map(|s| (m, s)));
    if let Some((member, id)) = client_id
        && id.trim().is_empty()
    {
        report(
            cx,
            DiagnosticLevel::Warning,
            &member.value,
            t!("rules.kr_mcp_006.reasons.client_id_non_empty").to_string(),
        );
    }
    let has_client_id = client_id.is_some_and(|(_, id)| !id.is_empty());
    if let Some(member) = get("clientSecret")
        && let Some(secret) = member.value.value.as_str()
    {
        if secret.trim().is_empty() {
            report(
                cx,
                DiagnosticLevel::Warning,
                &member.value,
                t!("rules.kr_mcp_006.reasons.client_secret_non_empty").to_string(),
            );
        } else if client_id.is_none() {
            report(
                cx,
                DiagnosticLevel::Warning,
                &member.value,
                t!("rules.kr_mcp_006.reasons.client_secret_requires_client_id").to_string(),
            );
        }
    }

    if !remote {
        // OAuth applies to HTTP(S) remote servers only.
        report(
            cx,
            DiagnosticLevel::Warning,
            &oauth.value,
            t!("rules.kr_mcp_006.reasons.http_url").to_string(),
        );
        return;
    }
    if !validated {
        return;
    }

    // The V3 engine validates these two for http servers and skips the server
    // when they are invalid.
    let v3_level = fatal_level(cx.engines, Fatal::V3).map(|(level, _)| level);
    if let Some(member) = get("redirectUri")
        && let Some(uri) = member.value.value.as_str()
        && let Some(problem) = redirect_uri_problem(uri)
        && let Some(level) = v3_level.or(Some(DiagnosticLevel::Warning))
    {
        report(cx, level, &member.value, problem);
    }
    if let Some(member) = get("clientMetadataUrl")
        && let Some(metadata_url) = member.value.value.as_str()
    {
        if has_client_id {
            // "When a clientId is present, Kiro uses it and never consults
            // clientMetadataUrl."
            report(
                cx,
                DiagnosticLevel::Warning,
                &member.value,
                t!("rules.kr_mcp_006.reasons.client_metadata_url_ignored").to_string(),
            );
        } else if !is_valid_client_metadata_url(metadata_url) {
            if let Some(level) = v3_level {
                report(
                    cx,
                    level,
                    &member.value,
                    t!("rules.kr_mcp_006.reasons.client_metadata_url").to_string(),
                );
            }
        } else if get("redirectUri").is_none() {
            // Under CIMD the authorization server validates the callback
            // against the hosted document; Kiro's random loopback port is
            // "rejected on every connect".
            report(
                cx,
                DiagnosticLevel::Warning,
                &member.value,
                t!("rules.kr_mcp_006.reasons.client_metadata_url_redirect").to_string(),
            );
        }
    }
}

/// Mirrors the V3 engine's redirectUri check: `:port`, `host:port`, or a full
/// `http://host:port[/path]` URL without query or fragment, where host is
/// localhost or 127.0.0.1 and port is 1-65535.
fn redirect_uri_problem(value: &str) -> Option<String> {
    let value = value.trim();
    let format = || Some(t!("rules.kr_mcp_006.reasons.redirect_uri_format").to_string());
    if value.contains("://") {
        let Ok(url) = url::Url::parse(value) else {
            return format();
        };
        if url.scheme() != "http" {
            return format();
        }
        if url.query().is_some() || url.fragment().is_some() {
            return Some(t!("rules.kr_mcp_006.reasons.redirect_uri_query").to_string());
        }
        let host_ok = matches!(url.host_str(), Some("localhost" | "127.0.0.1"));
        // The url crate drops a scheme-default port; the V3 engine then sees
        // no port, which it rejects as well.
        let port_ok = url.port().is_some_and(|p| p > 0);
        return (!host_ok || !port_ok).then(|| format().unwrap_or_default());
    }
    let (host, port) = match value.strip_prefix(':') {
        Some(port) => ("127.0.0.1", port),
        None => match value.rsplit_once(':') {
            Some((host, port)) => (host, port),
            None => return format(),
        },
    };
    let host_ok = matches!(host, "localhost" | "127.0.0.1");
    let port_ok = (1..=5).contains(&port.len())
        && port.chars().all(|c| c.is_ascii_digit())
        && port.parse::<u32>().is_ok_and(|p| (1..=65535).contains(&p));
    (!host_ok || !port_ok).then(|| format().unwrap_or_default())
}

fn is_valid_client_metadata_url(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| url.scheme() == "https" && url.path() != "/")
}

// ---------------------------------------------------------------------------
// Agent Plugins mcp.json (KR-MCP-012 to KR-MCP-014)
// ---------------------------------------------------------------------------

fn is_plugin_mcp_schema(value: &str) -> bool {
    plugin_schema_version(value, "mcp.schema.json").is_some()
}

/// `1.x.y` from `https://agent-plugins.org/schemas/1.x.y/<file>`.
fn plugin_schema_version<'s>(value: &'s str, file: &str) -> Option<&'s str> {
    let rest = value.strip_prefix("https://agent-plugins.org/schemas/")?;
    let (version, tail) = rest.split_once('/')?;
    if tail != file {
        return None;
    }
    let mut parts = version.split('.');
    let valid = parts.next() == Some("1")
        && parts.clone().count() == 2
        && parts.all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    valid.then_some(version)
}

fn validate_plugin_mcp(cx: &mut Cx<'_>, manifest_schema: Option<&str>) {
    // The plugin loader uses JSON.parse: no comments, no trailing commas.
    let doc = match parse_json_document(cx.content, false) {
        Ok(doc) => doc,
        Err(error) => {
            if cx.enabled("KR-MCP-001") {
                let offset = offset_of(cx.content, error.line, error.column);
                cx.push(
                    DiagnosticLevel::Error,
                    "KR-MCP-001",
                    offset,
                    offset,
                    t!(
                        "rules.kr_mcp_001_parse.message",
                        error = error.message.as_str()
                    )
                    .to_string(),
                    t!("rules.kr_mcp_001_plugin.suggestion").to_string(),
                );
            }
            return;
        }
    };
    let root = &doc.root;
    let Some(members) = root.value.as_object() else {
        if cx.enabled("KR-MCP-001") {
            cx.push(
                DiagnosticLevel::Error,
                "KR-MCP-001",
                root.start,
                root.end,
                t!(
                    "rules.kr_mcp_001_root_type.message",
                    found = root.value.type_name()
                )
                .to_string(),
                t!("rules.kr_mcp_001_plugin.suggestion").to_string(),
            );
        }
        return;
    };

    // KR-MCP-012: $schema, checked first by the loader.
    let schema = root.value.get("$schema");
    let schema_text = schema.and_then(|m| m.value.value.as_str());
    let mcp_version = schema_text.and_then(|s| plugin_schema_version(s, "mcp.schema.json"));
    if mcp_version.is_none() {
        if cx.enabled("KR-MCP-012") {
            let (start, end) =
                schema.map_or((root.start, root.start), |m| (m.value.start, m.value.end));
            cx.push(
                DiagnosticLevel::Error,
                "KR-MCP-012",
                start,
                end,
                t!("rules.kr_mcp_012.schema_message").to_string(),
                t!("rules.kr_mcp_012.suggestion").to_string(),
            );
        }
        return;
    }
    // Compared only against a valid plugin.json schema: an invalid one makes
    // Kiro reject the whole plugin before it reads mcp.json.
    if let Some(manifest_version) =
        manifest_schema.and_then(|s| plugin_schema_version(s, "plugin.schema.json"))
        && Some(manifest_version) != mcp_version
        && let Some(schema) = schema
    {
        if cx.enabled("KR-MCP-012") {
            cx.push(
                DiagnosticLevel::Error,
                "KR-MCP-012",
                schema.value.start,
                schema.value.end,
                t!(
                    "rules.kr_mcp_012.version_message",
                    mcp = mcp_version.unwrap_or_default(),
                    manifest = manifest_version
                )
                .to_string(),
                t!("rules.kr_mcp_012.suggestion").to_string(),
            );
        }
        return;
    }

    // KR-MCP-013: any other top-level key disables every server.
    let unknown: Vec<&JsonMember> = members
        .iter()
        .filter(|m| m.key != "$schema" && m.key != "mcpServers")
        .collect();
    if !unknown.is_empty() {
        if cx.enabled("KR-MCP-013") {
            for member in unknown {
                cx.push(
                    DiagnosticLevel::Error,
                    "KR-MCP-013",
                    member.key_start,
                    member.key_end,
                    t!("rules.kr_mcp_013.message", field = member.key.as_str()).to_string(),
                    t!("rules.kr_mcp_013.suggestion").to_string(),
                );
            }
        }
        return;
    }

    let Some(servers) = root.value.get("mcpServers") else {
        if cx.enabled("KR-MCP-001") {
            cx.push(
                DiagnosticLevel::Error,
                "KR-MCP-001",
                root.start,
                root.start,
                t!("rules.kr_mcp_001_root.message").to_string(),
                t!("rules.kr_mcp_001_plugin.suggestion").to_string(),
            );
        }
        return;
    };
    let Some(entries) = servers.value.value.as_object() else {
        if cx.enabled("KR-MCP-001") {
            cx.push(
                DiagnosticLevel::Error,
                "KR-MCP-001",
                servers.value.start,
                servers.value.end,
                t!(
                    "rules.kr_mcp_001_root_type.message",
                    found = servers.value.value.type_name()
                )
                .to_string(),
                t!("rules.kr_mcp_001_plugin.suggestion").to_string(),
            );
        }
        return;
    };
    for (index, entry) in entries.iter().enumerate() {
        if let Some(later) = entries[index + 1..].iter().find(|m| m.key == entry.key) {
            if cx.enabled("KR-MCP-005") {
                let (later_line, _) = cx.lines.line_col(later.key_start);
                cx.push(
                    DiagnosticLevel::Warning,
                    "KR-MCP-005",
                    entry.key_start,
                    entry.key_end,
                    t!(
                        "rules.kr_mcp_005.message",
                        server = entry.key.as_str(),
                        line = later_line
                    )
                    .to_string(),
                    t!("rules.kr_mcp_005.suggestion").to_string(),
                );
            }
            continue;
        }
        check_secrets(cx, &entry.key, &entry.value);
        if let Some((node, reason)) = plugin_server_problem(entry)
            && cx.enabled("KR-MCP-014")
        {
            cx.push(
                DiagnosticLevel::Error,
                "KR-MCP-014",
                node.start,
                node.end,
                t!(
                    "rules.kr_mcp_014.message",
                    server = entry.key.as_str(),
                    reason = reason.as_str()
                )
                .to_string(),
                t!("rules.kr_mcp_014.suggestion").to_string(),
            );
        }
    }
}

fn plugin_reason(key: &str) -> String {
    match key {
        "not_object" => t!("rules.kr_mcp_014.reasons.not_object"),
        "type_missing" => t!("rules.kr_mcp_014.reasons.type_missing"),
        "command_missing" => t!("rules.kr_mcp_014.reasons.command_missing"),
        "command_escapes" => t!("rules.kr_mcp_014.reasons.command_escapes"),
        "command_path" => t!("rules.kr_mcp_014.reasons.command_path"),
        "args" => t!("rules.kr_mcp_014.reasons.args"),
        "env" => t!("rules.kr_mcp_014.reasons.env"),
        "env_reserved" => t!("rules.kr_mcp_014.reasons.env_reserved"),
        "cwd" => t!("rules.kr_mcp_014.reasons.cwd"),
        "url_missing" => t!("rules.kr_mcp_014.reasons.url_missing"),
        "url_invalid" => t!("rules.kr_mcp_014.reasons.url_invalid"),
        "url_userinfo" => t!("rules.kr_mcp_014.reasons.url_userinfo"),
        "url_fragment" => t!("rules.kr_mcp_014.reasons.url_fragment"),
        "url_insecure" => t!("rules.kr_mcp_014.reasons.url_insecure"),
        _ => t!("rules.kr_mcp_014.reasons.headers"),
    }
    .to_string()
}

/// Mirrors the Agent Plugins server loader: the first reason an entry is
/// skipped, with the node to point at.
fn plugin_server_problem(entry: &JsonMember) -> Option<(&JsonNode, String)> {
    let node = &entry.value;
    let reason = |key: &str| plugin_reason(key);
    let Some(fields) = node.value.as_object() else {
        return Some((node, reason("not_object")));
    };
    let get = |key: &str| node.value.get(key);
    let Some(kind) = get("type").and_then(|m| m.value.value.as_str()) else {
        return Some((
            get("type").map_or(node, |m| &m.value),
            reason("type_missing"),
        ));
    };
    let allowed: &[&str] = match kind {
        "stdio" => &["type", "command", "args", "env", "cwd"],
        "streamable-http" | "sse" => &["type", "url", "headers"],
        _ => {
            let type_node = get("type").map_or(node, |m| &m.value);
            return Some((
                type_node,
                t!("rules.kr_mcp_014.reasons.type_unknown", value = kind).to_string(),
            ));
        }
    };
    if let Some(field) = fields.iter().find(|f| !allowed.contains(&f.key.as_str())) {
        return Some((
            &field.value,
            t!(
                "rules.kr_mcp_014.reasons.unknown_field",
                field = field.key.as_str(),
                server_type = kind
            )
            .to_string(),
        ));
    }
    if kind == "stdio" {
        let Some(command_member) = get("command") else {
            return Some((node, reason("command_missing")));
        };
        let Some(command) = command_member
            .value
            .value
            .as_str()
            .filter(|c| !c.is_empty())
        else {
            return Some((&command_member.value, reason("command_missing")));
        };
        if let Some(relative) = command.strip_prefix("./") {
            if relative.split(['/', '\\']).any(|part| part == "..") {
                return Some((&command_member.value, reason("command_escapes")));
            }
        } else if command.contains(['/', '\\']) || command.starts_with("..") {
            return Some((&command_member.value, reason("command_path")));
        }
        if let Some(args) = get("args")
            && !is_string_array(&args.value.value)
        {
            return Some((&args.value, reason("args")));
        }
        if let Some(env) = get("env") {
            let Some(vars) = env.value.value.as_object() else {
                return Some((&env.value, reason("env")));
            };
            if vars
                .iter()
                .any(|v| v.key == "PLUGIN_ROOT" || v.key == "PLUGIN_DATA")
            {
                return Some((&env.value, reason("env_reserved")));
            }
            if let Some(var) = vars.iter().find(|v| v.value.value.as_str().is_none()) {
                return Some((&var.value, reason("env")));
            }
        }
        if let Some(cwd_member) = get("cwd") {
            let Some(cwd) = cwd_member.value.value.as_str() else {
                return Some((&cwd_member.value, reason("cwd")));
            };
            let form_ok = cwd.starts_with("./")
                || ["${PLUGIN_ROOT}", "${PLUGIN_DATA}"]
                    .iter()
                    .any(|base| cwd == *base || cwd.starts_with(&format!("{base}/")));
            if cwd.contains("..") || !form_ok {
                return Some((&cwd_member.value, reason("cwd")));
            }
        }
        return None;
    }
    let Some(url_member) = get("url") else {
        return Some((node, reason("url_missing")));
    };
    let Some(url_text) = url_member.value.value.as_str().filter(|u| !u.is_empty()) else {
        return Some((&url_member.value, reason("url_missing")));
    };
    let url_problem = match url::Url::parse(url_text) {
        Err(_) => Some("url_invalid"),
        Ok(url) if !matches!(url.scheme(), "http" | "https") => Some("url_invalid"),
        Ok(url) if !url.username().is_empty() || url.password().is_some() => Some("url_userinfo"),
        Ok(url) if url.fragment().is_some() => Some("url_fragment"),
        Ok(url) if url.scheme() == "http" && !is_loopback(&url) => Some("url_insecure"),
        Ok(_) => None,
    };
    if let Some(key) = url_problem {
        return Some((&url_member.value, reason(key)));
    }
    if let Some(headers) = get("headers") {
        let Some(entries) = headers.value.value.as_object() else {
            return Some((&headers.value, reason("headers")));
        };
        let mut seen = std::collections::HashSet::new();
        for header in entries {
            if header.value.value.as_str().is_none()
                || !seen.insert(header.key.to_ascii_lowercase())
            {
                return Some((&header.value, reason("headers")));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests;
